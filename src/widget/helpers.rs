use crate::{
    Size,
    widget::{Element, Length},
};

pub(in crate::widget) trait SizeField<T> {
    fn get<'a>(&self, size: &'a Size<T>) -> &'a T;
}

pub(in crate::widget) struct Width;
pub(in crate::widget) struct Height;

impl<T> SizeField<T> for Width {
    fn get<'a>(&self, s: &'a Size<T>) -> &'a T {
        &s.width
    }
}

impl<T> SizeField<T> for Height {
    fn get<'a>(&self, s: &'a Size<T>) -> &'a T {
        &s.height
    }
}

#[inline]
pub(in crate::widget) fn equalize_sizes<M>(
    children: &[Element<M>],
    axis: impl SizeField<i32>,
    axis_length: impl SizeField<Length<i32>>,
    inner: i32,
) -> Vec<(usize, i32)> {
    struct Alloc {
        index: usize,
        allocated: i32,
        min: i32,
        max: i32,
        grows: bool,
    }

    let mut allocs: Vec<Alloc> = Vec::with_capacity(children.len());
    let mut remaining = inner;

    for (i, child) in children.iter().enumerate() {
        let layout = child.layout();

        let raw_min = *axis.get(&layout.min);
        let raw_max = *axis.get(&layout.max);
        let grows = matches!(axis_length.get(&layout.size), Length::Grow);

        let (base, eff_min) = match *axis_length.get(&layout.size) {
            Length::Fixed(x) => {
                let b = x.clamp(raw_min, raw_max);
                (b, b)
            }
            Length::Fit => {
                let b = (*axis.get(&layout.current_size)).clamp(raw_min, raw_max);
                (b, raw_min)
            }
            Length::Grow => (raw_min, raw_min),
        };

        allocs.push(Alloc {
            index: i,
            allocated: base,
            min: eff_min,
            max: raw_max,
            grows,
        });

        remaining -= base;
    }

    // Distribute a budget across (index, cap) pairs as evenly as possible without exceeding caps.
    let bounded_equal_fill = |caps: Vec<(usize, i32)>, budget: i32| -> (i32, Vec<(usize, i32)>) {
        let n = caps.len();
        if n == 0 || budget <= 0 {
            return (0, caps.into_iter().map(|(i, _)| (i, 0)).collect());
        }

        let mut sorted_indices: Vec<usize> = (0..n).collect();
        sorted_indices.sort_by_key(|&j| caps[j].1);

        let mut assigned = vec![0i32; n];

        let mut used: i64 = 0;
        let mut prev_cap: i32 = 0;
        let mut base: i32 = 0;

        let finalize_output = |assigned_caps_index: Vec<i32>,
                               caps_ref: &[(usize, i32)],
                               used64: i64|
         -> (i32, Vec<(usize, i32)>) {
            let used_i32 = used64.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            let result = assigned_caps_index
                .into_iter()
                .enumerate()
                .map(|(k, amt)| (caps_ref[k].0, amt))
                .collect::<Vec<_>>();
            (used_i32, result)
        };

        for (pos, &idx) in sorted_indices.iter().enumerate() {
            let cap_at_idx = caps[idx].1;
            let remaining_n = (n - pos) as i64;

            let delta = cap_at_idx - prev_cap;
            let required = (delta as i64) * remaining_n;

            if delta > 0 && (used + required) <= (budget as i64) {
                base += delta;
                used += required;
                prev_cap = cap_at_idx;
                continue;
            }

            let remaining_budget = (budget as i64) - used;
            if remaining_budget > 0 {
                let share = (remaining_budget / remaining_n) as i32;
                let remainder = (remaining_budget % remaining_n) as usize;

                base += share;

                for &j_done in &sorted_indices[..pos] {
                    assigned[j_done] = caps[j_done].1;
                }
                for &j_pending in &sorted_indices[pos..] {
                    assigned[j_pending] = base.min(caps[j_pending].1);
                }
                let mut to_dist = remainder;
                for &j_pending in sorted_indices[pos..].iter().rev() {
                    if to_dist == 0 {
                        break;
                    }
                    if assigned[j_pending] < caps[j_pending].1 {
                        assigned[j_pending] += 1;
                        to_dist -= 1;
                    }
                }

                used = budget as i64;
            }

            return finalize_output(assigned, &caps, used);
        }

        for &j in &sorted_indices {
            assigned[j] = caps[j].1;
        }
        finalize_output(assigned, &caps, used)
    };

    // Not enough space: take back from items above their minimums, as evenly as possible.
    if remaining < 0 {
        let deficit = -remaining;
        let caps = allocs
            .iter()
            .enumerate()
            .map(|(i, a)| (i, (a.allocated - a.min).max(0)))
            .filter(|&(_, cap)| cap > 0)
            .collect::<Vec<_>>();

        let (used, assigned) = bounded_equal_fill(caps, deficit);

        for (i, take) in assigned {
            if take > 0 {
                allocs[i].allocated -= take;
            }
        }
        remaining += used;
    }

    // Extra space: first level growable items up to the current max level, then grow within max.
    if remaining > 0 {
        let grower_idxs: Vec<_> = (0..allocs.len()).filter(|&i| allocs[i].grows).collect();

        if !grower_idxs.is_empty() {
            let target = grower_idxs
                .iter()
                .map(|&i| allocs[i].allocated)
                .max()
                .unwrap();

            // Level up growable items that are below the target level (respecting their max).
            let level_caps: Vec<_> = grower_idxs
                .iter()
                .filter_map(|&i| {
                    if allocs[i].allocated < target && allocs[i].allocated < allocs[i].max {
                        let cap = (target.min(allocs[i].max)) - allocs[i].allocated;
                        if cap > 0 { Some((i, cap)) } else { None }
                    } else {
                        None
                    }
                })
                .collect();

            if !level_caps.is_empty() && remaining > 0 {
                let (used, assigned) = bounded_equal_fill(level_caps, remaining);
                for (i, add) in assigned {
                    if add > 0 {
                        allocs[i].allocated += add;
                    }
                }
                remaining -= used;
            }

            // If space remains, grow all growables up to their max.
            if remaining > 0 {
                let grow_caps = grower_idxs
                    .iter()
                    .filter_map(|&i| {
                        let cap = allocs[i].max - allocs[i].allocated;
                        if cap > 0 { Some((i, cap)) } else { None }
                    })
                    .collect::<Vec<_>>();

                if !grow_caps.is_empty() {
                    let (_, assigned) = bounded_equal_fill(grow_caps, remaining);
                    for (i, add) in assigned {
                        if add > 0 {
                            allocs[i].allocated += add;
                        }
                    }
                }
            }
        }
    }

    allocs.into_iter().map(|a| (a.index, a.allocated)).collect()
}
