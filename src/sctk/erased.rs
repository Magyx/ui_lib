// ui/sctk_erased.rs
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, QueueHandle};

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
}
