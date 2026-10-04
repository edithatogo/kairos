use kairo_ecs_des::{
    FlowError, FlowRuntime, LifecycleRecord, PreemptionStrategy, RequestId, RequestState,
    ResourceId, WorkId, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};

const STAFF_RESOURCE_LABEL: &str = "Staff-A-duty";
const BED_RESOURCE_LABEL: &str = "Bed-A";
const CLEANING_RESOURCE_LABEL: &str = "Cleaning";
const CASE_ID: &str = "q4.synthetic";
const STAFF_ACTOR: &str = "Staff-A";
const NORMAL_LABEL: &str = "normal_staff";
const URGENT_LABEL: &str = "urgent_staff";
const PATIENT_A_LABEL: &str = "patient_a_bed";
const PATIENT_B_LABEL: &str = "patient_b_bed";
const CLEANING_LABEL: &str = "cleaning";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    Continuous,
    PausedAtBoundaries,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskPhase {
    NormalIntake,
    UrgentInterruption,
    Cleaning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixtureContext {
    pub case_id: &'static str,
    pub actor_label: &'static str,
    pub task_label: &'static str,
    pub phase: TaskPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedRequest {
    pub label: &'static str,
    pub request: RequestId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedContext {
    pub label: &'static str,
    pub context: FixtureContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalState {
    pub label: &'static str,
    pub request: RequestState,
    pub work: Option<WorkState>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundarySnapshot {
    pub label: &'static str,
    pub at: SimTime,
    pub total: u32,
    pub available: u32,
    pub active: usize,
    pub queued: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowOutput {
    pub records: Vec<LifecycleRecord>,
    pub requests: Vec<NamedRequest>,
    pub contexts: Vec<NamedContext>,
    pub terminal: Vec<TerminalState>,
    pub boundaries: Vec<BoundarySnapshot>,
}

#[derive(Clone, Copy)]
struct StaffWork {
    request: RequestId,
    work: WorkId,
    context: FixtureContext,
}

fn time(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn duration(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn staff_context(task_label: &'static str, phase: TaskPhase) -> FixtureContext {
    FixtureContext {
        case_id: CASE_ID,
        actor_label: STAFF_ACTOR,
        task_label,
        phase,
    }
}

fn cleaning_context() -> FixtureContext {
    FixtureContext {
        case_id: CASE_ID,
        actor_label: "Cleaner-A",
        task_label: CLEANING_LABEL,
        phase: TaskPhase::Cleaning,
    }
}

fn append_dispatch(
    flow: &FlowRuntime,
    resources: &[(ResourceId, &'static str)],
    dispatch: kairo_ecs_des::FlowDispatch,
    output: &mut WorkflowOutput,
) -> Result<(), FlowError> {
    if let Some(error) = dispatch.error {
        return Err(error);
    }
    output.records.extend(dispatch.records);
    for (resource_id, label) in resources {
        let resource = flow.resource(*resource_id)?;
        if resource.active.len() + resource.available as usize != resource.total as usize {
            return Err(FlowError::InvalidState);
        }
        output.boundaries.push(BoundarySnapshot {
            label,
            at: dispatch.at,
            total: resource.total,
            available: resource.available,
            active: resource.active.len(),
            queued: resource.queued.len(),
        });
    }
    Ok(())
}

fn collect_one(
    flow: &mut FlowRuntime,
    resources: &[(ResourceId, &'static str)],
    output: &mut WorkflowOutput,
    use_run_for: bool,
) -> Result<bool, FlowError> {
    let dispatch = if use_run_for {
        flow.run_for(1)?.dispatches.into_iter().next()
    } else {
        flow.step()?
    };
    if let Some(dispatch) = dispatch {
        append_dispatch(flow, resources, dispatch, output)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn collect_required(
    flow: &mut FlowRuntime,
    resources: &[(ResourceId, &'static str)],
    output: &mut WorkflowOutput,
    use_run_for: bool,
    expected_tick: u128,
) -> Result<(), FlowError> {
    if !collect_one(flow, resources, output, use_run_for)? || flow.now() != time(expected_tick) {
        return Err(FlowError::InvalidState);
    }
    Ok(())
}

fn drain(
    flow: &mut FlowRuntime,
    resources: &[(ResourceId, &'static str)],
    output: &mut WorkflowOutput,
    use_run_for: bool,
) -> Result<(), FlowError> {
    for _ in 0..128 {
        if !collect_one(flow, resources, output, use_run_for)? {
            return Ok(());
        }
    }
    Err(FlowError::InvalidState)
}

fn assert_context(flow: &FlowRuntime, work: StaffWork) -> Result<(), FlowError> {
    let retained = flow.work_context::<FixtureContext>(work.work)?;
    if *retained != work.context {
        return Err(FlowError::InvalidState);
    }
    Ok(())
}

fn assert_resources_drained(
    flow: &FlowRuntime,
    resources: &[(ResourceId, &'static str)],
) -> Result<(), FlowError> {
    for (resource_id, _) in resources {
        let snapshot = flow.resource(*resource_id)?;
        if snapshot.available != snapshot.total
            || !snapshot.active.is_empty()
            || !snapshot.queued.is_empty()
        {
            return Err(FlowError::InvalidState);
        }
    }
    Ok(())
}

/// Run the staged public-API staff, bed and cleaning workflow in one live FlowRuntime.
pub fn run(mode: RunMode) -> Result<WorkflowOutput, FlowError> {
    let use_run_for = mode == RunMode::PausedAtBoundaries;
    let mut flow = FlowRuntime::new();
    let staff_actor = flow.spawn_actor()?;
    let patient_a_actor = flow.spawn_actor()?;
    let patient_b_actor = flow.spawn_actor()?;
    let cleaner_actor = flow.spawn_actor()?;
    let staff_resource = flow.create_resource(1)?;
    let bed_resource = flow.create_resource(1)?;
    let cleaning_resource = flow.create_resource(1)?;
    let resources = [
        (staff_resource, STAFF_RESOURCE_LABEL),
        (bed_resource, BED_RESOURCE_LABEL),
        (cleaning_resource, CLEANING_RESOURCE_LABEL),
    ];

    let normal_context = staff_context(NORMAL_LABEL, TaskPhase::NormalIntake);
    let normal_work = flow.create_work(
        staff_actor,
        duration(8),
        "q4.synthetic.staff.normal",
        normal_context,
    )?;
    let normal_request = flow
        .acquire(staff_resource)
        .owner(staff_actor)
        .timed_work(normal_work)
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()?;
    let normal = StaffWork {
        request: normal_request,
        work: normal_work,
        context: normal_context,
    };

    let mut output = WorkflowOutput {
        records: Vec::new(),
        requests: Vec::new(),
        contexts: Vec::new(),
        terminal: Vec::new(),
        boundaries: Vec::new(),
    };
    collect_required(&mut flow, &resources, &mut output, use_run_for, 0)?;
    assert_context(&flow, normal)?;

    let patient_a_at = flow.now();
    let patient_a_request = flow
        .acquire(bed_resource)
        .owner(patient_a_actor)
        .priority(10)
        .at(patient_a_at)
        .submit()?;
    collect_required(&mut flow, &resources, &mut output, use_run_for, 0)?;
    let patient_a_lease = flow
        .request(patient_a_request)?
        .lease
        .ok_or(FlowError::InvalidState)?;
    if flow.request(patient_a_request)?.state != RequestState::Active {
        return Err(FlowError::InvalidState);
    }

    let urgent_context = staff_context(URGENT_LABEL, TaskPhase::UrgentInterruption);
    let urgent_work = flow.create_work(
        staff_actor,
        duration(2),
        "q4.synthetic.staff.urgent",
        urgent_context,
    )?;
    let urgent_request = flow
        .acquire(staff_resource)
        .owner(staff_actor)
        .at(time(3))
        .timed_work(urgent_work)
        .priority(2)
        .can_preempt(true)
        .submit()?;
    let urgent = StaffWork {
        request: urgent_request,
        work: urgent_work,
        context: urgent_context,
    };

    collect_required(&mut flow, &resources, &mut output, use_run_for, 3)?;
    if flow.request(normal.request)?.state != RequestState::Suspended {
        return Err(FlowError::InvalidState);
    }
    assert_context(&flow, normal)?;
    assert_context(&flow, urgent)?;
    let suspended = flow.work_progress(normal.work)?;
    if suspended.state != WorkState::Suspended
        || suspended.useful_elapsed != duration(3)
        || suspended.remaining != duration(5)
        || suspended.cumulative_busy != duration(3)
    {
        return Err(FlowError::InvalidState);
    }
    if flow.resource(bed_resource)?.active.first().copied() != Some(patient_a_lease) {
        return Err(FlowError::InvalidState);
    }

    collect_required(&mut flow, &resources, &mut output, use_run_for, 5)?;
    if flow.request(normal.request)?.state != RequestState::Active
        || flow.request(urgent.request)?.state != RequestState::Completed
    {
        return Err(FlowError::InvalidState);
    }
    assert_context(&flow, normal)?;
    assert_context(&flow, urgent)?;
    let resumed = flow.work_progress(normal.work)?;
    if resumed.state != WorkState::Active || resumed.remaining != duration(5) {
        return Err(FlowError::InvalidState);
    }

    let cleaning_context = cleaning_context();
    let cleaning_work = flow.create_work(
        cleaner_actor,
        duration(2),
        "q4.synthetic.cleaning",
        cleaning_context,
    )?;
    let cleaning_at = flow.now();
    let cleaning_request = flow
        .acquire(cleaning_resource)
        .owner(cleaner_actor)
        .timed_work(cleaning_work)
        .priority(10)
        .at(cleaning_at)
        .submit()?;
    collect_required(&mut flow, &resources, &mut output, use_run_for, 5)?;
    if flow.request(cleaning_request)?.state != RequestState::Active
        || flow.resource(bed_resource)?.active.first().copied() != Some(patient_a_lease)
    {
        return Err(FlowError::InvalidState);
    }
    if *flow.work_context::<FixtureContext>(cleaning_work)? != cleaning_context {
        return Err(FlowError::InvalidState);
    }

    let patient_b_request = flow
        .acquire(bed_resource)
        .owner(patient_b_actor)
        .priority(10)
        .at(time(6))
        .submit()?;
    collect_required(&mut flow, &resources, &mut output, use_run_for, 6)?;
    let bed_at_six = flow.resource(bed_resource)?;
    if flow.request(patient_b_request)?.state != RequestState::Queued
        || bed_at_six.active.first().copied() != Some(patient_a_lease)
        || bed_at_six.available != 0
        || bed_at_six.queued != [patient_b_request]
    {
        return Err(FlowError::InvalidState);
    }

    collect_required(&mut flow, &resources, &mut output, use_run_for, 7)?;
    if flow.request(cleaning_request)?.state != RequestState::Completed
        || flow.work_progress(cleaning_work)?.state != WorkState::Completed
        || *flow.work_context::<FixtureContext>(cleaning_work)? != cleaning_context
    {
        return Err(FlowError::InvalidState);
    }
    let bed_before_release = flow.resource(bed_resource)?;
    if bed_before_release.available != 0
        || bed_before_release.active.first().copied() != Some(patient_a_lease)
        || flow.request(patient_b_request)?.state != RequestState::Queued
    {
        return Err(FlowError::InvalidState);
    }

    flow.release(patient_a_lease, flow.now())?;
    collect_required(&mut flow, &resources, &mut output, use_run_for, 7)?;
    if flow.request(patient_a_request)?.state != RequestState::Released
        || flow.request(patient_b_request)?.state != RequestState::Active
    {
        return Err(FlowError::InvalidState);
    }
    let patient_b_lease = flow
        .request(patient_b_request)?
        .lease
        .ok_or(FlowError::InvalidState)?;
    let bed_after_first_release = flow.resource(bed_resource)?;
    if bed_after_first_release.available != 0
        || bed_after_first_release.active.first().copied() != Some(patient_b_lease)
    {
        return Err(FlowError::InvalidState);
    }

    flow.release(patient_b_lease, flow.now())?;
    collect_required(&mut flow, &resources, &mut output, use_run_for, 7)?;
    if flow.request(patient_b_request)?.state != RequestState::Released {
        return Err(FlowError::InvalidState);
    }
    drain(&mut flow, &resources, &mut output, use_run_for)?;
    assert_resources_drained(&flow, &resources)?;

    output.requests = vec![
        NamedRequest {
            label: NORMAL_LABEL,
            request: normal.request,
        },
        NamedRequest {
            label: URGENT_LABEL,
            request: urgent.request,
        },
        NamedRequest {
            label: PATIENT_A_LABEL,
            request: patient_a_request,
        },
        NamedRequest {
            label: PATIENT_B_LABEL,
            request: patient_b_request,
        },
        NamedRequest {
            label: CLEANING_LABEL,
            request: cleaning_request,
        },
    ];
    output.contexts = vec![
        NamedContext {
            label: NORMAL_LABEL,
            context: *flow.work_context::<FixtureContext>(normal.work)?,
        },
        NamedContext {
            label: URGENT_LABEL,
            context: *flow.work_context::<FixtureContext>(urgent.work)?,
        },
        NamedContext {
            label: CLEANING_LABEL,
            context: *flow.work_context::<FixtureContext>(cleaning_work)?,
        },
    ];
    output.terminal = vec![
        TerminalState {
            label: NORMAL_LABEL,
            request: flow.request(normal.request)?.state,
            work: Some(flow.work_progress(normal.work)?.state),
        },
        TerminalState {
            label: URGENT_LABEL,
            request: flow.request(urgent.request)?.state,
            work: Some(flow.work_progress(urgent.work)?.state),
        },
        TerminalState {
            label: PATIENT_A_LABEL,
            request: flow.request(patient_a_request)?.state,
            work: None,
        },
        TerminalState {
            label: PATIENT_B_LABEL,
            request: flow.request(patient_b_request)?.state,
            work: None,
        },
        TerminalState {
            label: CLEANING_LABEL,
            request: flow.request(cleaning_request)?.state,
            work: Some(flow.work_progress(cleaning_work)?.state),
        },
    ];
    if output.terminal[0].request != RequestState::Completed
        || output.terminal[0].work != Some(WorkState::Completed)
        || output.terminal[1].request != RequestState::Completed
        || output.terminal[1].work != Some(WorkState::Completed)
        || output.terminal[2].request != RequestState::Released
        || output.terminal[2].work.is_some()
        || output.terminal[3].request != RequestState::Released
        || output.terminal[3].work.is_some()
        || output.terminal[4].request != RequestState::Completed
        || output.terminal[4].work != Some(WorkState::Completed)
    {
        return Err(FlowError::InvalidState);
    }
    Ok(output)
}

#[cfg(not(test))]
fn main() -> Result<(), FlowError> {
    let continuous = run(RunMode::Continuous)?;
    let paused = run(RunMode::PausedAtBoundaries)?;
    assert_eq!(continuous, paused);
    for record in &continuous.records {
        let label = continuous
            .requests
            .iter()
            .find(|named| named.request == record.request)
            .map(|named| named.label)
            .unwrap_or("unknown");
        println!(
            "t={} {} {:?} priority={} queue={} active={}",
            record.at.ticks(),
            label,
            record.transition,
            record.snapshot.priority_level,
            record.snapshot.queue_len,
            record.snapshot.active_count
        );
    }
    Ok(())
}
