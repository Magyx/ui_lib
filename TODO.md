# UI Library — TODO

<!--toc:start-->
- [UI Library — TODO](#ui-library-todo)
  - [Bugs & fixes](#bugs-fixes)
    - [P0 — correctness blockers](#p0-correctness-blockers)
    - [P1 — real user-visible bugs](#p1-real-user-visible-bugs)
    - [P2 — fix when adjacent](#p2-fix-when-adjacent)
    - [P3 — polish](#p3-polish)
  - [Features](#features)
    - [P1 — table-stakes for "semi-feature-complete"](#p1-table-stakes-for-semi-feature-complete)
    - [P2 — meaningful additions](#p2-meaningful-additions)
    - [P3 — eventually](#p3-eventually)
  - [Suggested execution order](#suggested-execution-order)
<!--toc:end-->

Priorities:

- **P0** — correctness blocker. Fix before anything else; the library is unreliable until these land.
- **P1** — high-impact gap or bug. Real users hit these within minutes.
- **P2** — meaningful but workable around. Fix when adjacent code is being touched.
- **P3** — polish, nice-to-have, or small infra cleanup.

---

## Bugs & fixes

### P0 — correctness blockers

- [x] **Scrollable is fundamentally broken.** Three interlocking issues; fix as a single PR.
  - `__Node.content_size.height` is set to `current_size.height` in `measure_height` (`layout.rs:4044, 4057`), so `Scrollable::content_h` always equals viewport height and the wheel handler's `max > 0` guard is never true. `content_size` must track the unconstrained natural size separately.
  - `write_back` (`layout.rs:3627`) writes the layout engine's unscrolled `pos` into widgets via `set_layout`. Visual paint applies `children_offset` (`layout.rs:3556, 3574–3578`) but widget `self.x/self.y` stay unscrolled, so hit-testing inside scrolled regions is wrong. Thread the cumulative offset into `write_back` or store the post-offset position on the node.
  - `Scrollable::handle` reads `self.content_h` but `prepare()` (which sets it) runs *after* `handle()` in `Engine::poll`. On frame N, scroll uses frame N−1's content height; on frame 0 it's zero.
- [ ] **Double-click / double-emit bug.** `Engine::poll` (`graphics.rs:3101`) and `render_if_needed` (`graphics.rs:3166`) both run `root.handle(event: None)`, but `mouse_buttons_pressed`/`released` are only cleared at the *start* of `handle_platform_event` (`graphics.rs:3246–3247`). Released bits leak into redraw frames; buttons re-fire `on_press`. Clear pressed/released after the render pass too, or make widgets only consult them when `ctx.event` is `Some(MouseButton)`.
- [ ] **Two `handle` passes per frame.** `Engine::poll` and `Engine::render_if_needed` both call `root.handle()` with `event: None`. Wasted traversal and the proximate cause of #2 above. The TODO at `graphics.rs:3159` acknowledges this. Split into `update` (per-frame mouse-state-derived hover/active) vs `handle_event` (only when there's a discrete event).
- [ ] **`ViewState` is never invalidated.** `graphics.rs:3127` notes this. Removed widgets leak entries forever, and a list that shrinks then grows will hand stale state to new widgets at the same indices. Mark touched IDs during the frame and sweep unreferenced ones at end-of-frame.
- [ ] **Hard 1024-node panic.** `layout.rs:3333`'s `MAX_NODES = 1024` plus `assert!` in `create_node` is a runtime crash for users with moderately-sized trees. Switch to `Vec` or return `Result`.

### P1 — real user-visible bugs

- [ ] **Atlas has no eviction.** `render/texture.rs:5946` TODO. Once full, glyph upload silently returns `None` and `Text::paint` skips those glyphs — text just disappears. Add LRU eviction or grow-on-full.
- [ ] **No HiDPI / scale-factor handling.** `Target::scale` is set to `1` (`graphics.rs:2921`) and never read. Layout, fonts, and mouse coords all run in physical pixels — UI looks tiny on 2× displays. Pick a model (logical-px throughout, scale at render) and apply consistently.
- [ ] **`Modifiers` snapshot has no home.** `event.rs:2530` plumbs `ModifiersChanged` through `UiEventRef`, but `Context` (`context.rs:2066`) has no `modifiers` field. Any widget that wants Shift+Arrow, Ctrl+C, etc. has nowhere to read modifier state from. Add `pub modifiers: Modifiers` to `Context`.
- [ ] **No keyed children / stable identity across reorder.** Identity comes from `mix64(parent, idx)` (`layout.rs:3411, 3648`), which is purely positional. Sort, filter, or reorder a list and state attaches to the wrong items. Add an explicit `key` mechanism (à la React) or accept `supplied_id` more broadly.
- [ ] **`Scrollable` mouse-y compensation is incomplete.** `scroll.rs:10596–10609` shifts `ctx.ui.mouse_pos.y` for the child's `handle()` but leaves `ctx.event` untouched, so widgets reading `Ui::CursorMoved` from `ctx.event` see unshifted coords. Also one-axis only — won't generalize when horizontal scroll is added. Drop entirely once #1 (offset-aware `write_back`) lands.
- [ ] **Slider value pinned at edges.** `slider.rs:10797` maps cursor across `[self.x, self.x + self.w]` without accounting for knob radius. Click at the very left edge → knob center is forced inside the track but value is `lo`; the knob jumps off the cursor. Either subtract half the knob width on both sides for the value calculation, or document that the track is the value range.
- [ ] **Slider value-change epsilon is meaningless.** `slider.rs:10799` uses `f32::EPSILON ≈ 1.19e-7` as the change threshold — any movement clears it. Either always emit on drag, or use a sensible threshold (e.g. one pixel of cursor movement at the current width).
- [ ] **Click-to-cursor in `TextInput` is wrong inside scrollables.** `text_input.rs:12148` uses `inner_bounds()` derived from `self.x`/`self.y`, which carry the unscrolled position (see #1). Fixed automatically once #1 lands; flagging because it's user-visible.
- [ ] **Grid doesn't actually align columns.** `grid.rs` builds rows of independently-laid-out `Row`s, so column widths drift across rows. A grid that doesn't align columns isn't a grid. Either rename (`WrappingRows`?) or do a real grid pass that finds per-column max widths first.

### P2 — fix when adjacent

- [ ] **`Text` reshapes on every layout, prepare, and `min_height_for_width`.** `text.rs:11385–11424, 11447, 11489` — three full `set_text` + `shape_until_scroll` cycles per frame per text node. TODO at `text.rs:11483` acknowledges it. Cache shaped output keyed on `(text, attrs, wrap, width)`.
- [ ] **`Text` min-width uses `split_whitespace`.** `text.rs:11405` — wrong for CJK / Thai / any script without whitespace word boundaries. Returns the whole string as one "word," producing a min-width that prevents wrapping. Use cosmic-text's Unicode line-break opportunities.
- [ ] **No event consumption.** `Button::handle` (`button.rs:9375`) sets `active_item = self.id` on press regardless of whether another widget already claimed it. Overlapping widgets all set themselves; the last in iteration order wins. Works by accident for overlays because handle order ≈ paint order, but fragile. Add explicit "topmost hit consumes the event."
- [ ] **`Scrollable` HIT_SLOP overlaps content.** `scroll.rs:10495` — 4px slop on a 6px track produces a ~14px hit zone that overlaps surrounding content. Outside-bounds clicks just inside the right edge accidentally start scroll drags. Tighten or differentiate thumb vs track slop.
- [ ] **`Scrollable` redraws every drag-frame even without movement.** `scroll.rs:10571` recomputes and `request_redraw`s whether or not `my` changed. Cheap fix: only redraw if `st.y` actually changed.
- [ ] **`Grid::new` uses `cells.remove(0)` in a loop.** `grid.rs:9550` — O(n²). Use `drain(..take)` or chunk a slice.
- [ ] **sRGB compositing in shader.** `ui_shader.wgsl:1922, 1931` does `color * texture` without linearization, but the surface format is sRGB (`graphics.rs:2867`). Visible on tinted glyphs and AA edges against colored backgrounds. Convert to linear before multiplying or use a linear render target.
- [ ] **Texture-handle generation miss returns blue.** `ui_shader.wgsl:1927` returns `vec4(0, 0, 1, 0)`. Alpha=0 hides it under premultiplied compositing but bleeds blue under straight alpha. Return `vec4(0)`.
- [ ] **`Text::layout` uses unwrapped intrinsic for line count.** `text.rs:11425` — `lines` always 1 for any text that fits unwrapped; `Length::Fit` text wraps maximally aggressively as a result. Either document or use a smarter intrinsic.

### P3 — polish

- [ ] Typo: `instancess` in `widget/mod.rs:9837` and `tests/common.rs:12940–12941`.
- [ ] `DEFAULT_MAX_TEXTURES` (`consts.rs`) is a hard cap with no graceful "create another atlas" fallback. Document or fix.
- [ ] `widget/svg.rs` rasterizes via resvg into the same atlas as glyphs — combined with the no-eviction issue, dynamically-sized SVGs will exhaust it. Subsumed by P1 atlas eviction.
- [ ] Resolve the controlled/uncontrolled inconsistency: `Slider` value lives in app state, everything else in `view_state`. Pick one or document both.
- [ ] `consts.rs` TODO list — `// TODO: should add configurability` at `graphics.rs:2863`, `// TODO: maybe return a result` at `graphics.rs:3022, 3129, 3241`.
- [ ] `// TODO: winit can only have 1 target` at `winit.rs:12498`.
- [ ] `// TODO: propagate new_output and output_destroyed` at `sctk/state.rs:8775`.

---

## Features

### P1 — table-stakes for "semi-feature-complete"

- [ ] **Border-radius, borders, shadows, gradients in the shader.** Currently `ui_shader.wgsl` only does flat-color or texture-sample — no rounded corners, no borders (despite `TextColors` declaring `border` / `focus_border` fields that go nowhere), no shadows, no gradients, no analytic AA. The `Primitive` already has spare slots in `data1`/`data2`. Single highest-leverage feature in the library — every widget gets nicer visuals for free.
- [ ] **Text selection.** No shift+arrow, no click-drag, no double-click word, no triple-click line. Without this, the text widget feels broken within seconds.
- [ ] **Clipboard.** Ctrl/Cmd+C/X/V; primary selection on Linux. Pairs with selection.
- [ ] **IME / preedit.** `TextInput` only carries committed `text`; no preedit/composition string. CJK and dead-key composition won't render correctly. Both winit and SCTK (`text-input-v3`) expose this.
- [ ] **Dropdown / Select / Combobox.** Uses your existing overlay layer + a popup positioner. Real apps need this constantly.
- [ ] **Modal / Dialog.** Scrim layer, focus trap, escape-to-dismiss. Once this and dropdown work, the overlay/positioning code is properly exercised.
- [ ] **Tab focus traversal.** Currently `TextField` *unfocuses* on Tab — placeholder, not real behavior. Need `focusable` + `tab_index` on `Node`, focus order built during layout, Tab/Shift+Tab cycling at root.
- [ ] **Keyboard activation of buttons.** Space/Enter when focused. Trivially small once focus traversal is in.
- [ ] **Focus-visible ring.** Visual indicator when a widget is keyboard-focused. Needs the borders work above.
- [ ] **Checkbox, Radio (group), Switch, Tooltip, Tabs, ProgressBar, Spinner.** Mechanical once theme + borders + focus are in. Group as one milestone.

### P2 — meaningful additions

- [ ] **Theme struct.** Palette + spacing + typography + radii, threaded through `LayoutCtx`/`PaintCtx`. Widgets default-read from theme, override per-instance. Do this *before* adding the widget batch above so they can use it. Light/dark variants out of the box.
- [ ] **Animation primitives.** Spring or ease-curve tweens, register-and-tick mechanism, integration with redraw loop. Without this every interaction is instant-snap. Hover fades, popover enter/exit, scroll inertia.
- [ ] **Undo/redo in text input.** Ctrl+Z / Ctrl+Shift+Z. Standard ring buffer per `TextInputViewState`.
- [ ] **Password mode (`is_secret`).** Bullet-glyph rendering in `TextInput`.
- [ ] **Cursor icon changes.** I-beam over text fields, pointer over buttons. Plumb a cursor request through `Context`; let the platform layer set it.
- [ ] **Drag-and-drop, file drop events.** Plumbed through the `Event` enum; widgets opt in.

### P3 — eventually

- [ ] **AccessKit integration.** Build a parallel `accesskit::Node` tree during paint with role/name/bounds/state. Drives screen readers across all three platforms. One-time integration, unblocks "production-ready" claims.
- [ ] **Tree view / expandable list.**
- [ ] **`prelude` module.** Public re-exports are scattered across `model`, `widget`, `event`. A curated prelude saves users from spelunking.
- [ ] **Column-builder helpers.** `Column::new().push(x).push(y)` alongside the existing `el!` macro form.
- [ ] **Multiple winit targets** (subsumed by `winit.rs:12498` TODO).

---

## Suggested execution order

1. P0 bugs (#1–5) as one or two PRs. Library is not trustworthy until these are gone.
2. P1 bugs that don't depend on layout rework: atlas eviction, modifiers field, double-click fix.
3. Border-radius / borders / shadows in the shader. This unblocks visual quality across every widget at once.
4. Theme struct.
5. Text selection + clipboard + IME (the text widget is the most-touched widget in any app).
6. Tab focus traversal + focus ring.
7. Dropdown + Modal + Tooltip.
8. The widget batch (Checkbox, Radio, Switch, Tabs, ProgressBar).
9. Animation.
10. AccessKit.

Steps 1–2 turn the library from "demo" into "I can trust this." Steps 3–7 turn it into "I could plausibly build a real app with this." 8+ is filling out the surface.
