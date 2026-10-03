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

#[cfg(test)]
mod tests {
    use super::{select_replacement, select_victim, HolderCandidate, WaitingCandidate};
    use kairo_ecs_types::{SimDuration, SimTime};

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
}
