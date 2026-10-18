//! Private deterministic staff selection for model adapters.
//!
//! This helper chooses an eligible staff member for a waiting work or walk
//! request. It does not reserve resources, create events, or model cleaning.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DispatchActivity {
    Work,
    Walk,
    Wait,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DispatchRequest {
    pub(crate) id: u64,
    pub(crate) urgency: i32,
    pub(crate) fifo_sequence: u64,
    pub(crate) assigned_zone: String,
    pub(crate) required_skills: BTreeSet<String>,
    pub(crate) activity: DispatchActivity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StaffMember {
    pub(crate) id: u64,
    pub(crate) zone: String,
    pub(crate) skills: BTreeSet<String>,
    pub(crate) available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DispatchSelection {
    pub(crate) request_id: u64,
    pub(crate) staff_id: u64,
    pub(crate) activity: DispatchActivity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DispatchError {
    DuplicateRequestId,
    DuplicateStaffId,
}

impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRequestId => formatter.write_str("duplicate dispatch request id"),
            Self::DuplicateStaffId => formatter.write_str("duplicate staff id"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// Select the highest-priority request that has an eligible staff member.
///
/// Requests order by urgency descending, FIFO sequence ascending, then request
/// id ascending. Available staff must match the assigned zone exactly and have
/// every required skill; the lowest stable staff id breaks eligible ties.
/// Waiting is an explicit non-dispatch activity and never consumes a staff
/// assignment. Exact strings are used; this helper does not normalize labels.
pub(crate) fn select_dispatch(
    requests: &[DispatchRequest],
    staff: &[StaffMember],
) -> Result<Option<DispatchSelection>, DispatchError> {
    let mut request_ids = BTreeSet::new();
    if requests
        .iter()
        .any(|request| !request_ids.insert(request.id))
    {
        return Err(DispatchError::DuplicateRequestId);
    }
    let mut staff_ids = BTreeSet::new();
    if staff.iter().any(|member| !staff_ids.insert(member.id)) {
        return Err(DispatchError::DuplicateStaffId);
    }

    let mut ordered_requests: Vec<_> = requests.iter().collect();
    ordered_requests.sort_by_key(|request| {
        (
            std::cmp::Reverse(request.urgency),
            request.fifo_sequence,
            request.id,
        )
    });

    for request in ordered_requests {
        if request.activity == DispatchActivity::Wait {
            continue;
        }
        if let Some(member) = staff
            .iter()
            .filter(|member| {
                member.available
                    && member.zone == request.assigned_zone
                    && request.required_skills.is_subset(&member.skills)
            })
            .min_by_key(|member| member.id)
        {
            return Ok(Some(DispatchSelection {
                request_id: request.id,
                staff_id: member.id,
                activity: request.activity,
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skills(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn request(id: u64, urgency: i32, fifo_sequence: u64) -> DispatchRequest {
        DispatchRequest {
            id,
            urgency,
            fifo_sequence,
            assigned_zone: "north".to_owned(),
            required_skills: skills(&["triage"]),
            activity: DispatchActivity::Work,
        }
    }

    fn staff(id: u64) -> StaffMember {
        StaffMember {
            id,
            zone: "north".to_owned(),
            skills: skills(&["triage", "assessment"]),
            available: true,
        }
    }

    #[test]
    fn urgency_precedes_fifo_and_input_order() {
        let selected = select_dispatch(
            &[request(9, 2, 0), request(8, 3, 99), request(7, 3, 4)],
            &[staff(20)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.request_id, 7);
        assert_eq!(selected.activity, DispatchActivity::Work);
    }

    #[test]
    fn fifo_then_request_id_are_stable_tie_breakers() {
        let selected = select_dispatch(
            &[request(9, 1, 4), request(8, 1, 4), request(7, 1, 3)],
            &[staff(20)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.request_id, 7);

        let selected = select_dispatch(&[request(9, 1, 4), request(8, 1, 4)], &[staff(20)])
            .unwrap()
            .unwrap();
        assert_eq!(selected.request_id, 8);
    }

    #[test]
    fn eligibility_uses_exact_zone_required_skill_subset_and_availability() {
        let mut wrong_zone = staff(1);
        wrong_zone.zone = "North".to_owned();
        let mut missing_skill = staff(2);
        missing_skill.skills = skills(&["assessment"]);
        let mut unavailable = staff(3);
        unavailable.available = false;
        let selected = select_dispatch(
            &[request(5, 1, 0)],
            &[wrong_zone, missing_skill, unavailable, staff(4)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.staff_id, 4);
    }

    #[test]
    fn lowest_staff_id_breaks_eligible_staff_ties() {
        let selected = select_dispatch(&[request(5, 1, 0)], &[staff(9), staff(2)])
            .unwrap()
            .unwrap();
        assert_eq!(selected.staff_id, 2);
    }

    #[test]
    fn wait_is_distinct_and_does_not_mask_later_work() {
        let mut waiting = request(1, 9, 0);
        waiting.activity = DispatchActivity::Wait;
        let mut walking = request(2, 1, 1);
        walking.activity = DispatchActivity::Walk;
        let selected = select_dispatch(&[waiting, walking], &[staff(3)])
            .unwrap()
            .unwrap();
        assert_eq!(
            selected,
            DispatchSelection {
                request_id: 2,
                staff_id: 3,
                activity: DispatchActivity::Walk,
            }
        );
    }

    #[test]
    fn duplicate_ids_and_no_eligible_staff_fail_closed() {
        assert_eq!(
            select_dispatch(&[request(1, 1, 0), request(1, 2, 1)], &[staff(3)]),
            Err(DispatchError::DuplicateRequestId)
        );
        assert_eq!(
            select_dispatch(&[request(1, 1, 0)], &[staff(3), staff(3)]),
            Err(DispatchError::DuplicateStaffId)
        );
        let mut unavailable = staff(3);
        unavailable.available = false;
        assert_eq!(
            select_dispatch(&[request(1, 1, 0)], &[unavailable]),
            Ok(None)
        );
    }
}
