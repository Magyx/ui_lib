use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
};

use crate::{
    engine::TargetId,
    gpu::Gpu,
    render::texture::{TextureHandle, TextureRegistry},
};

pub type TaskId = u64;

pub type Payload = Box<dyn Any + Send + 'static>;

pub type BoxWork = Pin<Box<dyn Future<Output = Payload> + Send + 'static>>;

pub type Finish<M> = Box<dyn for<'e> FnOnce(Payload, &mut UploadCtx<'e>) -> M + 'static>;

pub type ErasedFinish =
    Box<dyn for<'e> FnOnce(Payload, &mut UploadCtx<'e>) -> Box<dyn Any> + 'static>;

pub struct UploadCtx<'e> {
    pub gpu: &'e Gpu,
    pub textures: &'e mut TextureRegistry,
}

impl UploadCtx<'_> {
    /// Upload an RGBA8 image and return a handle to it. Mirrors
    /// [`Engine::load_texture_rgba8`](crate::graphics::Engine::load_texture_rgba8).
    pub fn load_rgba8(&mut self, width: u32, height: u32, pixels_rgba8: &[u8]) -> TextureHandle {
        self.textures
            .load_rgba8(self.gpu, width, height, pixels_rgba8)
    }

    /// Overwrite an existing texture's contents in place.
    pub fn update_rgba8(&mut self, handle: TextureHandle, pixels_rgba8: &[u8]) -> bool {
        self.textures.update_rgba8(self.gpu, handle, pixels_rgba8)
    }
}

/// Decoded, ready-to-upload image data returned by a [`Task::load_image`]
/// loader. Decoding is the app's job (the library takes no image dependency);
/// the loader hands back tightly-packed RGBA8, `width * height * 4` bytes.
pub struct RawImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A unit of work returned from `update`.
///
/// See the [module docs](self). Construct via [`Task::none`], [`Task::redraw`],
/// [`Task::batch`], [`Task::perform`], or a specific task like
/// [`Task::load_image`], then adapt the message with [`Task::map`].
pub enum Task<M> {
    /// Do nothing.
    None,
    /// Request the current target repaint this frame.
    Redraw,
    /// Run several tasks.
    Batch(Vec<Task<M>>),
    /// Background work plus its main-thread finish. Not usually built by hand.
    Work { run: BoxWork, finish: Finish<M> },
}

impl<M: 'static> Task<M> {
    /// A task that does nothing.
    pub fn none() -> Self {
        Task::None
    }

    /// Request a repaint of the current target.
    pub fn redraw() -> Self {
        Task::Redraw
    }

    /// Run several tasks.
    pub fn batch(tasks: impl IntoIterator<Item = Task<M>>) -> Self {
        Task::Batch(tasks.into_iter().collect())
    }

    /// Run an async future off-thread; when it resolves, `then` runs on the
    /// main thread to produce the message. `then` may be `!Send` and may return
    /// a `!Send` `M`; only `work` and its output `T` cross the thread boundary.
    pub fn perform<T, Fut, F>(work: Fut, then: F) -> Self
    where
        T: Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        F: FnOnce(T) -> M + 'static,
    {
        Task::Work {
            run: Box::pin(async move { Box::new(work.await) as Payload }),
            finish: Box::new(move |p, _up| then(*downcast::<T>(p))),
        }
    }

    /// Run a blocking closure off-thread (decode, IO, hashing …); when it
    /// returns, `then` runs on the main thread to produce the message.
    pub fn blocking<T, W, F>(work: W, then: F) -> Self
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
        F: FnOnce(T) -> M + 'static,
    {
        Task::Work {
            run: Box::pin(async move { Box::new(work()) as Payload }),
            finish: Box::new(move |p, _up| then(*downcast::<T>(p))),
        }
    }

    /// Decode/produce image bytes off-thread, then upload on the main thread and
    /// resolve to the resulting [`TextureHandle`].
    ///
    /// ```ignore
    /// Task::load_image(move || RawImage { width, height, rgba })
    ///     .map(Message::ImageLoaded)
    /// ```
    pub fn load_image<W, F>(loader: W, then: F) -> Self
    where
        W: FnOnce() -> RawImage + Send + 'static,
        F: FnOnce(TextureHandle) -> M + 'static,
    {
        Task::Work {
            run: Box::pin(async move { Box::new(loader()) as Payload }),
            finish: Box::new(|p, up| {
                let img = *downcast::<RawImage>(p);
                let handle = up.load_rgba8(img.width, img.height, &img.rgba);
                then(handle)
            }),
        }
    }

    /// Transform the message a task will eventually produce. Composes onto the
    /// finish closure only; the async half is untouched. `f` is `Fn + Clone` so
    /// it can fan out across a [`Batch`](Task::Batch) — a plain `fn` (e.g. a
    /// message variant constructor) satisfies this for free.
    pub fn map<N: 'static>(self, f: impl Fn(M) -> N + Clone + 'static) -> Task<N> {
        match self {
            Task::None => Task::None,
            Task::Redraw => Task::Redraw,
            Task::Batch(v) => Task::Batch(v.into_iter().map(|t| t.map(f.clone())).collect()),
            Task::Work { run, finish } => Task::Work {
                run,
                finish: Box::new(move |p, up| f(finish(p, up))),
            },
        }
    }
}

