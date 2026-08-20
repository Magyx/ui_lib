use crate::{context::Id, tree::ROOT_SEED};

/// A focus scope is identified by the path-hash [`Id`] of the widget that
/// introduces it. Scopes are therefore stable across frames for free. The root
/// scope is [`ROOT_SEED`].
pub type ScopeId = Id;

/// Traversal direction for a keyboard focus move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Next,
    Prev,
}

/// A pending focus mutation, resolved at the end of the event walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusRequest {
    /// Focus a specific widget (e.g. a click landed on it).
    Set(Id),
    /// Advance/retreat keyboard focus within the active scope.
    Move(Dir),
    /// Drop keyboard focus entirely.
    Clear,
}

#[derive(Clone, Copy)]
struct Focusable {
    id: Id,
    scope: ScopeId,
}

/// Owner of focus/hover/press policy. Lives on [`Context`](crate::context::Context).
pub struct Focus {
    hovered: Option<Id>,
    pressed: Option<Id>,
    focused: Option<Id>,

    /// The trapping scope with the highest z-order (else root). Used to gate
    /// pointer input and to filter keyboard traversal. Finalized at the end of
    /// each walk; reads *during* a walk see the previous walk's value (a stable
    /// one-walk carryover, the usual immediate-mode trick).
    active_scope: ScopeId,
    /// Accumulates the topmost trap seen during the in-progress walk.
    active_scope_next: ScopeId,

    /// Rebuilt each walk, in tree-visit (== tab) order.
    ring: Vec<Focusable>,
    /// Resolved at end of walk.
    request: Option<FocusRequest>,
}

impl Default for Focus {
    fn default() -> Self {
        Self::new()
    }
}

impl Focus {
    pub(crate) fn new() -> Self {
        Self {
            hovered: None,
            pressed: None,
            focused: None,
            active_scope: ROOT_SEED,
            active_scope_next: ROOT_SEED,
            ring: Vec::new(),
            request: None,
        }
    }

    #[inline]
    pub fn is_focused(&self, id: Id) -> bool {
        self.focused == Some(id)
    }
    #[inline]
    pub fn is_hovered(&self, id: Id) -> bool {
        self.hovered == Some(id)
    }
    #[inline]
    pub fn is_pressed(&self, id: Id) -> bool {
        self.pressed == Some(id)
    }
    #[inline]
    pub fn focused(&self) -> Option<Id> {
        self.focused
    }
    #[inline]
    pub fn hovered(&self) -> Option<Id> {
        self.hovered
    }
    #[inline]
    pub fn pressed(&self) -> Option<Id> {
        self.pressed
    }
    #[inline]
    pub fn active_scope(&self) -> ScopeId {
        self.active_scope
    }

    /// Whether pointer input should reach a widget in `scope`. With no active
    /// trap (root), everything is reachable; with a modal open, only widgets in
    /// its scope are — clicks elsewhere land on the backdrop and do nothing.
    #[inline]
    pub fn pointer_available(&self, scope: ScopeId) -> bool {
        self.active_scope == ROOT_SEED || scope == self.active_scope
    }

    /// Central hover assignment. Interactive widgets claim hover when the
    /// pointer is over them and [`pointer_available`](Self::pointer_available);
    /// the last (deepest / topmost) claimer wins.
    #[inline]
    pub(crate) fn claim_hover(&mut self, id: Id) {
        self.hovered = Some(id);
    }
    #[inline]
    pub(crate) fn begin_press(&mut self, id: Id) {
        self.pressed = Some(id);
    }
    #[inline]
    pub(crate) fn end_press(&mut self, id: Id) {
        if self.pressed == Some(id) {
            self.pressed = None;
        }
    }

    #[inline]
    pub(crate) fn request_set(&mut self, id: Id) {
        self.request = Some(FocusRequest::Set(id));
    }
    #[inline]
    pub(crate) fn request_move(&mut self, dir: Dir) {
        self.request = Some(FocusRequest::Move(dir));
    }
    #[inline]
    pub(crate) fn request_clear(&mut self) {
        self.request = Some(FocusRequest::Clear);
    }

    /// Start of an event walk: drop the transient ring + hover, reset the scope
    /// accumulator. `pressed`/`focused` persist across walks (a drag or caret
    /// outlives a single event) and are pruned by [`sweep`](Self::sweep).
    pub(crate) fn begin_walk(&mut self) {
        self.ring.clear();
        self.hovered = None;
        self.active_scope_next = ROOT_SEED;
    }

    /// Register a focusable in visit order. No dedup: the walk visits each node
    /// once per pass, and the ring is cleared at `begin_walk`.
    #[inline]
    pub(crate) fn register(&mut self, id: Id, scope: ScopeId) {
        self.ring.push(Focusable { id, scope });
    }

    /// Note a trapping scope. Last-seen wins, which in document order tracks the
    /// most recently opened (topmost, last-painted) trapping overlay.
    #[inline]
    pub(crate) fn note_trap(&mut self, scope: ScopeId) {
        self.active_scope_next = scope;
    }

    /// End of an event walk: promote the accumulated scope and resolve the
    /// pending request against the freshly built ring.
    pub(crate) fn end_walk(&mut self) {
        self.active_scope = self.active_scope_next;
        if let Some(req) = self.request.take() {
            match req {
                FocusRequest::Set(id) => self.focused = Some(id),
                FocusRequest::Clear => self.focused = None,
                FocusRequest::Move(dir) => self.move_focus(dir),
            }
        }
    }

