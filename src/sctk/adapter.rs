use super::{erased::SctkErased, handler::SctkHandler, msg::Emit};
use smithay_client_toolkit::session_lock::{
    SessionLock, SessionLockSurface, SessionLockSurfaceConfigure,
};
use std::fmt::Debug;
use std::marker::PhantomData;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, QueueHandle};

pub struct SctkAdapter<H, M, F>
where
    H: SctkHandler<M>,
    F: FnMut(M),
{
    sink: F,
    _pdh: PhantomData<H>,
    _pdm: PhantomData<M>,
}

impl<H, M, F> SctkAdapter<H, M, F>
where
    H: SctkHandler<M>,
    F: FnMut(M),
{
    pub fn new(sink: F) -> Self {
        Self {
            sink,
            _pdh: PhantomData,
            _pdm: PhantomData,
        }
    }

    #[inline]
    fn flush(&mut self, out: Emit<M>) {
        for m in out {
            (self.sink)(m)
        }
    }
}

impl<H, M, F> SctkErased for SctkAdapter<H, M, F>
where
    H: SctkHandler<M>,
    F: FnMut(M),
{
    fn runtime_add_global(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        name: u32,
        iface: &str,
        ver: u32,
    ) {
        self.flush(H::runtime_add_global(c, q, name, iface, ver));
    }
    fn runtime_remove_global(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        name: u32,
        iface: &str,
    ) {
        self.flush(H::runtime_remove_global(c, q, name, iface));
    }

    fn new_output(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        o: WlOutput,
    ) {
        self.flush(H::new_output(c, q, o));
    }
    fn update_output(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        o: WlOutput,
    ) {
        self.flush(H::update_output(c, q, o));
    }
    fn output_destroyed(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        o: WlOutput,
    ) {
        self.flush(H::output_destroyed(c, q, o));
    }

    fn locked(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        sl: SessionLock,
    ) {
        self.flush(H::locked(c, q, sl));
    }

    fn finished(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        sl: SessionLock,
    ) {
        self.flush(H::finished(c, q, sl));
    }

    fn configure(
        &mut self,
        c: &Connection,
        q: &QueueHandle<super::state::SctkState>,
        s: SessionLockSurface,
        conf: SessionLockSurfaceConfigure,
        serial: u32,
    ) {
        self.flush(H::configure(c, q, s, conf, serial));
    }
}

pub fn erase<H, M, F>(sink: F) -> Box<dyn SctkErased>
where
    H: SctkHandler<M> + 'static,
    M: 'static + Debug,
    F: FnMut(M) + 'static,
{
    Box::new(SctkAdapter::<H, M, _>::new(sink))
}
