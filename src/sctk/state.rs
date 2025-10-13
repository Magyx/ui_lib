use std::collections::HashMap;

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
    Connection, Proxy, QueueHandle,
    globals::GlobalList,
    protocol::{
        wl_keyboard::WlKeyboard, wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
};

use crate::{
    model::{Position, Size},
    sctk::{LayerOptions, OutputSelector, OutputSet, SurfaceId},
};

use super::{SctkEvent, erased::SctkErased, helpers};

pub struct SurfaceRec {
    pub wl_surface: WlSurface,
    _layer_surface: LayerSurface,
    _output: WlOutput,
    pub size: Size<u32>,
}

pub struct SctkState {
    // sctk state objects
    registry: RegistryState,
    _compositor: CompositorState,
    outputs: OutputState,
    seats: SeatState,
    _layer_shell: LayerShell,

    // surface & role
    pub surfaces: HashMap<SurfaceId, SurfaceRec>,
    by_surface_id: HashMap<u32, SurfaceId>,
    kbd_focus: Option<SurfaceId>,

    // event queue for the generic runner
    handler: Box<dyn SctkErased>,
    events: Vec<SctkEvent>,
    pub closed: bool,
    pub needs_redraw: bool,
}

impl SctkState {
    fn make_surface(
        out: &WlOutput,
        compositor: &CompositorState,
        qh: &QueueHandle<Self>,
        opts: &LayerOptions,
        layer_shell: &LayerShell,
    ) -> (WlSurface, LayerSurface) {
        let wl_surface = compositor.create_surface(qh);
        let layer_surface = layer_shell.create_layer_surface(
            qh,
            wl_surface.clone(),
            opts.layer,
            opts.namespace.as_ref(),
            Some(out),
        );
        layer_surface.set_anchor(opts.anchors);
        layer_surface.set_size(opts.size.width, opts.size.height);
        layer_surface.set_keyboard_interactivity(opts.keyboard_interactivity);
        if opts.exclusive_zone != 0 {
            layer_surface.set_exclusive_zone(opts.exclusive_zone);
        }
        layer_surface.commit();
        (wl_surface, layer_surface)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        _globals: &GlobalList,
        qh: &QueueHandle<Self>,
        opts: LayerOptions,
        compositor: CompositorState,
        layer_shell: LayerShell,
        outputs: OutputState,
        seats: SeatState,
        registry: RegistryState,
        handler: Box<dyn SctkErased>,
    ) -> anyhow::Result<Self> {
        let chosen = helpers::pick_outputs(
            &outputs,
            opts.output
                .as_ref()
                .unwrap_or(&OutputSet::One(OutputSelector::First)),
        );

        let mut surfaces = HashMap::new();
        let mut by_surface_id = HashMap::new();
        for out in chosen {
            let (wl, layer) = Self::make_surface(&out, &compositor, qh, &opts, &layer_shell);
            let sid = SurfaceId(wl.id().protocol_id());
            by_surface_id.insert(layer.wl_surface().id().protocol_id(), sid);
            surfaces.insert(
                sid,
                SurfaceRec {
                    wl_surface: wl,
                    _layer_surface: layer,
                    _output: out,
                    size: opts.size,
                },
            );
        }

        Ok(Self {
            registry,
            _compositor: compositor,
            outputs,
            seats,
            _layer_shell: layer_shell,

            surfaces,
            by_surface_id,
            kbd_focus: None,

            handler,
            events: Vec::new(),
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

// TODO: propagate new_output and output_destroyed when
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
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let lid = layer.wl_surface().id().protocol_id();
        if let Some(sid) = self.by_surface_id.get(&lid).copied()
            && let Some(s) = self.surfaces.get_mut(&sid)
        {
            let (w, h) = configure.new_size;
            let w = if w == 0 { s.size.width } else { w };
            let h = if h == 0 { s.size.height } else { h };
            if w != s.size.width || h != s.size.height {
                s.size = Size::new(w, h);
                self.events.push(SctkEvent::Resized {
                    surface: sid,
                    size: s.size,
                });
            }
        }

        // Publish our new state
        layer.wl_surface().commit();
        self.needs_redraw = true;
    }
}

impl SeatHandler for SctkState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        cap: Capability,
    ) {
        match cap {
            Capability::Pointer => {
                _ = self.seats.get_pointer(qh, &seat);
            }
            Capability::Keyboard => {
                _ = self.seats.get_keyboard(qh, &seat, None);
            }
            _ => { /* Not supported atm */ }
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
            let sid = match self.by_surface_id.get(&ev.surface.id().protocol_id()) {
                Some(&sid) => sid,
                None => continue,
            };

            match ev.kind {
                PointerEventKind::Enter { .. } => {}
                PointerEventKind::Leave { .. } => {}
                PointerEventKind::Motion { .. } => {
                    let (x, y) = ev.position;
                    self.events.push(SctkEvent::PointerMoved {
                        surface: sid,
                        pos: Position::new(x as f32, y as f32),
                    });
                }
                PointerEventKind::Press { .. } => {
                    self.events.push(SctkEvent::PointerDown { surface: sid })
                }
                PointerEventKind::Release { .. } => {
                    self.events.push(SctkEvent::PointerUp { surface: sid })
                }
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
        surface: &WlSurface,
        _serial: u32,
        _rawkeys: &[u32],
        _keysyms: &[Keysym],
    ) {
        self.kbd_focus = Some(SurfaceId(surface.id().protocol_id()));
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
    ) {
        self.kbd_focus = None;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(sid) = self.kbd_focus {
            self.events.push(SctkEvent::Key {
                surface: sid,
                raw_code: event.raw_code,
                keysym: event.keysym,
                utf8: event.utf8.clone(),
                pressed: true,
                repeat: false,
            });
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(sid) = self.kbd_focus {
            self.events.push(SctkEvent::Key {
                surface: sid,
                raw_code: event.raw_code,
                keysym: event.keysym,
                utf8: None,
                pressed: false,
                repeat: false,
            });
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(sid) = self.kbd_focus {
            self.events.push(SctkEvent::Key {
                surface: sid,
                raw_code: event.raw_code,
                keysym: event.keysym,
                utf8: event.utf8.clone(),
                pressed: true,
                repeat: true,
            });
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        if let Some(sid) = self.kbd_focus {
            self.events.push(SctkEvent::Modifiers(sid, modifiers));
        }
    }
}

delegate_registry!(SctkState);
delegate_compositor!(SctkState);
delegate_output!(SctkState);
delegate_seat!(SctkState);
delegate_pointer!(SctkState);
delegate_keyboard!(SctkState);
delegate_layer!(SctkState);
