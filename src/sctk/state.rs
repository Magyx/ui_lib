use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
};
use wayland_client::{
    Connection, QueueHandle,
    globals::GlobalList,
    protocol::{
        wl_keyboard::WlKeyboard, wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
};

use crate::{
    model::{Position, Size},
    sctk::LayerOptions,
};

use super::{SctkEvent, erased::SctkErased, helpers};

pub struct SctkState {
    // sctk state objects
    registry: RegistryState,
    _compositor: CompositorState,
    outputs: OutputState,
    seats: SeatState,
    _layer_shell: LayerShell,

    // surface & role
    pub wl_surface: WlSurface,
    _layer_surface: LayerSurface,

    // event queue for the generic runner
    handler: Box<dyn SctkErased>,
    events: Vec<SctkEvent>,
    pub size: Size<u32>,
    pub closed: bool,
    pub needs_redraw: bool,
}

impl SctkState {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        _globals: &GlobalList,
        qh: &QueueHandle<Self>,
        opts: LayerOptions<'_>,
        compositor: CompositorState,
        layer_shell: LayerShell,
        outputs: OutputState,
        seats: SeatState,
        registry: RegistryState,
        handler: Box<dyn SctkErased>,
    ) -> anyhow::Result<Self> {
        let chosen_output = opts
            .output
            .as_ref()
            .and_then(|sel| helpers::pick_output(&outputs, sel))
            .or_else(|| outputs.outputs().nth(0));

        let wl_surface = compositor.create_surface(qh);
        let layer_surface = layer_shell.create_layer_surface(
            qh,
            wl_surface.clone(),
            opts.layer,
            opts.namespace,
            chosen_output.as_ref(),
        );

        layer_surface.set_anchor(opts.anchors);
        layer_surface.set_size(opts.size.width, opts.size.height);
        layer_surface.set_keyboard_interactivity(opts.keyboard_interactivity);
        if opts.exclusive_zone != 0 {
            layer_surface.set_exclusive_zone(opts.exclusive_zone);
        }
        layer_surface.commit();

        Ok(Self {
            registry,
            _compositor: compositor,
            outputs,
            seats,
            _layer_shell: layer_shell,

            wl_surface,
            _layer_surface: layer_surface,

            handler,
            events: Vec::new(),
            size: opts.size,
            closed: false,
            needs_redraw: true,
        })
    }

    pub(super) fn take_events(&mut self) -> impl Iterator<Item = SctkEvent> + '_ {
        self.events.drain(..)
    }
}

// === Handlers on SctkState =====================================================================

impl ProvidesRegistryState for SctkState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    fn runtime_add_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        self.handler
            .runtime_add_global(conn, qh, name, interface, version);
    }

    fn runtime_remove_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
    ) {
        self.handler
            .runtime_remove_global(conn, qh, name, interface);
    }
}

impl OutputHandler for SctkState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        self.handler.new_output(conn, qh, output);
    }

    fn update_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        self.handler.update_output(conn, qh, output);
    }

    fn output_destroyed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        self.handler.output_destroyed(conn, qh, output);
    }
}

impl CompositorHandler for SctkState {
    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
    }

    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
    }
}

impl LayerShellHandler for SctkState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.events.push(SctkEvent::Closed);
        self.closed = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // sctk 0.20 gives a struct; 0 means "no constraint".
        let (w, h) = configure.new_size;
        let w = if w == 0 { self.size.width } else { w };
        let h = if h == 0 { self.size.height } else { h };
        if w != self.size.width || h != self.size.height {
            self.size = Size::new(w, h);
            self.events.push(SctkEvent::Resized { size: self.size });
        }

        // Publish our new state
        self.wl_surface.commit();
        self.needs_redraw = true;
    }
}

impl SeatHandler for SctkState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        cap: Capability,
    ) {
        if cap == Capability::Pointer {
            _ = self.seats.get_pointer(qh, &seat);
        }
        if cap == Capability::Keyboard {
            _ = self.seats.get_keyboard(qh, &seat, None);
        }
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        _cap: Capability,
    ) {
    }

    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
    ) {
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
    ) {
    }
}

impl PointerHandler for SctkState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for ev in events {
            match ev.kind {
                PointerEventKind::Enter { .. } => {}
                PointerEventKind::Leave { .. } => {}
                PointerEventKind::Motion { .. } => {
                    let (x, y) = ev.position;
                    self.events.push(SctkEvent::PointerMoved {
                        pos: Position::new(x as f32, y as f32),
                    });
                }
                PointerEventKind::Press { .. } => self.events.push(SctkEvent::PointerDown),
                PointerEventKind::Release { .. } => self.events.push(SctkEvent::PointerUp),
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

impl KeyboardHandler for SctkState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
        _rawkeys: &[u32],
        _keysyms: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let ch = event
            .utf8
            .as_ref()
            .and_then(|s| s.bytes().next())
            .unwrap_or(0);
        self.events.push(SctkEvent::Keyboard { ch });
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

delegate_registry!(SctkState);
delegate_compositor!(SctkState);
delegate_output!(SctkState);
delegate_seat!(SctkState);
delegate_pointer!(SctkState);
delegate_keyboard!(SctkState);
delegate_layer!(SctkState);
