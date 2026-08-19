use std::collections::HashMap;

use calloop::LoopHandle;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_session_lock, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::channel as loop_channel,
        client::{
            Connection, Proxy, QueueHandle,
            protocol::{
                wl_keyboard::WlKeyboard,
                wl_output::{Transform, WlOutput},
                wl_pointer::WlPointer,
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
        },
    },
    registry::{ProvidesRegistryState, RegistryHandler, RegistryState},
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardData, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
    },
    session_lock::{SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface},
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        xdg::{
            XdgShell,
            window::{Window, WindowHandler},
        },
    },
};

use super::{
    SctkEvent, SurfaceId,
    erased::SctkErased,
    error, helpers,
    options::{LayerOptions, LockOptions, OutputSelector, OutputSet, XdgOptions},
};
use crate::model::{Position, Size};

#[allow(dead_code)]
enum SurfaceRole {
    Layer(LayerSurface),
    Xdg(Window),
    Lock(SessionLockSurface),
}

pub struct SurfaceRec {
    pub configured: bool,
    pub wl_surface: WlSurface,
    pub size: Size<u32>,
    pub scale_factor: i32,
    role: SurfaceRole,
    pub output: Option<WlOutput>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LastDevice {
    Pointer,
    Keyboard,
}

#[derive(Default)]
struct SeatRec {
    pointer: Option<WlPointer>,
    keyboard: Option<WlKeyboard>,
    kbd_serial: Option<u32>,
    kbd_focus: Option<SurfaceId>,
    pointer_focus: Option<SurfaceId>,
}

pub struct SctkState {
    // sctk state objects
    registry: RegistryState,
    _compositor: CompositorState,
    pub outputs: OutputState,
    seats: SeatState,
    _layer_shell: Option<LayerShell>,
    _xdg_shell: Option<XdgShell>,
    session_lock: SessionLockState,

    // surface & role
    pub surfaces: HashMap<SurfaceId, SurfaceRec>,
    by_surface_id: HashMap<u32, SurfaceId>,
    active_lock: Option<SessionLock>,

    // input
    seat_recs: HashMap<WlSeat, SeatRec>,
    last_active: Option<(WlSeat, LastDevice)>,

