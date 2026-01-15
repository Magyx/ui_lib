use smithay_client_toolkit::{
    reexports::client::{Connection, QueueHandle, protocol::wl_output::WlOutput},
    session_lock::{SessionLock, SessionLockSurface, SessionLockSurfaceConfigure},
};

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
        output: WlOutput,
    );
    fn update_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: WlOutput,
    );
    fn output_destroyed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        output: WlOutput,
    );

    // SessionLockHandler
    fn locked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: SessionLock,
    );

    fn finished(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        session_lock: SessionLock,
    );

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<super::state::SctkState>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        serial: u32,
    );
}
