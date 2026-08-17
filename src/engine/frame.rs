use super::{Engine, TargetId};
use crate::{
    context::{EventCtx, LayoutCtx, PaintCtx, PrepareCtx, SweepCtx},
    event::{Event, KeyState, ScrollDelta, ToEvent},
    model::{Position, Size},
    primitive::Instance,
    render::renderer::Presented,
    task::{Task, UploadCtx},
    tree,
    widget::Element,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOutcome {
    /// A frame was encoded and submitted.
    Rendered,
    /// Rendering was skipped (need was false, target missing, or Timeout).
    Skipped,
    /// The surface was Lost or Outdated; `surface.configure()` has been called
    /// and the caller should try rendering again.
    NeedsRerender,
    /// A frame was rendered, but the surface wants reconfiguring for the next one.
    RenderedSuboptimal,
}

impl<'a> Engine<'a> {
    fn spawn_task<M: 'static>(&mut self, tid: &TargetId, task: Task<M>, redraw: &mut bool) {
        match task {
            Task::None => (),
            Task::Redraw => *redraw = true,
            Task::Batch(tasks) => {
                for t in tasks {
                    self.spawn_task(tid, t, redraw);
                }
            }
            Task::Work { run, finish } => {
                let Some(target) = self.targets.get_mut(tid) else {
                    return;
                };
                let id = target.ctx.tasks.alloc_id();
                target
                    .ctx
                    .tasks
                    .finishers
                    .insert(id, crate::task::erase(finish));
                self.runner.spawn(*tid, id, run);
            }
        }
    }

    /// Spawn a [`Task`] against a target from outside `poll`. This is the
    /// imperative escape hatch (e.g. kick off a load at startup).
    pub fn spawn<M: 'static>(&mut self, tid: &TargetId, task: Task<M>) {
        let mut redraw = false;
        self.spawn_task(tid, task, &mut redraw);
        if redraw && let Some(t) = self.targets.get_mut(tid) {
            t.ctx.request_redraw();
        }
    }

    pub fn poll<S, P, M: 'static, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        tid: &TargetId,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> Task<M>,
        state: &mut S,
        params: &P,
    ) -> bool {
        crate::scope!("Engine::poll");

        let target = if let Some(t) = self.targets.get_mut(tid) {
            t
        } else {
            return false;
        };

        let now = std::time::Instant::now();
        let total = now.duration_since(target.start_time);
        let dt = now.duration_since(target.last_frame_time);
        target.last_frame_time = now;
        target.globals.time = total.as_secs_f32();
        target.globals.delta_time = dt.as_secs_f32();

        crate::plot!("ui.dt_ms", (target.globals.delta_time as f64) * 1000.0);

        self.text.set_scale_factor(target.scale_factor as f32);

        let mut require_redraw = false;

        if let Some(root) = target.root.as_mut() {
            let mut event_cx = EventCtx::new(
                &target.globals,
                &mut *self.text,
                &mut target.ctx,
                None,
                &self.layout_engine,
                &mut *self.message_sink,
            );
            let mut cursor = 0usize;
            tree::handle_tree(root.as_mut(), &mut event_cx, &mut cursor);
            target.ctx.mouse_buttons_pressed = 0;
            target.ctx.mouse_buttons_released = 0;
        } else {
            require_redraw = true;
        }

        require_redraw |= target.ctx.take_redraw();

        self.landings.clear();
        self.runner.drain(&mut self.landings);
        for (tid, id, payload) in self.landings.drain(..) {
            if let Some(t) = self.targets.get_mut(&tid) {
                t.ctx.tasks.inbox.push_back((id, payload));
            }
        }

        // TODO: bound the wall-clock time spent here. Take a start
        // Instant and, after each finisher, break once a budget is exceeded,
        // leaving the remaining (id, payload) entries in `inbox` for next frame.
        // Uploading many large textures in one poll otherwise spikes frame time.
        loop {
            let Some((finish, payload)) = self.targets.get_mut(tid).and_then(|t| {
                let (id, payload) = t.ctx.tasks.inbox.pop_front()?;
                t.ctx.tasks.finishers.remove(&id).map(|f| (f, payload))
            }) else {
                break;
            };

            let msg_any = {
                let mut up = UploadCtx {
                    gpu: &self.gpu,
                    textures: &mut self.renderer.textures,
                };
                finish(payload, &mut up)
            };
            if let Ok(msg) = msg_any.downcast::<M>() {
                let task = update(self, &Event::Message(*msg), state, params);
                self.spawn_task(tid, task, &mut require_redraw);
            }
        }

        for message in self.message_sink.drain() {
            let msg = message.downcast::<M>().unwrap();
            let task = update(self, &Event::Message(*msg), state, params);
            self.spawn_task(tid, task, &mut require_redraw);
        }

        let task = update(self, &Event::RedrawRequested, state, params);
        self.spawn_task(tid, task, &mut require_redraw);

        require_redraw
    }
    pub fn render_if_needed<S>(
        &mut self,
        tid: &TargetId,
        need: bool,
        view: &impl Fn(&TargetId, &S) -> Element,
        state: &mut S,
    ) -> crate::Result<RenderOutcome> {
        if !need {
            return Ok(RenderOutcome::Skipped);
        }

        let Some(target) = self.targets.get_mut(tid) else {
            return Ok(RenderOutcome::Skipped);
        };

        crate::scope!("Engine::render");

        self.text.set_scale_factor(target.scale_factor as f32);

        target.root = Some(view(tid, state));
        let root = target.root.as_mut().unwrap();

        let max = Size::new(
            target.globals.window_size[0] as i32,
            target.globals.window_size[1] as i32,
        )
        .max(Size::new(1, 1));

        let root_id = {
            crate::scope!("layout");
            let mut layout_ctx = LayoutCtx::new(
                &target.globals,
                &mut target.ctx.view_state,
                &mut *self.text,
                &self.theme,
            );
            tree::run_layout(
                &mut self.layout_engine,
                &mut layout_ctx,
                root.as_mut(),
                max.width,
                max.height,
            )
        };

        self.text.tick();

        {
            crate::scope!("prepare");
            let mut prepare_ctx = PrepareCtx::new(
                &target.globals,
                &mut *self.text,
                &self.gpu,
                &mut self.renderer.textures,
                &mut self.pipeline_registry,
                target.config.format,
                self.immediate_size,
                &self.layout_engine,
                &mut target.ctx.view_state,
                &self.theme,
            );
            let mut cursor = root_id;
            tree::prepare_tree(root.as_mut(), &mut prepare_ctx, &mut cursor);
        }

        {
            crate::scope!("paint");
            let mut paint_ctx = PaintCtx::new(
                &target.globals,
                &*self.text,
                &self.layout_engine,
                &mut target.ctx.view_state,
                &self.theme,
                &target.ctx.focus,
            );

            let mut cursor = root_id;
            let screen_clip = Some([
                0,
                0,
                target.globals.window_size[0] as i32,
                target.globals.window_size[1] as i32,
            ]);
            self.instance_buf.clear();
            self.instance_buf.push(Instance::ui(
                Position::default(),
                Size::from(target.globals.window_size),
                self.theme.surface,
            ));

            tree::paint_tree(
                root.as_mut(),
                &mut paint_ctx,
                &self.layout_engine,
                &mut cursor,
                &mut self.instance_buf,
                screen_clip,
            );
        }

        {
            crate::scope!("view_state::sweep");
            target.ctx.sweep_focus();
            let mut sweep_ctx = SweepCtx {
                gpu: &self.gpu,
                texture: &mut self.renderer.textures,
            };
            target.ctx.view_state.sweep(&mut sweep_ctx);
        }

        crate::plot!("ui.instances", self.instance_buf.len() as f64);
        crate::plot!("ui.nodes", self.layout_engine.node_count as f64);

        target.globals.frame = target.globals.frame.wrapping_add(1);

        match self.renderer.render(
            &self.gpu,
            &target.surface,
            &mut target.attachments,
            &mut self.pipeline_registry,
            &target.globals,
            &self.instance_buf,
        ) {
            Presented::Ok => Ok(RenderOutcome::Rendered),
            Presented::Suboptimal => {
                target.surface.configure(&self.gpu.device, &target.config);
                Ok(RenderOutcome::RenderedSuboptimal)
            }
            Presented::SurfaceLost => {
                target.surface.configure(&self.gpu.device, &target.config);
                Ok(RenderOutcome::NeedsRerender)
            }
        }
    }
    pub fn handle_platform_event<S, P, M: 'static, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        target_id: &TargetId,
        event: &E,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> Task<M>,
        state: &mut S,
        params: &P,
    ) {
        let target = match self.targets.get_mut(target_id) {
            Some(t) => t,
            None => {
                return; // TODO: maybe return a result instead
            }
        };

        let event = event.to_event();
        target.ctx.mouse_buttons_pressed = 0;
        target.ctx.mouse_buttons_released = 0;

        self.text.set_scale_factor(target.scale_factor as f32);

        match event {
            Event::Resized { size } => {
                if size.width > 0 && size.height > 0 {
                    let sf = target.scale_factor;
                    let lw = (size.width as f64 / sf).round() as u32;
                    let lh = (size.height as f64 / sf).round() as u32;
                    target.size = Size::new(lw.max(1), lh.max(1));
                    target.globals.window_size = [lw as f32, lh as f32];
                    target.config.width = size.width;
                    target.config.height = size.height;
                    target.surface.configure(&self.gpu.device, &target.config);
                }
                target.ctx.request_redraw();
            }
            Event::ScaleFactorChanged { factor } => {
                target.scale_factor = factor;
                target.globals.scale = factor as f32;

                let pw = (target.size.width as f64 * factor).round() as u32;
                let ph = (target.size.height as f64 * factor).round() as u32;
                if pw > 0 && ph > 0 {
                    target.config.width = pw;
                    target.config.height = ph;
                    target.surface.configure(&self.gpu.device, &target.config);
                }
                target.ctx.request_redraw();
            }
            Event::CursorLeft => {
                target.ctx.mouse_pos = crate::context::POINTER_ELSEWHERE;
                target.globals.mouse_pos = [
                    crate::context::POINTER_ELSEWHERE.x,
                    crate::context::POINTER_ELSEWHERE.y,
                ];
                target.ctx.request_redraw();
            }
            Event::CursorMoved { position } => {
                let sf = target.scale_factor as f32;
                let lp = Position::new(position.x / sf, position.y / sf);
                target.ctx.mouse_pos = lp;
                target.globals.mouse_pos = [lp.x, lp.y];
            }
            Event::MouseInput { button, state } => {
                let bit = 1u32 << button.bit();
                match state {
                    KeyState::Pressed => {
                        target.ctx.mouse_buttons_down |= bit;
                        target.ctx.mouse_buttons_pressed |= bit;
                        target.globals.mouse_buttons |= bit;
                    }
                    KeyState::Released => {
                        target.ctx.mouse_buttons_down &= !bit;
                        target.ctx.mouse_buttons_released |= bit;
                        target.globals.mouse_buttons &= !bit;
                    }
                }
            }
            Event::ModifiersChanged(m) => {
                target.ctx.modifiers = m;
            }
            Event::Focused(f) => {
                target.ctx.surface_focused = f;
                target.ctx.request_redraw();
            }
            _ => (),
        }

        if let Some(root) = target.root.as_mut() {
            use crate::event::UiEventRef as Ui;
            let logical_size = target.size;
            let logical_mouse = target.ctx.mouse_pos;
            let ev_view = match &event {
                Event::RedrawRequested => Some(Ui::RedrawRequested),
                Event::Resized { .. } => Some(Ui::Resized { size: logical_size }),
                Event::ScaleFactorChanged { factor } => {
                    Some(Ui::ScaleFactorChanged { factor: *factor })
                }
                Event::CursorEntered => Some(Ui::CursorEntered),
                Event::CursorLeft => Some(Ui::CursorLeft),
                Event::CursorMoved { .. } => Some(Ui::CursorMoved {
                    position: logical_mouse,
                }),
                Event::MouseInput { button, state } => Some(Ui::MouseButton {
                    button: *button,
                    state: *state,
                }),
                Event::MouseWheel(d) => {
                    let logical_delta = if d.units == crate::event::ScrollUnits::Pixels {
                        let sf = target.scale_factor as f32;
                        ScrollDelta {
                            dx: d.dx / sf,
                            dy: d.dy / sf,
                            units: d.units,
                        }
                    } else {
                        *d
                    };
                    Some(Ui::MouseWheel(logical_delta))
                }
                Event::Key(k) => Some(Ui::Key(k)),
                Event::Text(t) => Some(Ui::Text(t)),
                Event::ModifiersChanged(m) => Some(Ui::ModifiersChanged(m)),
                Event::Focused(f) => Some(Ui::Focused(*f)),
                _ => None,
            };

            if ev_view.is_some() {
                let mut ctx = EventCtx::new(
                    &target.globals,
                    &mut *self.text,
                    &mut target.ctx,
                    ev_view,
                    &self.layout_engine,
                    &mut *self.message_sink,
                );
                let mut cursor = 0usize;
                tree::handle_tree(root.as_mut(), &mut ctx, &mut cursor);
            }
        }

        if self.targets.contains_key(target_id) {
            let task = update(self, &event, state, params);
            let mut redraw = false;
            self.spawn_task(target_id, task, &mut redraw);
            if redraw && let Some(target) = self.targets.get_mut(target_id) {
                target.ctx.request_redraw();
            }
        }
    }
}
