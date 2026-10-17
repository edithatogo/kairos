use kairo_ecs_types::{SimDuration, SimTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HolderCandidate<Id> {
    pub(crate) id: Id,
    pub(crate) priority_level: i32,
    pub(crate) original_admission_sequence: u64,
    pub(crate) remaining: SimDuration,
    pub(crate) completion_at: Option<SimTime>,
    pub(crate) timed: bool,
    pub(crate) preemptible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WaitingCandidate<Id> {
    pub(crate) id: Id,
    pub(crate) priority_level: i32,
    pub(crate) original_admission_sequence: u64,
    pub(crate) can_preempt: bool,
}

#[cfg(test)]
pub(crate) fn select_victim<Id: Copy + Ord>(
    incoming_priority: i32,
    incoming_can_preempt: bool,
    now: SimTime,
    holders: &[HolderCandidate<Id>],
) -> Option<Id> {
    if !incoming_can_preempt {
        return None;
    }

    holders
        .iter()
        .filter(|holder| {
            holder.timed
                && holder.preemptible
                && holder.priority_level > incoming_priority
                && holder.remaining > SimDuration::ZERO
                && holder
                    .completion_at
                    .is_some_and(|completion_at| completion_at > now)
        })
        .max_by_key(|holder| {
            (
                holder.priority_level,
                holder.original_admission_sequence,
                holder.id,
            )
        })
        .map(|holder| holder.id)
}

#[cfg(test)]
pub(crate) fn select_replacement<Id: Copy + Ord>(
    now: SimTime,
    waiters: &[WaitingCandidate<Id>],
    holders: &[HolderCandidate<Id>],
) -> Option<(Id, Id)> {
    let mut ordered_waiters: Vec<_> = waiters.iter().collect();
    ordered_waiters.sort_by_key(|waiter| {
        (
            waiter.priority_level,
            waiter.original_admission_sequence,
            waiter.id,
        )
    });

    ordered_waiters.into_iter().find_map(|waiter| {
        select_victim(waiter.priority_level, waiter.can_preempt, now, holders)
            .map(|victim| (waiter.id, victim))
    })
}

/// Selects a replacement from an already ordered waiter stream without
/// allocating a waiter projection or sorting it again.
///
/// `ordered_waiters` must be ordered by
/// `(priority_level, original_admission_sequence, id)`, as the production
/// queue is. The globally worst eligible holder is the only victim that can
/// satisfy any incoming priority; if the stream reaches that priority, no
/// later waiter can replace a holder.
pub(crate) fn select_ordered_replacement<Id, I>(
    now: SimTime,
    ordered_waiters: I,
    holders: &[HolderCandidate<Id>],
) -> Option<(Id, Id)>
where
    Id: Copy + Ord,
    I: IntoIterator<Item = WaitingCandidate<Id>>,
{
    let worst_holder = holders
        .iter()
        .filter(|holder| {
            holder.timed
                && holder.preemptible
                && holder.remaining > SimDuration::ZERO
                && holder
                    .completion_at
                    .is_some_and(|completion_at| completion_at > now)
        })
        .max_by_key(|holder| {
            (
                holder.priority_level,
                holder.original_admission_sequence,
                holder.id,
            )
        })?;

    for waiter in ordered_waiters {
        if waiter.priority_level >= worst_holder.priority_level {
            return None;
        }
        if waiter.can_preempt {
            return Some((waiter.id, worst_holder.id));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        select_ordered_replacement, select_replacement, select_victim, HolderCandidate,
        WaitingCandidate,
    };
    use kairo_ecs_types::{SimDuration, SimTime};
    use std::cell::Cell;

    fn at(ticks: u128) -> SimTime {
        SimTime::from_ticks(ticks)
    }

    fn duration(ticks: u128) -> SimDuration {
        SimDuration::from_ticks(ticks)
    }

    fn holder(
        id: u64,
        priority_level: i32,
        original_admission_sequence: u64,
    ) -> HolderCandidate<u64> {
        HolderCandidate {
            id,
            priority_level,
            original_admission_sequence,
            remaining: duration(5),
            completion_at: Some(at(10)),
            timed: true,
            preemptible: true,
        }
    }

    fn waiter(
        id: u64,
        priority_level: i32,
        original_admission_sequence: u64,
        can_preempt: bool,
    ) -> WaitingCandidate<u64> {
        WaitingCandidate {
            id,
            priority_level,
            original_admission_sequence,
            can_preempt,
        }
    }

    #[test]
    fn requires_preempting_incoming_claim_and_strictly_worse_holder() {
        let holders = [holder(1, 9, 1), holder(2, 4, 2)];

        assert_eq!(select_victim(2, false, at(0), &holders), None);
        assert_eq!(select_victim(9, true, at(0), &holders), None);
        assert_eq!(select_victim(10, true, at(0), &holders), None);
        assert_eq!(select_victim(8, true, at(0), &holders), Some(1));
    }

    #[test]
    fn excludes_untimed_nonpreemptible_zero_due_and_missing_completion_holders() {
        let mut untimed = holder(1, 9, 1);
        untimed.timed = false;
        let mut nonpreemptible = holder(2, 9, 2);
        nonpreemptible.preemptible = false;
        let mut zero_remaining = holder(3, 9, 3);
        zero_remaining.remaining = SimDuration::ZERO;
        let mut due = holder(4, 9, 4);
        due.completion_at = Some(at(5));
        let mut missing_completion = holder(5, 9, 5);
        missing_completion.completion_at = None;
        let future = holder(6, 9, 6);
        let holders = [
            untimed,
            nonpreemptible,
            zero_remaining,
            due,
            missing_completion,
            future,
        ];

        assert_eq!(select_victim(1, true, at(5), &holders), Some(6));
    }

    #[test]
    fn chooses_worst_priority_then_latest_original_admission_then_largest_id() {
        let holders = [
            holder(90, 8, 8),
            holder(20, 9, 5),
            holder(30, 9, 7),
            holder(40, 9, 7),
        ];

        assert_eq!(select_victim(1, true, at(0), &holders), Some(40));
    }

    #[test]
    fn holder_iteration_permutations_select_the_same_victim() {
        let first = [holder(8, 8, 10), holder(4, 9, 2), holder(7, 9, 5)];
        let second = [holder(7, 9, 5), holder(8, 8, 10), holder(4, 9, 2)];
        let third = [holder(4, 9, 2), holder(7, 9, 5), holder(8, 8, 10)];

        assert_eq!(select_victim(1, true, at(0), &first), Some(7));
        assert_eq!(select_victim(1, true, at(0), &second), Some(7));
        assert_eq!(select_victim(1, true, at(0), &third), Some(7));
    }

    #[test]
    fn scans_past_earlier_nonpreempting_waiter_and_sorts_waiter_order() {
        let waiters = [waiter(30, 2, 2, true), waiter(10, 1, 1, false)];
        let holders = [holder(90, 9, 1)];

        assert_eq!(
            select_replacement(at(0), &waiters, &holders),
            Some((30, 90))
        );
    }

    #[test]
    fn each_replacement_reselects_from_the_remaining_holder_projection() {
        let waiters = [waiter(10, 2, 1, true)];
        let holders = [holder(90, 9, 1), holder(80, 8, 2)];

        assert_eq!(
            select_replacement(at(0), &waiters, &holders),
            Some((10, 90))
        );
        let remaining_holders = [holders[1]];
        assert_eq!(
            select_replacement(at(0), &waiters, &remaining_holders),
            Some((10, 80))
        );
    }

    #[test]
    fn priority_extremes_do_not_require_negation_or_overflow() {
        let holders = [holder(1, i32::MAX, 1), holder(2, i32::MIN, 2)];

        assert_eq!(select_victim(i32::MIN, true, at(0), &holders), Some(1));
        assert_eq!(select_victim(i32::MAX, true, at(0), &holders), None);
    }

    #[test]
    fn replacement_search_returns_none_when_no_waiter_has_a_victim() {
        let waiters = [waiter(10, 2, 1, false), waiter(20, 3, 2, true)];
        let holders = [holder(90, 3, 1)];

        assert_eq!(select_replacement(at(0), &waiters, &holders), None);
    }

    #[test]
    fn ordered_selector_matches_sorting_reference_over_deterministic_grid() {
        let mut waiters = [
            waiter(5, 0, 1, true),
            waiter(2, 0, 1, true),
            waiter(9, i32::MIN, 4, false),
            waiter(7, i32::MAX, 0, true),
            waiter(1, -1, 3, true),
            waiter(3, 0, 1, false),
        ];
        let mut holders = [
            holder(100, i32::MAX, 9),
            holder(90, i32::MAX, 9),
            holder(80, i32::MIN, 0),
            holder(70, i32::MAX, 8),
            holder(60, i32::MAX, 7),
            holder(50, i32::MAX, 6),
            holder(40, i32::MAX, 5),
        ];
        holders[2].timed = false;
        holders[3].preemptible = false;
        holders[4].remaining = SimDuration::ZERO;
        holders[5].completion_at = Some(at(5));
        holders[6].completion_at = None;

        for variant in 0..6 {
            for (index, waiter) in waiters.iter_mut().enumerate() {
                waiter.can_preempt = (index + variant) % 3 != 0;
                waiter.priority_level = match (index + variant) % 6 {
                    0 => i32::MIN,
                    1 => -1,
                    2 | 3 => 0,
                    4 => 1,
                    _ => i32::MAX,
                };
                waiter.original_admission_sequence = ((index * 5 + variant) % 4) as u64;
            }
            for rotation in 0..holders.len() {
                holders.rotate_left(1);
                if rotation % 2 == 1 {
                    holders.reverse();
                }
                for incoming_can_preempt in [false, true] {
                    let mut input = waiters;
                    for waiter in &mut input {
                        waiter.can_preempt &= incoming_can_preempt;
                    }
                    let expected = select_replacement(at(5), &input, &holders);
                    input.sort_by_key(|candidate| {
                        (
                            candidate.priority_level,
                            candidate.original_admission_sequence,
                            candidate.id,
                        )
                    });
                    assert_eq!(
                        select_ordered_replacement(at(5), input, &holders),
                        expected,
                        "variant={variant}, rotation={rotation}, incoming={incoming_can_preempt}"
                    );
                }
            }
        }
    }

    #[test]
    fn ordered_selector_does_not_consume_waiters_without_eligible_holders() {
        let mut holders = [holder(9, 99, 1)];
        holders[0].timed = false;
        let waiters = std::iter::from_fn(|| -> Option<WaitingCandidate<u64>> {
            panic!("waiter stream must remain untouched")
        });

        assert_eq!(select_ordered_replacement(at(0), waiters, &holders), None);
    }

    #[test]
    fn ordered_selector_stops_at_equal_or_worse_priority_cutoff() {
        let holders = [holder(9, 4, 10)];
        for level in [4, 5] {
            let yielded = Cell::new(0usize);
            let waiters = [waiter(1, level, 0, true), waiter(2, level + 1, 1, true)];
            let ordered = waiters
                .into_iter()
                .inspect(|_| yielded.set(yielded.get() + 1));
            assert_eq!(select_ordered_replacement(at(0), ordered, &holders), None);
            assert_eq!(yielded.get(), 1);
        }
    }

    #[test]
    fn ordered_selector_keeps_first_eligible_waiter_tie_and_worst_holder_tie() {
        let holders = [holder(3, 9, 8), holder(8, 9, 8), holder(7, 9, 8)];
        let waiters = [
            waiter(4, 2, 1, true),
            waiter(6, 2, 1, false),
            waiter(2, 2, 1, true),
            waiter(1, 3, 0, true),
        ];
        let mut ordered = waiters;
        ordered.sort_by_key(|candidate| {
            (
                candidate.priority_level,
                candidate.original_admission_sequence,
                candidate.id,
            )
        });

        assert_eq!(
            select_ordered_replacement(at(0), ordered, &holders),
            Some((2, 8))
        );
        assert_eq!(select_ordered_replacement(at(0), [], &holders), None);
        let all_nonpreempting = [waiter(1, 1, 0, false), waiter(2, 2, 1, false)];
        assert_eq!(
            select_ordered_replacement(at(0), all_nonpreempting, &holders),
            None
        );
    }
}