    // event queue for the generic runner
    handler: Box<dyn SctkErased>,
    event_tx: loop_channel::Sender<SctkEvent>,
    loop_handle: Option<LoopHandle<'static, Self>>,
}

impl SctkState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: RegistryState,
        compositor: CompositorState,
        outputs: OutputState,
        seats: SeatState,
        layer_shell: Option<LayerShell>,
        xdg_shell: Option<XdgShell>,
        session_lock: SessionLockState,
        surfaces: Option<HashMap<SurfaceId, SurfaceRec>>,
        by_surface_id: Option<HashMap<u32, SurfaceId>>,
        handler: Box<dyn SctkErased>,
        event_tx: loop_channel::Sender<SctkEvent>,
    ) -> Self {
        Self {
            registry,
            _compositor: compositor,
            outputs,
            seats,
            _layer_shell: layer_shell,
            _xdg_shell: xdg_shell,
            session_lock,

            surfaces: surfaces.unwrap_or_default(),
            by_surface_id: by_surface_id.unwrap_or_default(),
            seat_recs: HashMap::default(),
            last_active: None,
            active_lock: None,

            handler,
            event_tx,
            loop_handle: None,
        }
    }

    fn same_output(a: Option<&WlOutput>, b: Option<&WlOutput>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.id().protocol_id() == b.id().protocol_id(),
            _ => false,
        }
    }

    fn make_surface(
        out: Option<&WlOutput>,
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
            out,
        );
        layer_surface.set_anchor(opts.anchors);
        layer_surface.set_margin(opts.margin.y, opts.margin.z, opts.margin.w, opts.margin.x);
        layer_surface.set_size(opts.size.width, opts.size.height);
        layer_surface.set_keyboard_interactivity(opts.keyboard_interactivity);
        if opts.exclusive_zone != 0 {
            layer_surface.set_exclusive_zone(opts.exclusive_zone);
        }
        layer_surface.commit();
        (wl_surface, layer_surface)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_for_layer(
        qh: &QueueHandle<Self>,
        opts: LayerOptions,
        compositor: CompositorState,
        layer_shell: LayerShell,
        outputs: OutputState,
        seats: SeatState,
        registry: RegistryState,
        session_lock: SessionLockState,
        handler: Box<dyn SctkErased>,
        event_tx: loop_channel::Sender<SctkEvent>,
    ) -> crate::Result<Self> {
        let chosen =
            helpers::pick_outputs(&outputs, opts.output.as_ref().unwrap_or(&OutputSet::Active));

        let mut surfaces = HashMap::new();
        let mut by_surface_id = HashMap::new();
        for out in chosen {
            let (wl, layer) =
                Self::make_surface(out.as_wl_output(), &compositor, qh, &opts, &layer_shell);
            let sid = SurfaceId(wl.id().protocol_id());
            by_surface_id.insert(layer.wl_surface().id().protocol_id(), sid);
            surfaces.insert(
                sid,
                SurfaceRec {
                    configured: false,
                    wl_surface: wl,
                    size: opts.size,
                    scale_factor: 1,
                    role: SurfaceRole::Layer(layer),
                    output: out.into_option(),
                },
            );
        }

        Ok(Self::new(
            registry,
            compositor,
            outputs,
            seats,
            Some(layer_shell),
            None,
            session_lock,
            Some(surfaces),
            Some(by_surface_id),
            handler,
            event_tx,
        ))
    }

    pub fn spawn_layer_surfaces(
        &mut self,
        qh: &QueueHandle<Self>,
        opts: LayerOptions,
    ) -> Vec<SurfaceId> {
        let layer_shell = self._layer_shell.as_ref().expect("Layer shell not bound");
        let chosen = helpers::pick_outputs(
            &self.outputs,
            opts.output.as_ref().unwrap_or(&OutputSet::Active),
        );

        let mut surfaces = Vec::new();
        for out in chosen {
            let (wl, layer) = Self::make_surface(
                out.as_wl_output(),
                &self._compositor,
                qh,
                &opts,
                layer_shell,
            );
            let sid = SurfaceId(wl.id().protocol_id());
            self.by_surface_id
                .insert(layer.wl_surface().id().protocol_id(), sid);
            self.surfaces.insert(
                sid,
                SurfaceRec {
                    configured: false,
                    wl_surface: wl,
                    size: opts.size,
                    scale_factor: 1,
                    role: SurfaceRole::Layer(layer),
                    output: out.into_option(),
                },
            );
            surfaces.push(sid);
        }
        surfaces
    }

    pub fn ensure_layer_surfaces(
        &mut self,
        qh: &QueueHandle<Self>,
        opts: &LayerOptions,
    ) -> Vec<(SurfaceId, Size<u32>)> {
        let layer_shell = self._layer_shell.as_ref().expect("Layer shell not bound");
        let desired = helpers::pick_outputs(
            &self.outputs,
            opts.output.as_ref().unwrap_or(&OutputSet::Active),
        );

        let mut created = Vec::new();
        for out in desired {
            let exists = self.surfaces.values().any(|rec| {
                matches!(rec.role, SurfaceRole::Layer(_))
                    && Self::same_output(rec.output.as_ref(), out.as_wl_output())
            });
            if exists {
                continue;
            }

            let (wl, layer) =
                Self::make_surface(out.as_wl_output(), &self._compositor, qh, opts, layer_shell);
            let sid = SurfaceId(wl.id().protocol_id());
            self.by_surface_id
                .insert(layer.wl_surface().id().protocol_id(), sid);
            self.surfaces.insert(
                sid,
                SurfaceRec {
                    configured: false,
                    wl_surface: wl,
                    size: opts.size,
                    scale_factor: 1,
                    role: SurfaceRole::Layer(layer),
                    output: out.into_option(),
                },
            );
            created.push((sid, opts.size));
        }
        created
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_for_window(
        qh: &QueueHandle<Self>,
        opts: XdgOptions,
        compositor: CompositorState,
        xdg_shell: XdgShell,
        outputs: OutputState,
        seats: SeatState,
        registry: RegistryState,
        session_lock: SessionLockState,
        handler: Box<dyn SctkErased>,
        event_tx: loop_channel::Sender<SctkEvent>,
    ) -> crate::Result<Self> {
        let wl_surface = compositor.create_surface(qh);
        let window = xdg_shell.create_window(wl_surface, opts.decorations, qh);

        window.set_title(&opts.title);
        if let Some(app_id) = &opts.app_id {
            window.set_app_id(app_id);
        }
        window.set_min_size(None);
        window.set_max_size(None);
        window.commit();

        let output =
            helpers::pick_output(&outputs, &opts.output.unwrap_or(OutputSelector::First)).ok();

        let mut surfaces = HashMap::with_capacity(1);
        let mut by_surface_id = HashMap::with_capacity(1);
        let sid = SurfaceId(window.wl_surface().id().protocol_id());
        by_surface_id.insert(window.wl_surface().id().protocol_id(), sid);
        surfaces.insert(
            sid,
            SurfaceRec {
                configured: false,
                wl_surface: window.wl_surface().clone(),
                size: opts.size,
                scale_factor: 1,
                role: SurfaceRole::Xdg(window),
                output,
            },
        );

        Ok(Self::new(
            registry,
            compositor,
            outputs,
            seats,
            None,
            Some(xdg_shell),
            session_lock,
            Some(surfaces),
            Some(by_surface_id),
            handler,
            event_tx,
        ))
    }

    pub fn spawn_window(&mut self, qh: &QueueHandle<Self>, mut opts: XdgOptions) -> SurfaceId {
        let xdg = self._xdg_shell.as_ref().expect("XDG shell not bound");
        let wl_surface = self._compositor.create_surface(qh);
        let window = xdg.create_window(wl_surface.clone(), opts.decorations, qh);

        window.set_title(&opts.title);
        if let Some(app_id) = &opts.app_id {
            window.set_app_id(app_id);
        }
        window.set_min_size(None);
        window.set_max_size(None);
        window.commit();

        let output = super::helpers::pick_output(
            &self.outputs,
            &opts.output.take().unwrap_or(OutputSelector::First),
        )
        .ok();

        let sid = SurfaceId(window.wl_surface().id().protocol_id());
        self.by_surface_id
            .insert(window.wl_surface().id().protocol_id(), sid);
        self.surfaces.insert(
            sid,
            SurfaceRec {
                configured: false,
                wl_surface: window.wl_surface().clone(),
                size: opts.size,
                scale_factor: 1,
                role: SurfaceRole::Xdg(window),
                output,
            },
        );
        sid
    }

    fn emit_event(&self, ev: SctkEvent) {
        let _ = self.event_tx.send(ev);
    }

    pub fn set_loop_handle(&mut self, handle: LoopHandle<'static, SctkState>) {
        self.loop_handle = Some(handle);
    }

    fn seat_rec_mut(&mut self, seat: &WlSeat) -> &mut SeatRec {
        self.seat_recs.entry(seat.clone()).or_default()
    }
    fn seat_of_pointer(pointer: &WlPointer) -> Option<WlSeat> {
        pointer.data::<PointerData>().map(|d| d.seat().clone())
    }
    fn seat_of_keyboard(keyboard: &WlKeyboard) -> Option<WlSeat> {
        keyboard
            .data::<KeyboardData<Self>>()
            .map(|d| d.seat().clone())
    }

    pub fn grab_source(&self) -> Option<(&WlSeat, u32)> {
        let (seat, device) = self.last_active.as_ref()?;
        let rec = self.seat_recs.get(seat)?;
        let serial = match device {
            LastDevice::Pointer => {
                let data = rec.pointer.as_ref()?.data::<PointerData>()?;
                data.latest_button_serial()
                    .or_else(|| data.latest_enter_serial())?
            }
            LastDevice::Keyboard => rec.kbd_serial?,
        };
        Some((seat, serial))
    }
    pub fn cursor_source(&self) -> Option<(&WlPointer, u32)> {
        let (seat, _) = self.last_active.as_ref()?;
        let pointer = self.seat_recs.get(seat)?.pointer.as_ref()?;
        let serial = pointer.data::<PointerData>()?.latest_enter_serial()?;
        Some((pointer, serial))
    }

    pub fn surface_id_by_protocol_id(&self, id: u32) -> Option<&SurfaceId> {
        self.by_surface_id.get(&id)
    }

    pub fn remove_surface_by_surface_id(&mut self, sid: SurfaceId) {
        if let Some(sid) = self.by_surface_id.remove(&sid.0) {
            self.surfaces.remove(&sid);
            for rec in self.seat_recs.values_mut() {
                if rec.kbd_focus == Some(sid) {
                    rec.kbd_focus = None;
                }
                if rec.pointer_focus == Some(sid) {
                    rec.pointer_focus = None;
                }
            }
        }
    }

    pub fn lock_session(
        &mut self,
        qh: &QueueHandle<Self>,
        opts: LockOptions,
    ) -> crate::Result<Vec<SurfaceId>> {
        let lock = self
            .session_lock
            .lock(qh)
            .map_err(error::SctkError::session_lock)?;
        let chosen: Vec<WlOutput> = super::helpers::pick_outputs(
            &self.outputs,
            opts.output.as_ref().unwrap_or(&OutputSet::Active),
        )
        .into_iter()
        .filter_map(|o| o.into_option())
        .collect();

        self.active_lock = Some(lock.clone());

        let mut created = Vec::new();
        for out in chosen {
            let wl_surface = self._compositor.create_surface(qh);
            let lock_surface = lock.create_lock_surface(wl_surface.clone(), &out, qh);

            let sid = SurfaceId(wl_surface.id().protocol_id());
            self.by_surface_id
                .insert(wl_surface.id().protocol_id(), sid);
            self.surfaces.insert(
                sid,
                SurfaceRec {
                    configured: false,
                    wl_surface,
                    size: opts.size,
                    scale_factor: 1,
                    role: SurfaceRole::Lock(lock_surface),
                    output: Some(out),
                },
            );
            created.push(sid);
        }
        Ok(created)
    }

    pub fn ensure_lock_surfaces(
        &mut self,
        qh: &QueueHandle<Self>,
        opts: &LockOptions,
    ) -> crate::Result<Vec<(SurfaceId, Size<u32>)>> {
        let Some(lock) = self.active_lock.clone() else {
            return Ok(Vec::new());
        };
        if !lock.is_locked() {
            return Ok(Vec::new());
        }

        let desired: Vec<WlOutput> = super::helpers::pick_outputs(
            &self.outputs,
            opts.output.as_ref().unwrap_or(&OutputSet::Active),
        )
        .into_iter()
        .filter_map(|o| o.into_option())
        .collect();

        let mut created = Vec::new();
        for out in desired {
            let out_id = out.id().protocol_id();

            let exists = self.surfaces.values().any(|rec| {
                matches!(rec.role, SurfaceRole::Lock(_))
                    && rec
                        .output
                        .as_ref()
                        .is_some_and(|o| o.id().protocol_id() == out_id)
            });
            if exists {
                continue;
            }

            let wl_surface = self._compositor.create_surface(qh);
            let lock_surface = lock.create_lock_surface(wl_surface.clone(), &out, qh);

            let sid = SurfaceId(wl_surface.id().protocol_id());
            self.by_surface_id
                .insert(wl_surface.id().protocol_id(), sid);

            self.surfaces.insert(
                sid,
                SurfaceRec {
                    configured: false,
                    wl_surface,
                    size: opts.size,
                    scale_factor: 1,
                    role: SurfaceRole::Lock(lock_surface),
                    output: Some(out),
                },
            );
            created.push((sid, opts.size));
        }
        Ok(created)
    }

    pub fn has_active_lock(&self) -> bool {
        self.active_lock.is_some()
    }

    /// Be sure to flush and/or let another event cycle happen.
    pub fn unlock_session(&mut self) {
        if let Some(lock) = self.active_lock.take()
            && lock.is_locked()
        {
            lock.unlock();
        }
    }
}

