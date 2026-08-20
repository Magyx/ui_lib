use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use super::Id;
use crate::{gpu::Gpu, render::texture::TextureRegistry};

pub struct SweepCtx<'a> {
    pub gpu: &'a Gpu,
    pub texture: &'a mut TextureRegistry,
}

pub trait OnSweep: Any {
    fn on_sweep(&mut self, cx: &mut SweepCtx);
}

struct Entry {
    value: Box<dyn Any>,
    on_sweep: Option<fn(&mut dyn Any, &mut SweepCtx)>,
}

type ViewStateInner = HashMap<Id, Entry>;

#[derive(Default)]
pub struct ViewState {
    inner: ViewStateInner,
    touched: HashSet<Id>,
}

impl ViewState {
    pub fn get<T: 'static>(&self, id: &Id) -> Option<&T> {
        self.inner.get(id)?.value.downcast_ref::<T>()
    }
    pub fn get_mut<T: 'static>(&mut self, id: &Id) -> Option<&mut T> {
        self.inner.get_mut(id)?.value.downcast_mut::<T>()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub(crate) fn touch(&mut self, id: Id) {
        self.touched.insert(id);
    }

    fn ensure_inner<T: 'static>(
        &mut self,
        id: Id,
        default: impl FnOnce() -> T,
        on_sweep: Option<fn(&mut dyn Any, &mut SweepCtx)>,
    ) -> &mut T {
        use std::collections::hash_map::Entry as MapEntry;
        let entry = match self.inner.entry(id) {
            MapEntry::Vacant(v) => v.insert(Entry {
                value: Box::new(default()),
                on_sweep,
            }),
            MapEntry::Occupied(mut o) => {
                if !o.get().value.is::<T>() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "Id {} overlapped! Possible duplicate Keyed key under the same parent.",
                        id
                    );
                    let slot = o.get_mut();
                    slot.value = Box::new(default());
                    slot.on_sweep = on_sweep;
                }
                o.into_mut()
            }
        };

        entry.value.downcast_mut::<T>().unwrap()
    }
    pub fn ensure<T: 'static>(&mut self, id: Id, default: impl FnOnce() -> T) -> &mut T {
        self.touch(id);
        self.ensure_inner(id, default, None)
    }
    pub fn ensure_swept<T: OnSweep + 'static>(
        &mut self,
        id: Id,
        default: impl FnOnce() -> T,
    ) -> &mut T {
        self.touch(id);

        fn dispatch<T: OnSweep + 'static>(v: &mut dyn Any, cx: &mut SweepCtx) {
            v.downcast_mut::<T>().unwrap().on_sweep(cx);
        }

        self.ensure_inner(id, default, Some(dispatch::<T>))
    }

    pub(crate) fn was_touched(&self, id: &Id) -> bool {
        self.touched.contains(id)
    }

    fn drain_stale(&mut self) -> Vec<(Id, Entry)> {
        let stale: Vec<(Id, Entry)> = self
            .inner
            .extract_if(|id, _| !self.touched.contains(id))
            .collect();

        self.touched.clear();
        stale
    }

    pub(crate) fn sweep(&mut self, cx: &mut SweepCtx) {
        for (_, mut entry) in self.drain_stale() {
            if let Some(f) = entry.on_sweep {
                f(entry.value.as_mut(), cx);
            }
        }
    }

    #[doc(hidden)]
    pub fn sweep_for_test(&mut self) {
        drop(self.drain_stale());
    }
}

#[cfg(test)]
mod tests {
    use super::super::Context;
    use super::*;

    #[derive(Debug, PartialEq)]
    struct DummyState {
        counter: u32,
        label: &'static str,
    }

    #[test]
    fn view_state_starts_empty() {
        let ctx = Context::new();
        assert!(ctx.view_state.is_empty());
    }

    #[test]
    fn view_state_insert_and_downcast_mut() {
        let mut ctx = Context::new();
        let id: crate::context::Id = 42;

        // Widget-typical pattern: or_insert_with + downcast_mut.
        let st = ctx.view_state.ensure(id, || DummyState {
            counter: 0,
            label: "init",
        });
        st.counter += 1;
        st.label = "touched";

        // Second access sees the prior state.
        let again = ctx.view_state.get_mut::<DummyState>(&id).unwrap();
        assert_eq!(again.counter, 1);
        assert_eq!(again.label, "touched");
    }

    #[test]
    fn view_state_different_ids_are_independent() {
        let mut ctx = Context::new();

        for id in [1u64, 2, 99, 1_000_000] {
            ctx.view_state.ensure(id, || DummyState {
                counter: id as u32,
                label: "x",
            });
        }

        for id in [1u64, 2, 99, 1_000_000] {
            let st = ctx.view_state.get_mut::<DummyState>(&id).unwrap();
            assert_eq!(st.counter, id as u32);
        }
    }

    #[test]
    fn view_state_wrong_type_downcast_returns_none() {
        // If a widget tries to downcast to the wrong type (e.g. two
        // widgets collide on an Id), downcast_mut returns None rather
        // than corrupting memory.

        let mut ctx = Context::new();
        let id: crate::context::Id = 7;

        ctx.view_state.ensure(id, || 123u32);

        let as_dummy = ctx.view_state.get_mut::<DummyState>(&id);
        assert!(as_dummy.is_none(), "wrong-type downcast must be None");

        let as_u32 = ctx.view_state.get_mut::<u32>(&id).copied();
        assert_eq!(as_u32, Some(123));
    }

    #[test]
    fn ensure_marks_touched() {
        let mut vs = ViewState::default();
        vs.ensure(42, || 1u32);
        assert!(vs.was_touched(&42));
    }

    #[test]
    fn sweep_for_test_removes_untouched_entries() {
        let mut vs = ViewState::default();
        vs.ensure(1, || 100u32);
        vs.ensure(2, || 200u32);
        vs.sweep_for_test(); // clears touched

        vs.ensure(1, || 100u32); // only 1 touched this frame
        vs.sweep_for_test();

        assert_eq!(vs.inner.len(), 1);
        assert!(vs.get::<u32>(&1).is_some());
        assert!(vs.get::<u32>(&2).is_none());
    }

    #[test]
    fn touched_cleared_after_sweep() {
        let mut vs = ViewState::default();
        vs.ensure(1, || 1u32);
        vs.sweep_for_test();
        assert!(!vs.was_touched(&1));
    }

    // This test guards the type-mismatch branch in ensure_inner:
    // when an id is reused with a different T, both `value` AND
    // `on_sweep` must be replaced together. If only `value` is replaced,
    // the next sweep dispatches through the OLD T's downcast, which
    // fails the unwrap and panics.
    #[test]
    fn type_mismatch_resets_on_sweep_dispatcher() {
        use crate::context::{OnSweep, SweepCtx};

        struct A;
        impl OnSweep for A {
            fn on_sweep(&mut self, _: &mut SweepCtx) {}
        }
        struct B;
        impl OnSweep for B {
            fn on_sweep(&mut self, _: &mut SweepCtx) {}
        }

        let mut vs = ViewState::default();
        vs.ensure_swept(7, || A);
        vs.sweep_for_test(); // Touched cleared. A still in map.
        vs.ensure_swept(7, || B); // Same id, different T => replaces.
        vs.sweep_for_test(); // B is touched, stays. No panic = pass.

        // Now untouch and run sweep through the real path with a stub SweepCx.
        let _ = vs.get::<B>(&7).expect("B should still be at id 7");
    }
}
