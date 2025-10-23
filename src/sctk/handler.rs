use super::msg::Emit;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, QueueHandle};

#[allow(
    unused_variables,
    unused_mut,
    unused_imports,
    clippy::unused_self,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
pub trait SctkHandler<M> {
    // Registry/globals
    fn runtime_add_global(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
        version: u32,
    ) -> Emit<M> {
        Emit::none()
    }

    fn runtime_remove_global(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        name: u32,
        interface: &str,
    ) -> Emit<M> {
        Emit::none()
    }

    // Outputs
    fn new_output(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: WlOutput,
    ) -> Emit<M> {
        Emit::none()
    }

    fn update_output(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: WlOutput,
    ) -> Emit<M> {
        Emit::none()
    }

    fn output_destroyed(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: WlOutput,
    ) -> Emit<M> {
        Emit::none()
    }

    // Session Lock
    fn locked(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) -> Emit<M> {
        Emit::none()
    }

    fn finished(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) -> Emit<M> {
        Emit::none()
    }

    fn configure(
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: smithay_client_toolkit::session_lock::SessionLockSurface,
        configure: smithay_client_toolkit::session_lock::SessionLockSurfaceConfigure,
        serial: u32,
    ) -> Emit<M> {
        Emit::none()
    }

    // Keyboard
    fn key(
        seat_id: u32,
        keysym_raw: u32,
        pressed: bool,
        utf8: Option<&str>,
        mods_serialized: u32,
        serial: u32,
        time_msec: u32,
    ) -> Emit<M> {
        Emit::none()
    }

    // Pointer
    fn pointer_motion(seat_id: u32, dx: f64, dy: f64, serial: u32, time_msec: u32) -> Emit<M> {
        Emit::none()
    }
    fn pointer_button(
        seat_id: u32,
        button: u32,
        pressed: bool,
        serial: u32,
        time_msec: u32,
    ) -> Emit<M> {
        Emit::none()
    }
    fn pointer_axis(seat_id: u32, h: f64, v: f64, serial: u32, time_msec: u32) -> Emit<M> {
        Emit::none()
    }

    // Surface/lifecycle
    fn frame() -> Emit<M> {
        Emit::none()
    }
    fn focus_changed(focused: bool) -> Emit<M> {
        Emit::none()
    }
    fn close_requested() -> Emit<M> {
        Emit::none()
    }
}