// Handlers on SctkState

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
        <OutputState as RegistryHandler<Self>>::new_global(
            self, conn, qh, name, interface, version,
        );
        <SeatState as RegistryHandler<Self>>::new_global(self, conn, qh, name, interface, version);

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
        <OutputState as RegistryHandler<Self>>::remove_global(self, conn, qh, name, interface);
        <SeatState as RegistryHandler<Self>>::remove_global(self, conn, qh, name, interface);

        self.handler
            .runtime_remove_global(conn, qh, name, interface);
    }
}

// TODO: propagate new_output and output_destroyed when
impl OutputHandler for SctkState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {
        self.handler.new_output(conn, qh, output);
    }

    fn update_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {
        self.handler.update_output(conn, qh, output);
    }

    fn output_destroyed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {
        self.handler.output_destroyed(conn, qh, output);
    }
}

impl CompositorHandler for SctkState {
    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, time: u32) {
        self.handler.frame(conn, qh, surface, time);
    }

    fn surface_enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        output: &WlOutput,
    ) {
        self.handler.surface_enter(conn, qh, surface, output);
    }

    fn surface_leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        output: &WlOutput,
    ) {
        self.handler.surface_leave(conn, qh, surface, output);
    }

    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_factor: i32,
    ) {
        let proto_id = surface.id().protocol_id();
        if let Some(&sid) = self.by_surface_id.get(&proto_id)
            && let Some(rec) = self.surfaces.get_mut(&sid)
        {
            rec.scale_factor = new_factor;

            let phys_w = rec.size.width * new_factor as u32;
            let phys_h = rec.size.height * new_factor as u32;
            self.emit_event(SctkEvent::ScaleChanged {
                surface: sid,
                factor: new_factor,
            });
            self.emit_event(SctkEvent::Resized {
                surface: sid,
                size: Size::new(phys_w, phys_h),
            });
        }

        self.handler
            .scale_factor_changed(conn, qh, surface, new_factor);
    }

    fn transform_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_transform: Transform,
    ) {
        self.handler
            .transform_changed(conn, qh, surface, new_transform);
    }
}

