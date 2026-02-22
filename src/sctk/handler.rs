use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};

pub enum Emit<M> {
    None,
    One(M),
}

#[allow(
    unused_variables,
    unused_mut,
    unused_imports,
    clippy::unused_self,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
pub trait SctkHandler<M> {
    // ProvidesRegistryState
    fn runtime_add_global(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
        version: u32,
    ) -> Emit<M> {
        Emit::None
    }
    fn runtime_remove_global(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
    ) -> Emit<M> {
        Emit::None
    }

    // OutputHandler
    fn new_output(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> Emit<M> {
        Emit::None
    }
    fn update_output(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> Emit<M> {
        Emit::None
    }
    fn output_destroyed(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> Emit<M> {
        Emit::None
    }

    // CompositorHandler
    fn frame(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        time: u32,
    ) -> Emit<M> {
        Emit::None
    }
    fn surface_enter(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> Emit<M> {
        Emit::None
    }
    fn surface_leave(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> Emit<M> {
        Emit::None
    }
    fn scale_factor_changed(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_factor: i32,
    ) -> Emit<M> {
        Emit::None
    }
    fn transform_changed(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        new_transform: smithay_client_toolkit::reexports::client::protocol::wl_output::Transform,
    ) -> Emit<M> {
        Emit::None
    }

    // LayerShellHandler
    fn closed(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    ) -> Emit<M> {
        Emit::None
    }

    // WindowHandler
    fn request_close(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) -> Emit<M> {
        Emit::None
    }

    // SessionLockHandler
    fn locked(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) -> Emit<M> {
        Emit::None
    }
    fn finished(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) -> Emit<M> {
        Emit::None
    }
}
