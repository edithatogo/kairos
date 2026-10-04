#[path = "../examples/flow_staff_bed_cleaning.rs"]
mod scenario;

use kairo_ecs_des::{LifecycleTransition as T, RequestState, WorkState};
use kairo_ecs_types::SimDuration;
use scenario::{run, RunMode, TaskPhase};

fn duration(value: u128) -> SimDuration {
    SimDuration::from_ticks(value)
}

fn request(output: &scenario::WorkflowOutput, label: &str) -> kairo_ecs_des::RequestId {
    output
        .requests
        .iter()
        .find(|named| named.label == label)
        .unwrap_or_else(|| panic!("missing request alias {label}"))
        .request
}

fn assert_contiguous_event_ordinals(records: &[kairo_ecs_des::LifecycleRecord]) {
    let mut closed_events = Vec::new();
    let mut previous = None;
    let mut expected_ordinal = 0u32;
    for record in records {
        if previous == Some(record.causal_event_id) {
            assert_eq!(record.transition_ordinal, expected_ordinal);
            expected_ordinal += 1;
        } else {
            if let Some(event) = previous {
                closed_events.push(event);
            }
            assert!(!closed_events.contains(&record.causal_event_id));
            assert_eq!(record.transition_ordinal, 0);
            previous = Some(record.causal_event_id);
            expected_ordinal = 1;
        }
    }
}

#[test]
fn named_staff_lifecycle_keeps_exact_urgent_suspend_timeline() {
    let output = run(RunMode::Continuous).unwrap();
    let normal = request(&output, "normal_staff");
    let urgent = request(&output, "urgent_staff");
    let normal_rows: Vec<_> = output
        .records
        .iter()
        .filter(|row| row.request == normal)
        .collect();
    let urgent_rows: Vec<_> = output
        .records
        .iter()
        .filter(|row| row.request == urgent)
        .collect();

    assert_eq!(
        normal_rows
            .iter()
            .map(|row| (row.at.ticks(), row.transition))
            .collect::<Vec<_>>(),
        vec![
            (0, T::Queued),
            (0, T::Granted),
            (3, T::Preempted),
            (5, T::Resumed),
            (10, T::Completed)
        ]
    );
    assert_eq!(
        urgent_rows
            .iter()
            .map(|row| (row.at.ticks(), row.transition))
            .collect::<Vec<_>>(),
        vec![(3, T::Queued), (3, T::Granted), (5, T::Completed)]
    );
    assert_eq!(normal_rows[0].snapshot.owner, urgent_rows[0].snapshot.owner);
    assert!(normal_rows
        .iter()
        .all(|row| row.snapshot.owner == normal_rows[0].snapshot.owner));
    assert!(urgent_rows
        .iter()
        .all(|row| row.snapshot.owner == normal_rows[0].snapshot.owner));
    assert_eq!(
        normal_rows[0]
            .snapshot
            .progress
            .as_ref()
            .unwrap()
            .original_duration,
        duration(8)
    );
    assert_eq!(
        urgent_rows[0]
            .snapshot
            .progress
            .as_ref()
            .unwrap()
            .original_duration,
        duration(2)
    );
    assert_contiguous_event_ordinals(&output.records);
    assert_eq!(normal_rows[0].snapshot.priority_level, 10);
    assert_eq!(normal_rows[2].snapshot.priority_level, 10);
    assert_eq!(urgent_rows[0].snapshot.priority_level, 2);
    assert_eq!(urgent_rows[1].snapshot.priority_level, 2);

    let suspended = &normal_rows[2].snapshot;
    assert_eq!(
        suspended.strategy,
        Some(kairo_ecs_des::PreemptionStrategy::Suspend)
    );
    let progress = suspended.progress.as_ref().unwrap();
    assert_eq!(progress.useful_elapsed, duration(3));
    assert_eq!(progress.remaining, duration(5));
    assert_eq!(progress.cumulative_busy, duration(3));
    let resumed = normal_rows[3].snapshot.progress.as_ref().unwrap();
    assert_eq!(resumed.remaining, duration(5));
    assert_eq!(
        normal_rows[4]
            .snapshot
            .progress
            .as_ref()
            .unwrap()
            .useful_elapsed,
        duration(8)
    );
    assert_eq!(
        normal_rows[4]
            .snapshot
            .progress
            .as_ref()
            .unwrap()
            .cumulative_busy,
        duration(8)
    );
    assert_eq!(
        urgent_rows[2]
            .snapshot
            .progress
            .as_ref()
            .unwrap()
            .cumulative_busy,
        duration(2)
    );
}

#[test]
fn named_staff_context_and_terminal_values_remain_typed() {
    let output = run(RunMode::Continuous).unwrap();
    let normal = output
        .contexts
        .iter()
        .find(|item| item.label == "normal_staff")
        .unwrap();
    let urgent = output
        .contexts
        .iter()
        .find(|item| item.label == "urgent_staff")
        .unwrap();
    for (item, task, phase) in [
        (normal, "normal_staff", TaskPhase::NormalIntake),
        (urgent, "urgent_staff", TaskPhase::UrgentInterruption),
    ] {
        assert_eq!(item.context.case_id, "q4.synthetic");
        assert_eq!(item.context.actor_label, "Staff-A");
        assert_eq!(item.context.task_label, task);
        assert_eq!(item.context.phase, phase);
    }

    for label in ["normal_staff", "urgent_staff"] {
        let terminal = output
            .terminal
            .iter()
            .find(|item| item.label == label)
            .unwrap();
        assert_eq!(terminal.request, RequestState::Completed);
        assert_eq!(terminal.work, Some(WorkState::Completed));
    }
}

#[test]
fn staff_boundaries_preserve_capacity_and_finish_drained() {
    let output = run(RunMode::Continuous).unwrap();
    assert!(!output.boundaries.is_empty());
    assert!(output.boundaries.iter().all(|boundary| {
        boundary.active <= boundary.total as usize
            && boundary.active + boundary.available as usize == boundary.total as usize
    }));
    let staff_boundaries = output
        .boundaries
        .iter()
        .filter(|boundary| boundary.label == "Staff-A-duty")
        .collect::<Vec<_>>();
    assert!(!staff_boundaries.is_empty());
    assert!(staff_boundaries.iter().all(|boundary| boundary.total == 1));
    for tick in [0, 3, 5, 10] {
        assert!(
            staff_boundaries
                .iter()
                .any(|boundary| boundary.at.ticks() == tick),
            "missing Staff-A-duty boundary at tick {tick}"
        );
    }
    let final_boundary = staff_boundaries
        .iter()
        .find(|boundary| boundary.at.ticks() == 10)
        .unwrap();
    assert_eq!(final_boundary.active, 0);
    assert_eq!(final_boundary.queued, 0);
    assert_eq!(final_boundary.available, 1);
}

#[test]
fn continuous_and_paused_modes_match_raw_lifecycle_records() {
    let continuous = run(RunMode::Continuous).unwrap();
    let paused = run(RunMode::PausedAtBoundaries).unwrap();
    assert_eq!(continuous.records, paused.records);
    assert_eq!(continuous.requests, paused.requests);
    assert_eq!(continuous.contexts, paused.contexts);
    assert_eq!(continuous.terminal, paused.terminal);
    assert_eq!(continuous.boundaries, paused.boundaries);
    assert_eq!(continuous, paused);
}