impl LayerShellHandler for SctkState {
    fn closed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, layer: &LayerSurface) {
        let lid = layer.wl_surface().id().protocol_id();
        if let Some(&sid) = self.by_surface_id.get(&lid) {
            self.emit_event(SctkEvent::Closed { surface: sid });
        }
        self.handler.closed(conn, qh, layer);
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        serial: u32,
    ) {
        let lid = layer.wl_surface().id().protocol_id();
        if let Some(sid) = self.by_surface_id.get(&lid).copied()
            && let Some(rec) = self.surfaces.get_mut(&sid)
        {
            rec.configured = true;

            let (w, h) = configure.new_size;
            if w != 0 && h != 0 {
                let new_size = Size::new(w, h);
                if new_size != rec.size {
                    rec.size = new_size;
                    let sf = rec.scale_factor.max(1) as u32;
                    self.emit_event(SctkEvent::Resized {
                        surface: sid,
                        size: Size::new(w * sf, h * sf),
                    });
                }
            }

            self.handler
                .layer_configure(conn, qh, layer, configure, serial);
        }

        layer.wl_surface().commit();
    }
}

impl WindowHandler for SctkState {
    fn request_close(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window) {
        let wid = window.wl_surface().id().protocol_id();
        if let Some(&sid) = self.by_surface_id.get(&wid) {
            self.emit_event(SctkEvent::Closed { surface: sid });
        }
        self.handler.request_close(conn, qh, window);
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        serial: u32,
    ) {
        let wid = window.wl_surface().id().protocol_id();
        if let Some(sid) = self.by_surface_id.get(&wid).copied()
            && let Some(rec) = self.surfaces.get_mut(&sid)
            && let (Some(w), Some(h)) = configure.new_size
        {
            rec.configured = true;

            let new_size = Size::new(w.get(), h.get());
            if new_size != rec.size {
                rec.size = new_size;
                let sf = rec.scale_factor.max(1) as u32;
                self.emit_event(SctkEvent::Resized {
                    surface: sid,
                    size: Size::new(w.get() * sf, h.get() * sf),
                });
            }

            self.handler
                .window_configure(conn, qh, window, configure, serial);
        }

        window.wl_surface().commit();
    }
}

