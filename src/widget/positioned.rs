use crate::widget::prelude::*;

/// A transparent wrapper that patches parent-owned properties onto whatever
/// it wraps.
pub struct Positioned {
    inner: Element,
    placement: Placement,
    cross_self: Option<Align>,
}

impl crate::widget::IntoElement for Positioned {}

impl Positioned {
    pub fn new<E: Into<Element>>(inner: E) -> Self {
        Self {
            inner: inner.into(),
            placement: Placement::Flow,
            cross_self: None,
        }
    }

    /// Take the existing absolute placement, or start from `ABSOLUTE`.
    fn abs(&mut self) -> &mut Placement {
        if !self.placement.is_absolute() {
            self.placement = Placement::ABSOLUTE;
        }
        &mut self.placement
    }

    /// Place out of flow with `anchor` on the parent and `origin` on self.
    pub fn at(mut self, a: Align2, o: Align2) -> Self {
        if let Placement::Absolute { anchor, origin, .. } = self.abs() {
            *anchor = a;
            *origin = o;
        }
        self
    }

    /// `at(a, a)` — the common case. `pinned(Align2::CENTER)` centres.
    pub fn pinned(self, a: Align2) -> Self {
        self.at(a, a)
    }

    /// Shift by this after anchoring and edge pinning.
    pub fn offset(mut self, x: i32, y: i32) -> Self {
        if let Placement::Absolute { offset, .. } = self.abs() {
            *offset = Position::new(x, y);
        }
        self
    }

    /// Pin to specific sides. Both sides of an axis derives the size.
    pub fn edges(mut self, e: Edges) -> Self {
        if let Placement::Absolute { edges, .. } = self.abs() {
            *edges = e;
        }
        self
    }

    /// Override the cross-axis alignment the parent would otherwise apply.
    pub fn cross_self(mut self, a: Align) -> Self {
        self.cross_self = Some(a);
        self
    }

    /// Offer a placement for children that didn't choose one. `Stack` uses
    /// this to default its children to absolute at its `align`.
    pub(crate) fn default_to(&mut self, p: Placement) {
        if !self.placement.is_absolute() {
            self.placement = p;
        }
    }
}

impl Widget for Positioned {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a>) -> Node {
        let mut n = self.inner.as_mut().layout(ctx);
        // Innermost wins
        if self.placement.is_absolute() && !n.placement.is_absolute() {
            n.placement = self.placement;
        }
        if let Some(a) = self.cross_self {
            n.cross_self = Some(a);
        }
        n
    }
    fn key(&self) -> Option<u64> {
        self.inner.as_ref().key()
    }
    fn child_count(&self) -> usize {
        self.inner.as_ref().child_count()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
        self.inner.as_mut().child_mut(i)
    }
    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        self.inner.as_ref().child_env(env, theme)
    }
    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a>, width: i32) -> Option<i32> {
        self.inner.as_mut().min_height_for_width(ctx, width)
    }
    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        self.inner.as_mut().prepare(ctx);
    }
    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        self.inner.as_mut().paint(ctx, out);
    }
    fn paint_overlay(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        self.inner.as_mut().paint_overlay(ctx, out);
    }
    fn handle(&mut self, ctx: &mut EventCtx) {
        self.inner.as_mut().handle(ctx);
    }
    fn handle_after(&mut self, ctx: &mut EventCtx) {
        self.inner.as_mut().handle_after(ctx);
    }
    fn focus_trap(&self) -> bool {
        self.inner.as_ref().focus_trap()
    }
}

/// Parent-owned placement, available on every widget that derives `Widget`.
///
/// Each method wraps in a [`Positioned`]; further calls resolve to
/// `Positioned`'s inherent methods and mutate in place, so
/// `.pinned(TOP_RIGHT).offset(-8, 8)` keeps the anchor it was given.
pub trait Place: Sized {
    fn at(self, anchor: Align2, origin: Align2) -> Positioned
    where
        Self: Into<Element>,
    {
        Positioned::new(self).at(anchor, origin)
    }
    fn pinned(self, a: Align2) -> Positioned
    where
        Self: Into<Element>,
    {
        Positioned::new(self).pinned(a)
    }
    /// Place out of flow at a pixel offset from the anchor. With the default
    /// top-left anchor this is plain pixel positioning — the direct
    /// replacement for the old `Overlay::push(el, x, y)`.
    fn offset(self, x: i32, y: i32) -> Positioned
    where
        Self: Into<Element>,
    {
        Positioned::new(self).offset(x, y)
    }
    fn edges(self, e: Edges) -> Positioned
    where
        Self: Into<Element>,
    {
        Positioned::new(self).edges(e)
    }
    fn cross_self(self, a: Align) -> Positioned
    where
        Self: Into<Element>,
    {
        Positioned::new(self).cross_self(a)
    }
}

/// `Element` doesn't derive `Widget`, so it needs the impl by hand.
impl Place for Element {}

#[cfg(test)]
mod tests {
    use ui::prelude::*;

    #[test]
    fn positioned_shadows_place() {
        // 1. First call resolves to the `Place` trait method because base_element
        //    isn't a `Positioned` yet. This wraps it and sets anchor/origin.
        let p = Rectangle::new(Size::default(), Color::default()).pinned(Align2::CENTER);

        // 2. Second call MUST resolve to `Positioned::offset` (inherent method).
        //    If it incorrectly resolved to `Place::offset` (trait method), it
        //    would wrap `p` in a NEW `Positioned`, and the outermost placement
        //    would ONLY have the offset (losing the anchor/origin at the top level).
        let p = p.offset(15, 25);

        // 3. Verify that BOTH the anchor (from `pinned`) and the offset (from `offset`)
        //    were successfully merged into the single `Placement` struct.
        if let Placement::Absolute {
            anchor,
            origin,
            offset,
            ..
        } = p.placement
        {
            assert_eq!(
                anchor,
                Align2::CENTER,
                "Anchor should be retained from .pinned()"
            );
            assert_eq!(
                origin,
                Align2::CENTER,
                "Origin should be retained from .pinned()"
            );
            assert_eq!(
                offset,
                Position::new(15, 25),
                "Offset should be applied by .offset()"
            );
        } else {
            panic!("Expected placement to be Absolute but it was Flow");
        }
    }
}
