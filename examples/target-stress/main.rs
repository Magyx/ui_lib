//! Target open/close stress test.
//!
//! Drives ONE long-lived `Engine` (one wgpu Instance/Adapter/Device) and then
//! creates + attaches + renders + detaches + destroys a wlr-layer surface in a
//! loop. This reproduces the conditions `orbit_shell` actually runs under -- a
//! shared device with many surfaces attached/detached over time, and attach
//! deferred until *after* the compositor `configure` -- which the plain
//! `sctk-layer` example never exercises (it attaches once, synchronously, to a
//! cold device, then exits).
//!
//! Pass/fail: if it prints "reached N=<target>" the library's surface lifecycle
//! survived the stress under orbit's pattern. If it dies before that, the last
//! "iter <i>: <phase>" line on stderr tells you exactly where (attach / render /
//! destroy). Run with validation on to turn a random driver crash into a precise
//! message:
//!
//!   UI_WGPU_VALIDATION=1 UI_WGPU_DEBUG=1 RUST_BACKTRACE=1 \
//!     cargo run --release --example target-stress --features sctk,vulkan 2>&1 | tee stress.log
//!
//! Tunables via env: STRESS_ITERS (default 200), STRESS_W, STRESS_H (default 256).

use std::sync::Arc;

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::channel as loop_channel,
        client::{Connection, QueueHandle, globals::registry_queue_init},
    },
    registry::RegistryState,
    seat::SeatState,
    session_lock::SessionLockState,
    shell::wlr_layer::LayerShell,
};

use ui::{
    graphics::{Engine, RenderOutcome, TargetId},
    model::Size,
    sctk::{
        Anchor, DefaultHandler, KeyboardInteractivity, Layer, LayerOptions, OutputSelector,
        OutputSet, RawWaylandHandles, SctkEvent, erased, state::SctkState,
    },
    widget::{Element, Rectangle},
};

/// Minimal message + state. We never produce a message; the view is a single
/// (transparent) placeholder, so render still runs the default UI pipeline and
/// presents a frame -- which is what we want to exercise.
#[derive(Debug, Clone)]
struct Msg;

fn view(_tid: &TargetId, _state: &()) -> Element<Msg> {
    Rectangle::placeholder().into()
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iters = env_u32("STRESS_ITERS", 200);
    let w = env_u32("STRESS_W", 256);
    let h = env_u32("STRESS_H", 256);

    // 1) Wayland connection + queue (mirrors run_app_core / orbit's SctkApp::new).
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh: QueueHandle<SctkState> = event_queue.handle();

    // 2) Bind globals.
    let registry = RegistryState::new(&globals);
    let compositor = CompositorState::bind(&globals, &qh)?;
    let outputs = OutputState::new(&globals, &qh);
    let seats = SeatState::new(&globals, &qh);
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let session_lock = SessionLockState::new(&globals, &qh);

    // Message sink: drained but unused for this test.
    let (tx, _rx) = loop_channel::channel::<SctkEvent>();
    let handler = erased::erase::<DefaultHandler, Msg, _>(|_m: Msg| {});

    let mut state = SctkState::new(
        compositor,
        Some(layer_shell),
        None,
        outputs,
        seats,
        registry,
        session_lock,
        handler,
        tx,
    );

    // Let outputs settle so the First selector resolves.
    event_queue.roundtrip(&mut state)?;

    // 3) One long-lived engine, shared across every iteration.
    let mut engine: Engine<'_, Msg> = Engine::default();
    let mut s: () = ();

    let opts = LayerOptions {
        layer: Layer::Overlay,
        size: Size::new(w, h),
        anchors: Anchor::TOP | Anchor::LEFT,
        exclusive_zone: -1,
        keyboard_interactivity: KeyboardInteractivity::None,
        namespace: Some("ui-stress".to_string()),
        output: Some(OutputSet::One(OutputSelector::First)),
    };

    let mut completed = 0u32;
    for i in 0..iters {
        // --- create the wl_surface + layer surface (commits, no buffer yet) ---
        eprintln!("iter {i}: spawn");
        let sids = state.spawn_layer_surfaces(&qh, opts.clone());
        let Some(&sid) = sids.first() else {
            eprintln!("iter {i}: no output matched First selector; aborting");
            break;
        };
        // If the selector ever yields more than one, destroy the extras so the
        // count stays one-surface-per-iteration.
        for extra in sids.iter().skip(1).copied() {
            state.remove_surface_by_surface_id(extra);
        }

        // --- pump events until the compositor has configured this surface ---
        let mut spins = 0;
        let configured = loop {
            event_queue.blocking_dispatch(&mut state)?;
            match state.surfaces.get(&sid) {
                Some(rec) if rec.configured => break true,
                Some(_) => {}
                None => break false, // closed out from under us
            }
            spins += 1;
            if spins > 2000 {
                break false;
            }
        };
        if !configured {
            eprintln!("iter {i}: never configured (spins={spins}); skipping");
            state.remove_surface_by_surface_id(sid);
            let _ = conn.flush();
            continue;
        }

        // --- attach the wgpu surface AFTER configure (orbit's timing) ---
        let (phys, sf) = {
            let rec = &state.surfaces[&sid];
            let sf = rec.scale_factor.max(1) as f64;
            let phys = Size::new(
                rec.size.width * rec.scale_factor.max(1) as u32,
                rec.size.height * rec.scale_factor.max(1) as u32,
            );
            (phys, sf)
        };
        eprintln!("iter {i}: attach {}x{} sf={sf}", phys.width, phys.height);
        let handles = RawWaylandHandles::new(&conn, &state.surfaces[&sid].wl_surface);
        let tid = engine.attach_target(Arc::new(handles), phys, sf);

        // --- render at least one frame (configures swapchain + presents) ---
        eprintln!("iter {i}: render");
        for attempt in 0..3 {
            match engine.render_if_needed(&tid, true, &view, &mut s) {
                Ok(RenderOutcome::NeedsRerender) => continue,
                Ok(_) => break,
                Err(e) => {
                    eprintln!("iter {i}: render error on attempt {attempt}: {e:?}");
                    break;
                }
            }
        }

        // --- detach (drops wgpu Surface) THEN destroy the wl_surface ---
        eprintln!("iter {i}: detach + destroy");
        engine.detach_target(&tid);
        state.remove_surface_by_surface_id(sid);

        // Let the server process the destroy before the next create.
        let _ = conn.flush();
        event_queue.roundtrip(&mut state)?;

        completed += 1;
        eprintln!("iter {i}: OK ({completed} completed)");
    }

    println!("reached N={completed} (target {iters})");
    Ok(())
}