impl SessionLockHandler for SctkState {
    fn locked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) {
        self.handler.locked(conn, qh, session_lock);
    }

    fn finished(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) {
        self.active_lock = None;

        for (sid, key) in self
            .surfaces
            .iter()
            .filter_map(|(sid, rec)| {
                if let SurfaceRole::Lock(_) = rec.role {
                    Some((*sid, rec.wl_surface.id().protocol_id()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
        {
            self.surfaces.remove(&sid);
            self.by_surface_id.remove(&key);
            for rec in self.seat_recs.values_mut() {
                if rec.kbd_focus == Some(sid) {
                    rec.kbd_focus = None;
                }
            }
        }

        self.handler.finished(conn, qh, session_lock);
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: smithay_client_toolkit::session_lock::SessionLockSurface,
        configure: smithay_client_toolkit::session_lock::SessionLockSurfaceConfigure,
        serial: u32,
    ) {
        let lid = surface.wl_surface().id().protocol_id();
        if let Some(sid) = self.by_surface_id.get(&lid).copied()
            && let Some(rec) = self.surfaces.get_mut(&sid)
        {
            rec.configured = true;

            let (w, h) = configure.new_size;
            if w != 0 && h != 0 {
                let new_size = Size::new(w, h);
                if new_size != rec.size {
                    rec.size = new_size;
                    let sf = rec.scale_factor.max(1) as u32;
                    self.emit_event(SctkEvent::Resized {
                        surface: sid,
                        size: Size::new(w * sf, h * sf),
                    });
                }
            }
        }

        if let Some(sid) = self.by_surface_id.get(&lid).copied()
            && self.surfaces.get_mut(&sid).is_some()
        {
            self.handler
                .lock_configure(conn, qh, surface.clone(), configure, serial);
        }
    }
}

impl SeatHandler for SctkState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: WlSeat) {
        self.seat_recs.remove(&seat);
        if self.last_active.as_ref().is_some_and(|(s, _)| *s == seat) {
            self.last_active = None;
        }
    }

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
            Capability::Keyboard => match self.loop_handle.clone() {
                Some(lh) => {
                    let kbd = self.seats.get_keyboard_with_repeat(
                        qh,
                        &seat,
                        None,
                        lh,
                        Box::new(|state, kdb, event| {
                            let sid = SctkState::seat_of_keyboard(kdb)
                                .and_then(|s| state.seat_recs.get(&s))
                                .and_then(|r| r.kbd_focus);
                            if let Some(sid) = sid {
                                state.emit_event(SctkEvent::Key {
                                    surface: sid,
                                    raw_code: event.raw_code,
                                    keysym: event.keysym,
                                    utf8: event.utf8,
                                    pressed: true,
                                    repeat: true,
                                })
                            }
                        }),
                    );
                    if let Ok(kbd) = kbd {
                        self.seat_rec_mut(&seat).keyboard = Some(kbd);
                    }
                }
                None => {
                    if let Ok(kbd) = self.seats.get_keyboard(qh, &seat, None) {
                        self.seat_rec_mut(&seat).keyboard = Some(kbd);
                    }
                }
            },
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
        pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        let seat = Self::seat_of_pointer(pointer);

        for ev in events {
            let sid = match self.by_surface_id.get(&ev.surface.id().protocol_id()) {
                Some(&sid) => sid,
                None => continue,
            };

            let sf = self
                .surfaces
                .get(&sid)
                .map(|r| r.scale_factor.max(1) as f32)
                .unwrap_or(1.0);
            let (px, py) = ev.position;
            let pos = Position::new(px as f32 * sf, py as f32 * sf);

            match ev.kind {
                PointerEventKind::Enter { .. } => {
                    if let Some(seat) = &seat {
                        self.seat_rec_mut(seat).pointer_focus = Some(sid);
                        self.last_active = Some((seat.clone(), LastDevice::Pointer));
                    }
                    self.emit_event(SctkEvent::PointerEntered { surface: sid, pos });
                }
                PointerEventKind::Leave { .. } => {
                    if let Some(seat) = &seat {
                        let rec = self.seat_rec_mut(seat);
                        if rec.pointer_focus == Some(sid) {
                            rec.pointer_focus = None;
                        }
                    }
                    self.emit_event(SctkEvent::PointerLeft { surface: sid });
                }
                PointerEventKind::Motion { .. } => {
                    self.emit_event(SctkEvent::PointerMoved { surface: sid, pos });
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(seat) = &seat {
                        self.last_active = Some((seat.clone(), LastDevice::Pointer));
                    }
                    self.emit_event(SctkEvent::PointerButton {
                        surface: sid,
                        button,
                        pressed: true,
                    });
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(seat) = &seat {
                        self.last_active = Some((seat.clone(), LastDevice::Pointer));
                    }
                    self.emit_event(SctkEvent::PointerButton {
                        surface: sid,
                        button,
                        pressed: false,
                    });
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    source,
                    ..
                } => {
                    self.emit_event(SctkEvent::PointerAxis {
                        surface: sid,
                        h: horizontal.absolute,
                        v: vertical.absolute,
                        h120: horizontal.value120,
                        v120: vertical.value120,
                        source,
                    });
                }
            }
        }
    }
}

impl KeyboardHandler for SctkState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        surface: &WlSurface,
        serial: u32,
        _rawkeys: &[u32],
        _keysyms: &[Keysym],
    ) {
        let sid = SurfaceId(surface.id().protocol_id());
        if let Some(seat) = Self::seat_of_keyboard(keyboard) {
            let rec = self.seat_rec_mut(&seat);
            rec.kbd_focus = Some(sid);
            rec.kbd_serial = Some(serial);
            self.last_active = Some((seat, LastDevice::Keyboard));
        }
        self.emit_event(SctkEvent::Focus {
            surface: sid,
            focused: true,
        });
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
    ) {
        let sid = SurfaceId(surface.id().protocol_id());
        let Some(seat) = Self::seat_of_keyboard(keyboard) else {
            return;
        };
        let rec = self.seat_rec_mut(&seat);
        if rec.kbd_focus == Some(sid) {
            rec.kbd_focus = None;
            self.emit_event(SctkEvent::Focus {
                surface: sid,
                focused: false,
            });
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        let seat = Self::seat_of_keyboard(keyboard);
        if let Some(seat) = &seat {
            let rec = self.seat_rec_mut(seat);
            rec.kbd_serial = Some(serial);
            self.last_active = Some((seat.clone(), LastDevice::Keyboard));
        }
        let focus = seat
            .as_ref()
            .and_then(|s| self.seat_recs.get(s))
            .and_then(|r| r.kbd_focus);
        if let Some(sid) = focus {
            self.emit_event(SctkEvent::Key {
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
        keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let seat = Self::seat_of_keyboard(keyboard);
        let focus = seat
            .as_ref()
            .and_then(|s| self.seat_recs.get(s))
            .and_then(|r| r.kbd_focus);
        if let Some(sid) = focus {
            self.emit_event(SctkEvent::Key {
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
        keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let seat = Self::seat_of_keyboard(keyboard);
        let focus = seat
            .as_ref()
            .and_then(|s| self.seat_recs.get(s))
            .and_then(|r| r.kbd_focus);
        if let Some(sid) = focus {
            self.emit_event(SctkEvent::Key {
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
        keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        let focus = Self::seat_of_keyboard(keyboard)
            .and_then(|s| self.seat_recs.get(&s))
            .and_then(|r| r.kbd_focus);
        if let Some(sid) = focus {
            self.emit_event(SctkEvent::Modifiers(sid, modifiers));
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
delegate_session_lock!(SctkState);
delegate_xdg_shell!(SctkState);
delegate_xdg_window!(SctkState);
