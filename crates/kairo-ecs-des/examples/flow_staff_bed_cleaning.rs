use kairo_ecs_des::{
    FlowError, FlowRuntime, LifecycleRecord, PreemptionStrategy, RequestId, RequestState,
    ResourceId, WorkId, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};

const STAFF_RESOURCE_LABEL: &str = "Staff-A-duty";
const CASE_ID: &str = "q4.synthetic";
const STAFF_ACTOR: &str = "Staff-A";
const NORMAL_LABEL: &str = "normal_staff";
const URGENT_LABEL: &str = "urgent_staff";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    Continuous,
    PausedAtBoundaries,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskPhase {
    NormalIntake,
    UrgentInterruption,
    #[allow(dead_code)]
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

fn append_dispatch(
    flow: &FlowRuntime,
    resource_id: ResourceId,
    dispatch: kairo_ecs_des::FlowDispatch,
    output: &mut WorkflowOutput,
) -> Result<(), FlowError> {
    if let Some(error) = dispatch.error {
        return Err(error);
    }
    output.records.extend(dispatch.records);
    let resource = flow.resource(resource_id)?;
    output.boundaries.push(BoundarySnapshot {
        label: STAFF_RESOURCE_LABEL,
        at: dispatch.at,
        total: resource.total,
        available: resource.available,
        active: resource.active.len(),
        queued: resource.queued.len(),
    });
    if resource.active.len() + resource.available as usize != resource.total as usize {
        return Err(FlowError::InvalidState);
    }
    Ok(())
}

fn collect_one(
    flow: &mut FlowRuntime,
    resource_id: ResourceId,
    output: &mut WorkflowOutput,
    use_run_for: bool,
) -> Result<bool, FlowError> {
    let dispatch = if use_run_for {
        flow.run_for(1)?.dispatches.into_iter().next()
    } else {
        flow.step()?
    };
    if let Some(dispatch) = dispatch {
        append_dispatch(flow, resource_id, dispatch, output)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn drain(
    flow: &mut FlowRuntime,
    resource_id: ResourceId,
    output: &mut WorkflowOutput,
    use_run_for: bool,
) -> Result<(), FlowError> {
    for _ in 0..128 {
        if !collect_one(flow, resource_id, output, use_run_for)? {
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

fn assert_staff_capacity_drained(
    flow: &FlowRuntime,
    resource: kairo_ecs_des::ResourceId,
) -> Result<(), FlowError> {
    let snapshot = flow.resource(resource)?;
    if snapshot.total != 1
        || snapshot.available != 1
        || !snapshot.active.is_empty()
        || !snapshot.queued.is_empty()
    {
        return Err(FlowError::InvalidState);
    }
    Ok(())
}

/// Run the public-API staff interruption workflow in one live FlowRuntime.
pub fn run(mode: RunMode) -> Result<WorkflowOutput, FlowError> {
    let mut flow = FlowRuntime::new();
    let staff_actor = flow.spawn_actor()?;
    let staff_resource = flow.create_resource(1)?;
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
    let dispatched = collect_one(
        &mut flow,
        staff_resource,
        &mut output,
        mode == RunMode::PausedAtBoundaries,
    )?;
    if !dispatched || flow.now() != time(0) {
        return Err(FlowError::InvalidState);
    }
    assert_context(&flow, normal)?;

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

    match mode {
        RunMode::Continuous => {
            drain(&mut flow, staff_resource, &mut output, false)?;
        }
        RunMode::PausedAtBoundaries => {
            if !collect_one(&mut flow, staff_resource, &mut output, true)? || flow.now() != time(3)
            {
                return Err(FlowError::InvalidState);
            }
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

            if !collect_one(&mut flow, staff_resource, &mut output, true)? || flow.now() != time(5)
            {
                return Err(FlowError::InvalidState);
            }
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

            drain(&mut flow, staff_resource, &mut output, true)?;
        }
    }

    assert_staff_capacity_drained(&flow, staff_resource)?;
    output.requests = vec![
        NamedRequest {
            label: NORMAL_LABEL,
            request: normal.request,
        },
        NamedRequest {
            label: URGENT_LABEL,
            request: urgent.request,
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
    ];
    if output.terminal.iter().any(|state| {
        state.request != RequestState::Completed || state.work != Some(WorkState::Completed)
    }) {
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
        println!(
            "t={} {} {:?} priority={} queue={} active={}",
            record.at.ticks(),
            if continuous.requests[0].request == record.request {
                NORMAL_LABEL
            } else {
                URGENT_LABEL
            },
            record.transition,
            record.snapshot.priority_level,
            record.snapshot.queue_len,
            record.snapshot.active_count
        );
    }
    Ok(())
}
