use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::{collections::BTreeMap, sync::Mutex};

use super::Pipeline;

pub type SlotAlloc = extern "C" fn(u64) -> u32;

static NEXT_INDEX: AtomicU32 = AtomicU32::new(1);
static KEYS: Mutex<BTreeMap<u64, u32>> = Mutex::new(BTreeMap::new());

// State encoding:
// 0           = Uninitialized (neither foreign nor local allocator has been locked in)
// 1           = Locked to LOCAL allocator
// ptr (> 1)   = Configured to FOREIGN allocator function pointer
static ALLOCATOR_STATE: AtomicUsize = AtomicUsize::new(0);
const STATE_LOCAL: usize = 1;

/// Configures a foreign slot allocator.
/// Returns `true` if registered successfully, or `false` if an allocation has already occurred.
#[must_use]
pub fn set_slot_alloc(f: SlotAlloc) -> bool {
    let f_ptr = f as usize;

    ALLOCATOR_STATE
        .compare_exchange(0, f_ptr, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

const fn fnv1a(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut hash = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
        i += 1;
    }
    hash
}

extern "C" fn local_alloc(key: u64) -> u32 {
    let mut keys = KEYS.lock().unwrap();
    *keys
        .entry(key)
        .or_insert_with(|| NEXT_INDEX.fetch_add(1, Ordering::Relaxed))
}

pub fn slot_alloc() -> SlotAlloc {
    local_alloc
}

#[cold]
#[inline(never)]
fn assign<P: PipelineSlot>(memo: &AtomicU32) -> u32 {
    let key = fnv1a(core::any::type_name::<P>());
    let candidate = loop {
        match ALLOCATOR_STATE.load(Ordering::Acquire) {
            // Uninitialized: attempt to lock in the LOCAL allocator atomically
            0 => {
                if ALLOCATOR_STATE
                    .compare_exchange(0, STATE_LOCAL, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    break local_alloc(key);
                }
                // If CAS failed, another thread set state to Foreign or Local simultaneously; loop to re-read.
            }
            // State is locked to Local
            STATE_LOCAL => break local_alloc(key),
            // State is set to Foreign pointer
            foreign_ptr => {
                let f: SlotAlloc = unsafe { core::mem::transmute(foreign_ptr) };
                break f(key);
            }
        }
    };
    assert!(candidate != 0, "pipeline index space exhausted");

    // Two threads racing on the same type both take a candidate; the loser's
    // is simply never claimed, leaving one permanent hole in every registry.
    match memo.compare_exchange(0, candidate, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => candidate - 1,
        Err(actual) => actual - 1,
    }
}

/// Do not implement this by hand — derive it using [`#[derive(Pipeline)]`](Pipeline).
/// Every implementation MUST return a distinct `static`; two types sharing one would collide on the
/// same registry slot and silently draw with the wrong pipeline.
#[doc(hidden)]
pub trait PipelineSlot {
    fn slot() -> &'static AtomicU32
    where
        Self: Sized;

    fn as_any(&self) -> &dyn core::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
}

/// Identifies the pipeline that draws an instance.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PipelineId(u32);

impl PipelineId {
    #[inline(always)]
    pub fn of<P: Pipeline>() -> Self {
        let memo = P::slot();
        let v = memo.load(Ordering::Relaxed);
        if v != 0 {
            return Self(v - 1);
        }
        Self(assign::<P>(memo))
    }

    #[inline(always)]
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}