fn downcast<T: 'static>(p: Payload) -> Box<T> {
    p.downcast::<T>()
        .unwrap_or_else(|_| panic!("task payload type mismatch (library bug)"))
}

pub(crate) fn erase<M: 'static>(finish: Finish<M>) -> ErasedFinish {
    Box::new(move |p, up| Box::new(finish(p, up)) as Box<dyn Any>)
}

/// Drives the async half of tasks and delivers their payloads back to the main
/// thread, tagged with the target and task they belong to.
///
/// The runner owns both *execution* (thread pool, calloop, tokio …) and the
/// *wake path*: on a frame-polled loop (winit) delivery need only be drainable
/// next frame; on a blocking loop (calloop) delivery must also wake the loop.
/// Swap the default via
/// [`EngineBuilder::with_task_runner`](crate::builder::EngineBuilder::with_task_runner).
pub trait TaskRunner {
    /// Begin driving `run`. On completion, arrange for `(target, id, payload)`
    /// to become drainable via [`drain`](TaskRunner::drain) on the main thread.
    fn spawn(&self, target: TargetId, id: TaskId, run: BoxWork);

    /// Move every payload completed since the last call into `out`. Main-thread
    /// only; called once near the top of each [`poll`](crate::graphics::Engine::poll).
    fn drain(&self, out: &mut Vec<(TargetId, TaskId, Payload)>);
}

// TODO: thread-per-task is fine for a handful of image loads but wasteful
// under load; replace with a small fixed pool + work queue.
pub struct ThreadRunner {
    tx: std::sync::mpsc::Sender<(TargetId, TaskId, Payload)>,
    rx: std::sync::mpsc::Receiver<(TargetId, TaskId, Payload)>,
}

impl Default for ThreadRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadRunner {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { tx, rx }
    }
}

impl TaskRunner for ThreadRunner {
    fn spawn(&self, target: TargetId, id: TaskId, run: BoxWork) {
        let tx = self.tx.clone();
        std::thread::Builder::new()
            .name("ui-task".into())
            .spawn(move || {
                let payload = pollster::block_on(run);
                // Receiver gone => target detached; drop the payload.
                let _ = tx.send((target, id, payload));
            })
            .expect("spawn ui-task thread");
    }

    fn drain(&self, out: &mut Vec<(TargetId, TaskId, Payload)>) {
        while let Ok(item) = self.rx.try_recv() {
            out.push(item);
        }
    }
}

#[derive(Default)]
pub(crate) struct TaskStore {
    pub(crate) inbox: VecDeque<(TaskId, Payload)>,
    pub(crate) finishers: HashMap<TaskId, ErasedFinish>,
    next_id: TaskId,
}

impl TaskStore {
    pub(crate) fn alloc_id(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}