    fn move_focus(&mut self, dir: Dir) {
        let mut scoped = self.ring.iter().filter(|f| f.scope == self.active_scope);

        self.focused = match dir {
            Dir::Next => {
                if let Some(cur_id) = self.focused {
                    scoped
                        .by_ref()
                        .skip_while(|f| f.id != cur_id)
                        .nth(1)
                        .or_else(|| self.ring.iter().find(|f| f.scope == self.active_scope))
                        .map(|f| f.id)
                } else {
                    scoped.next().map(|f| f.id)
                }
            }
            Dir::Prev => {
                if let Some(cur_id) = self.focused {
                    scoped
                        .by_ref()
                        .rev()
                        .skip_while(|f| f.id != cur_id)
                        .nth(1)
                        .or_else(|| {
                            self.ring
                                .iter()
                                .rev()
                                .find(|f| f.scope == self.active_scope)
                        })
                        .map(|f| f.id)
                } else {
                    scoped.next_back().map(|f| f.id)
                }
            }
        };
    }

    /// Drop focus/press whose widget was not touched this frame (it was removed
    /// or shrank away). `active_scope` self-heals: a closed trap simply isn't
    /// re-noted next walk, so the accumulator falls back to root on its own.
    /// Hover is transient (cleared every `begin_walk`) so it needs no sweep.
    pub(crate) fn sweep(&mut self, was_touched: impl Fn(Id) -> bool) {
        if let Some(id) = self.focused
            && !was_touched(id)
        {
            self.focused = None;
        }
        if let Some(id) = self.pressed
            && !was_touched(id)
        {
            self.pressed = None;
        }
    }

    /// The current tab ring restricted to the active scope, in order. Exposed
    /// for tests and debugging overlays.
    #[doc(hidden)]
    pub fn active_ring(&self) -> Vec<Id> {
        self.ring
            .iter()
            .filter(|f| f.scope == self.active_scope)
            .map(|f| f.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Id = 10;
    const B: Id = 20;
    const C: Id = 30;
    const MODAL: ScopeId = 999;
    const M1: Id = 41;
    const M2: Id = 42;

    fn root_walk(f: &mut Focus, ids: &[Id]) {
        f.begin_walk();
        for &id in ids {
            f.register(id, ROOT_SEED);
        }
        f.end_walk();
    }

    #[test]
    fn tab_cycles_in_visit_order_with_wrap() {
        let mut f = Focus::new();
        f.request_move(Dir::Next);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(A));

        f.request_move(Dir::Next);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(B));

        f.request_move(Dir::Next);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(C));

        f.request_move(Dir::Next);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(A), "wraps around");
    }

    #[test]
    fn shift_tab_goes_backwards() {
        let mut f = Focus::new();
        f.request_set(B);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(B));

        f.request_move(Dir::Prev);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(A));

        f.request_move(Dir::Prev);
        root_walk(&mut f, &[A, B, C]);
        assert_eq!(f.focused(), Some(C), "wraps to last");
    }

    #[test]
    fn keyboard_traversal_is_trapped_to_active_scope() {
        let mut f = Focus::new();
        // A modal is open: a trap scope plus two entries inside it, and A/B in
        // root. Only modal entries are candidates.
        f.begin_walk();
        f.register(A, ROOT_SEED);
        f.note_trap(MODAL);
        f.register(M1, MODAL);
        f.register(M2, MODAL);
        f.register(B, ROOT_SEED);
        f.request_move(Dir::Next);
        f.end_walk();
        assert_eq!(f.active_scope(), MODAL);
        assert_eq!(f.focused(), Some(M1), "enters the trapped scope");

        let step = |f: &mut Focus| {
            f.begin_walk();
            f.register(A, ROOT_SEED);
            f.note_trap(MODAL);
            f.register(M1, MODAL);
            f.register(M2, MODAL);
            f.register(B, ROOT_SEED);
            f.request_move(Dir::Next);
            f.end_walk();
        };
        step(&mut f);
        assert_eq!(f.focused(), Some(M2));
        step(&mut f);
        assert_eq!(f.focused(), Some(M1), "cannot Tab out of the modal");
    }

    #[test]
    fn pointer_gated_by_active_scope() {
        let mut f = Focus::new();
        // No trap: everything reachable.
        assert!(f.pointer_available(ROOT_SEED));
        assert!(f.pointer_available(MODAL));

        // Open a trap.
        f.begin_walk();
        f.note_trap(MODAL);
        f.register(M1, MODAL);
        f.end_walk();
        assert!(f.pointer_available(MODAL));
        assert!(
            !f.pointer_available(ROOT_SEED),
            "clicks behind modal are dead"
        );
    }

    #[test]
    fn sweep_drops_vanished_focus_and_press() {
        let mut f = Focus::new();
        f.request_set(A);
        root_walk(&mut f, &[A, B]);
        f.begin_press(A);
        assert_eq!(f.focused(), Some(A));
        assert!(f.is_pressed(A));

        // A is gone this frame.
        f.sweep(|id| id == B);
        assert_eq!(f.focused(), None);
        assert!(!f.is_pressed(A));
    }

    #[test]
    fn hover_is_transient_across_walks() {
        let mut f = Focus::new();
        f.begin_walk();
        f.claim_hover(A);
        f.end_walk();
        assert!(f.is_hovered(A));

        // Next walk, nobody claims: hover clears.
        f.begin_walk();
        f.end_walk();
        assert!(!f.is_hovered(A));
    }

    #[test]
    fn closed_trap_falls_back_to_root() {
        let mut f = Focus::new();
        f.begin_walk();
        f.note_trap(MODAL);
        f.register(M1, MODAL);
        f.end_walk();
        assert_eq!(f.active_scope(), MODAL);

        // Modal closed: not re-noted.
        f.begin_walk();
        f.register(A, ROOT_SEED);
        f.end_walk();
        assert_eq!(f.active_scope(), ROOT_SEED);
    }
}
