use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use std::{fmt::Debug, marker::PhantomData};

use super::handler::{Emit, SctkHandler};

#[allow(clippy::too_many_arguments)]
pub trait SctkErased {
    // ProvidesRegistryState
    fn runtime_add_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
        version: u32,
    );
    fn runtime_remove_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
    );

    // OutputHandler
    fn new_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    );
    fn update_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    );
    fn output_destroyed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    );

    // CompositorHandler
    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        time: u32,
    );
    fn surface_enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    );
    fn surface_leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    );
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_factor: i32,
    );
    fn transform_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_transform: smithay_client_toolkit::reexports::client::protocol::wl_output::Transform,
    );

    // LayerShellHandler
    fn closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    );

    // WindowHandler
    fn request_close(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
    );

    // SessionLockHandler
    fn locked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    );
    fn finished(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    );
}

pub(crate) struct SctkAdapter<H, M, F>
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
        match out {
            Emit::None => (),
            Emit::One(m) => (self.sink)(m),
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
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        self.flush(H::runtime_add_global(conn, qh, name, interface, version));
    }
    fn runtime_remove_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
    ) {
        self.flush(H::runtime_remove_global(conn, qh, name, interface));
    }

    fn new_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.flush(H::new_output(conn, qh, output));
    }
    fn update_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.flush(H::update_output(conn, qh, output));
    }
    fn output_destroyed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.flush(H::output_destroyed(conn, qh, output));
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        time: u32,
    ) {
        self.flush(H::frame(conn, qh, surface, time));
    }
    fn surface_enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.flush(H::surface_enter(conn, qh, surface, output));
    }
    fn surface_leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.flush(H::surface_leave(conn, qh, surface, output));
    }
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.flush(H::scale_factor_changed(conn, qh, surface, new_factor));
    }
    fn transform_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_transform: smithay_client_toolkit::reexports::client::protocol::wl_output::Transform,
    ) {
        self.flush(H::transform_changed(conn, qh, surface, new_transform));
    }

    fn closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    ) {
        self.flush(H::closed(conn, qh, layer));
    }

    fn request_close(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        self.flush(H::request_close(conn, qh, window));
    }

    fn locked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) {
        self.flush(H::locked(conn, qh, session_lock));
    }
    fn finished(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) {
        self.flush(H::finished(conn, qh, session_lock));
    }
}

#[allow(dead_code)]
pub fn erase<H, M, F>(sink: F) -> Box<dyn SctkErased>
where
    H: SctkHandler<M> + 'static,
    M: 'static + Debug,
    F: FnMut(M) + 'static,
{
    Box::new(SctkAdapter::<H, M, _>::new(sink))
}

pub(super) struct SctkMuxAdapter<H, M, FU, FR>
where
    H: SctkHandler<M>,
    FU: FnMut(M),
    FR: FnMut(super::RunnerEvent),
{
    user: SctkAdapter<H, M, FU>,
    runner: SctkAdapter<super::RunnerHandler, super::RunnerEvent, FR>,
}

impl<H, M, FU, FR> SctkMuxAdapter<H, M, FU, FR>
where
    H: SctkHandler<M>,
    FU: FnMut(M),
    FR: FnMut(super::RunnerEvent),
{
    pub fn new(user_sink: FU, runner_sink: FR) -> Self {
        Self {
            user: SctkAdapter::new(user_sink),
            runner: SctkAdapter::new(runner_sink),
        }
    }
}

impl<H, M, FU, FR> SctkErased for SctkMuxAdapter<H, M, FU, FR>
where
    H: SctkHandler<M>,
    FU: FnMut(M),
    FR: FnMut(super::RunnerEvent),
{
    fn runtime_add_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        self.user
            .runtime_add_global(conn, qh, name, interface, version);
        self.runner
            .runtime_add_global(conn, qh, name, interface, version);
    }
    fn runtime_remove_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
    ) {
        self.user.runtime_remove_global(conn, qh, name, interface);
        self.runner.runtime_remove_global(conn, qh, name, interface);
    }

    fn new_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.user.new_output(conn, qh, output.clone());
        self.runner.new_output(conn, qh, output);
    }
    fn update_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.user.update_output(conn, qh, output.clone());
        self.runner.update_output(conn, qh, output);
    }
    fn output_destroyed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.user.output_destroyed(conn, qh, output.clone());
        self.runner.output_destroyed(conn, qh, output);
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        time: u32,
    ) {
        self.user.frame(conn, qh, surface, time);
        self.runner.frame(conn, qh, surface, time);
    }
    fn surface_enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.user.surface_enter(conn, qh, surface, output);
        self.runner.surface_enter(conn, qh, surface, output);
    }
    fn surface_leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.user.surface_leave(conn, qh, surface, output);
        self.runner.surface_leave(conn, qh, surface, output);
    }
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.user
            .scale_factor_changed(conn, qh, surface, new_factor);
        self.runner
            .scale_factor_changed(conn, qh, surface, new_factor);
    }
    fn transform_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_transform: smithay_client_toolkit::reexports::client::protocol::wl_output::Transform,
    ) {
        self.user
            .transform_changed(conn, qh, surface, new_transform);
        self.runner
            .transform_changed(conn, qh, surface, new_transform);
    }

    fn closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    ) {
        self.user.closed(conn, qh, layer);
        self.runner.closed(conn, qh, layer);
    }

    fn request_close(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        self.user.request_close(conn, qh, window);
        self.runner.request_close(conn, qh, window);
    }

    fn locked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) {
        self.user.locked(conn, qh, session_lock.clone());
        self.runner.locked(conn, qh, session_lock);
    }
    fn finished(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) {
        self.user.finished(conn, qh, session_lock.clone());
        self.runner.finished(conn, qh, session_lock);
    }
}

pub(super) fn erase_with_runner<H, M, FU, FR>(user_sink: FU, runner_sink: FR) -> Box<dyn SctkErased>
where
    H: SctkHandler<M> + 'static,
    M: 'static + std::fmt::Debug,
    FU: FnMut(M) + 'static,
    FR: FnMut(super::RunnerEvent) + 'static,
{
    Box::new(SctkMuxAdapter::<H, M, _, _>::new(user_sink, runner_sink))
}
