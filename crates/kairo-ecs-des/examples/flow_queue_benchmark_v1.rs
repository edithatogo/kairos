use kairo_ecs_des::{
    FlowConfig, FlowRuntime, LifecycleTransition, PreemptionStrategy, RequestId, RequestState,
};
use kairo_ecs_types::{SimDuration, SimTime};
use std::env;
use std::num::NonZeroU64;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scenario {
    Fifo,
    Priority,
    Churn,
    Interruptions,
    ManyResources,
}
impl Scenario {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fifo" => Ok(Self::Fifo),
            "priority" => Ok(Self::Priority),
            "churn" => Ok(Self::Churn),
            "interruptions" => Ok(Self::Interruptions),
            "many_resources" => Ok(Self::ManyResources),
            _ => Err(format!("unknown scenario: {value}")),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::Priority => "priority",
            Self::Churn => "churn",
            Self::Interruptions => "interruptions",
            Self::ManyResources => "many_resources",
        }
    }
}
struct Args {
    scenario: Scenario,
    n: usize,
    resources: usize,
    capacity: u32,
    seed: u64,
}
struct Stats {
    events: usize,
    records: usize,
    preemptions: usize,
    resumptions: usize,
    completions: usize,
}
impl Stats {
    fn new() -> Self {
        Self {
            events: 0,
            records: 0,
            preemptions: 0,
            resumptions: 0,
            completions: 0,
        }
    }
    fn include(&mut self, dispatch: &kairo_ecs_des::FlowDispatch) -> Result<(), String> {
        if let Some(error) = dispatch.error {
            return Err(format!("flow dispatch error: {error:?}"));
        }
        self.events += 1;
        self.records += dispatch.records.len();
        for row in &dispatch.records {
            match row.transition {
                LifecycleTransition::Preempted => self.preemptions += 1,
                LifecycleTransition::Resumed => self.resumptions += 1,
                LifecycleTransition::Completed => self.completions += 1,
                _ => {}
            }
        }
        Ok(())
    }
}
fn parse_args() -> Result<Args, String> {
    let mut values = env::args().skip(1);
    let mut scenario = None;
    let mut n = None;
    let mut resources = None;
    let mut capacity = None;
    let mut seed = None;
    while let Some(key) = values.next() {
        let value = values
            .next()
            .ok_or_else(|| format!("missing value for {key}"))?;
        match key.as_str() {
            "--scenario" => scenario = Some(Scenario::parse(&value)?),
            "--n" => n = Some(value.parse().map_err(|_| "invalid --n".to_string())?),
            "--resources" => {
                resources = Some(
                    value
                        .parse()
                        .map_err(|_| "invalid --resources".to_string())?,
                )
            }
            "--capacity" => {
                capacity = Some(
                    value
                        .parse()
                        .map_err(|_| "invalid --capacity".to_string())?,
                )
            }
            "--seed" => seed = Some(value.parse().map_err(|_| "invalid --seed".to_string())?),
            _ => return Err(format!("unknown option: {key}")),
        }
    }
    let args = Args {
        scenario: scenario.ok_or("missing --scenario")?,
        n: n.ok_or("missing --n")?,
        resources: resources.ok_or("missing --resources")?,
        capacity: capacity.ok_or("missing --capacity")?,
        seed: seed.ok_or("missing --seed")?,
    };
    if args.n == 0 || args.resources == 0 || args.capacity == 0 {
        return Err("--n, --resources and --capacity must be positive".to_string());
    }
    if args.scenario == Scenario::ManyResources && args.capacity != 1 {
        return Err("many_resources requires capacity=1".to_string());
    }
    if args.scenario == Scenario::Interruptions && args.resources != 1 {
        return Err("interruptions requires resources=1".to_string());
    }
    Ok(args)
}
fn at(tick: usize) -> SimTime {
    SimTime::from_ticks(tick as u128)
}
fn duration(ticks: u64) -> SimDuration {
    SimDuration::from_ticks(u128::from(ticks))
}
fn runtime() -> FlowRuntime {
    FlowRuntime::with_config(FlowConfig {
        // The 100k cases intentionally submit at one tick. Keep the runtime's
        // guard above the fixed workload so a benchmark never measures a halt.
        max_same_tick_flow_transitions: NonZeroU64::new(10_000_000).unwrap(),
    })
}
fn priority_for(index: usize, seed: u64) -> i32 {
    let mut value = (index as u64)
        .wrapping_add(seed)
        .wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((value ^ (value >> 31)) % 7) as i32
}
fn drain(runtime: &mut FlowRuntime, limit: usize, stats: &mut Stats) -> Result<(), String> {
    for _ in 0..limit {
        let Some(dispatch) = runtime
            .step()
            .map_err(|e| format!("flow step failed: {e:?}"))?
        else {
            break;
        };
        stats.include(&dispatch)?;
    }
    Ok(())
}
fn create_actor(runtime: &mut FlowRuntime) -> Result<kairo_ecs_types::EntityId, String> {
    runtime
        .spawn_actor()
        .map_err(|e| format!("spawn actor failed: {e:?}"))
}
fn submit_manual(
    runtime: &mut FlowRuntime,
    resource: kairo_ecs_des::ResourceId,
    owner: kairo_ecs_types::EntityId,
    priority: i32,
) -> Result<RequestId, String> {
    runtime
        .acquire(resource)
        .owner(owner)
        .priority(priority)
        .submit()
        .map_err(|e| format!("request submission failed: {e:?}"))
}
fn check_occupancy(
    runtime: &FlowRuntime,
    resources: &[kairo_ecs_des::ResourceId],
    expected_active: u32,
    expected_queued: usize,
) -> Result<Vec<(usize, usize)>, String> {
    let mut occupancy = Vec::with_capacity(resources.len());
    let mut active_total = 0usize;
    let mut queued_total = 0usize;
    for resource in resources {
        let snapshot = runtime
            .resource(*resource)
            .map_err(|e| format!("resource snapshot failed: {e:?}"))?;
        if snapshot.active.len() != expected_active as usize {
            return Err(format!(
                "active count mismatch: got {}, expected {expected_active}",
                snapshot.active.len()
            ));
        }
        active_total += snapshot.active.len();
        queued_total += snapshot.queued.len();
        occupancy.push((snapshot.active.len(), snapshot.queued.len()));
    }
    if active_total != expected_active as usize * resources.len() || queued_total != expected_queued
    {
        return Err(format!(
            "occupancy mismatch: active={active_total}, queued={queued_total}, expected active={}, queued={expected_queued}",
            expected_active as usize * resources.len()
        ));
    }
    Ok(occupancy)
}
fn verify_queue_order(
    runtime: &FlowRuntime,
    resource: kairo_ecs_des::ResourceId,
    expected: &[RequestId],
) -> Result<(), String> {
    let snapshot = runtime
        .resource(resource)
        .map_err(|e| format!("resource snapshot failed: {e:?}"))?;
    if snapshot.queued != expected {
        return Err(format!(
            "queue order mismatch: got {:?}, expected {:?}",
            snapshot.queued, expected
        ));
    }
    Ok(())
}
fn request_counts(runtime: &FlowRuntime, requests: &[RequestId]) -> Result<(usize, usize), String> {
    let mut terminal = 0usize;
    for id in requests {
        let request = runtime
            .request(*id)
            .map_err(|e| format!("request readback failed: {e:?}"))?;
        if matches!(
            request.state,
            RequestState::Released
                | RequestState::Cancelled
                | RequestState::TimedOut
                | RequestState::Completed
                | RequestState::Aborted
        ) {
            terminal += 1;
        }
    }
    Ok((requests.len(), terminal))
}
fn validate_scenario_counts(
    args: &Args,
    stats: &Stats,
    retained_requests: usize,
    terminal_requests: usize,
    retained_works: usize,
    work_counts: &[usize; 5],
) -> Result<(), String> {
    let n = args.n;
    let capacity_slots = args.resources * args.capacity as usize;
    let (events, records, requests, terminal, works, expected_work_counts) = match args.scenario {
        Scenario::Churn => (
            capacity_slots + 2 * n,
            n + 2 * capacity_slots + n / 2,
            capacity_slots + n,
            n - n / 2,
            0,
            [0; 5],
        ),
        Scenario::Interruptions => (
            args.capacity as usize + 2 * n,
            2 * args.capacity as usize + 5 * n,
            args.capacity as usize + n,
            n,
            args.capacity as usize + n,
            [0, args.capacity as usize, 0, n, 0],
        ),
        _ => (
            n + capacity_slots,
            n + 2 * capacity_slots,
            n + capacity_slots,
            0,
            0,
            [0; 5],
        ),
    };
    if stats.events != events || stats.records != records {
        return Err(format!(
            "scenario lifecycle mismatch: events={}, expected {events}; records={}, expected {records}",
            stats.events, stats.records
        ));
    }
    if retained_requests != requests || terminal_requests != terminal || retained_works != works {
        return Err(format!(
            "scenario retained-state mismatch: requests={retained_requests}/{requests}, terminal={terminal_requests}/{terminal}, works={retained_works}/{works}"
        ));
    }
    if *work_counts != expected_work_counts {
        return Err(format!(
            "scenario work-state mismatch: got {work_counts:?}, expected {expected_work_counts:?}"
        ));
    }
    let (preemptions, resumptions, completions) = if args.scenario == Scenario::Interruptions {
        (n, n, n)
    } else {
        (0, 0, 0)
    };
    if (stats.preemptions, stats.resumptions, stats.completions)
        != (preemptions, resumptions, completions)
    {
        return Err(format!(
            "scenario interruption mismatch: preemptions/resumptions/completions={}/{}/{}, expected {preemptions}/{resumptions}/{completions}",
            stats.preemptions, stats.resumptions, stats.completions
        ));
    }
    Ok(())
}

fn execute(args: Args) -> Result<(), String> {
    eprintln!("Q52_PHASE=setup");
    let setup_start = Instant::now();
    let mut flow = runtime();
    let resources = (0..args.resources)
        .map(|_| {
            flow.create_resource(args.capacity)
                .map_err(|e| format!("resource creation failed: {e:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut all_requests = Vec::with_capacity(args.n + args.resources * args.capacity as usize);
    let mut queue_requests: Vec<Vec<(RequestId, i32, usize)>> =
        (0..args.resources).map(|_| Vec::new()).collect();
    let mut works = Vec::new();
    let mut stats = Stats::new();
    let interruption = args.scenario == Scenario::Interruptions;

    if interruption {
        for _ in 0..args.capacity {
            let owner = create_actor(&mut flow)?;
            let work = flow
                .create_work(owner, duration(1_000_000_000), "q52-runtime", ())
                .map_err(|e| format!("low work creation failed: {e:?}"))?;
            works.push(work);
            let request = flow
                .acquire(resources[0])
                .owner(owner)
                .at(at(0))
                .priority(100)
                .timed_work(work)
                .preemptible(PreemptionStrategy::Suspend)
                .submit()
                .map_err(|e| format!("low request submission failed: {e:?}"))?;
            all_requests.push(request);
        }
        for arrival in 0..args.n {
            let owner = create_actor(&mut flow)?;
            let work = flow
                .create_work(owner, duration(1), "q52-runtime", ())
                .map_err(|e| format!("urgent work creation failed: {e:?}"))?;
            works.push(work);
            let request = flow
                .acquire(resources[0])
                .owner(owner)
                .at(at(arrival * 2 + 1))
                .priority(0)
                .can_preempt(true)
                .timed_work(work)
                .preemptible(PreemptionStrategy::Suspend)
                .submit()
                .map_err(|e| format!("urgent request submission failed: {e:?}"))?;
            all_requests.push(request);
        }
    } else {
        // Occupy every capacity slot, then submit exactly n waiting requests.
        for resource in &resources {
            for _ in 0..args.capacity {
                let owner = create_actor(&mut flow)?;
                let request = submit_manual(&mut flow, *resource, owner, 0)?;
                all_requests.push(request);
            }
        }
        for index in 0..args.n {
            let resource_index = index % args.resources;
            let owner = create_actor(&mut flow)?;
            let priority = match args.scenario {
                Scenario::Fifo | Scenario::ManyResources => 0,
                Scenario::Priority | Scenario::Churn => priority_for(index, args.seed),
                Scenario::Interruptions => unreachable!(),
            };
            let request = submit_manual(&mut flow, resources[resource_index], owner, priority)?;
            queue_requests[resource_index].push((request, priority, index));
            all_requests.push(request);
        }
    }
    let setup_ns = setup_start.elapsed().as_nanos();
    eprintln!("Q52_PHASE=dispatch");
    let initial_dispatch_limit = args.n + args.resources * args.capacity as usize;
    let dispatch_start = Instant::now();
    if interruption {
        // Each urgent submit has one timed completion. Old suspended-holder
        // completion tokens stay pending in the future and are not drained here.
        drain(&mut flow, args.capacity as usize + 2 * args.n, &mut stats)?;
    } else {
        drain(&mut flow, initial_dispatch_limit, &mut stats)?;
    }
    let initial_dispatch_ns = dispatch_start.elapsed().as_nanos();
    if !interruption {
        let active_slots = args.resources * args.capacity as usize;
        let expected_events = args.n + active_slots;
        let expected_records = args.n + 2 * active_slots;
        if stats.events != expected_events || stats.records != expected_records {
            return Err(format!(
                "initial admission lifecycle mismatch: events={}, expected {expected_events}; records={}, expected {expected_records}",
                stats.events, stats.records
            ));
        }
    }

    let mut occupancy = if interruption {
        let measured = check_occupancy(&flow, &resources, args.capacity, 0)?;
        if stats.preemptions != args.n || stats.resumptions != args.n {
            return Err(format!(
                "interruption oracle mismatch: preemptions={}, resumptions={}, expected={}",
                stats.preemptions, stats.resumptions, args.n
            ));
        }
        measured
    } else {
        let measured = check_occupancy(&flow, &resources, args.capacity, args.n)?;
        for (resource_index, resource) in resources.iter().enumerate() {
            let mut expected = queue_requests[resource_index].clone();
            if matches!(args.scenario, Scenario::Priority | Scenario::Churn) {
                expected.sort_by_key(|(_, priority, input_index)| (*priority, *input_index));
            }
            let ids: Vec<_> = expected.into_iter().map(|(id, _, _)| id).collect();
            verify_queue_order(&flow, *resource, &ids)?;
        }
        measured
    };

    let mut churn_setup_ns = 0u128;
    let mut churn_dispatch_ns = 0u128;
    if args.scenario == Scenario::Churn {
        let rekey_count = args.n / 2;
        let churn_setup_start = Instant::now();
        for (resource_index, rows) in queue_requests.iter().enumerate() {
            for (local_index, (request, _, _)) in rows.iter().enumerate() {
                if local_index
                    < rekey_count / args.resources
                        + usize::from(resource_index < rekey_count % args.resources)
                {
                    flow.reprioritize(*request, -1, flow.now())
                        .map_err(|e| format!("reprioritize failed: {e:?}"))?;
                } else {
                    flow.cancel(*request, flow.now())
                        .map_err(|e| format!("cancel failed: {e:?}"))?;
                }
            }
        }
        churn_setup_ns = churn_setup_start.elapsed().as_nanos();
        let churn_operation_count = args.n;
        let churn_dispatch_start = Instant::now();
        drain(&mut flow, churn_operation_count, &mut stats)?;
        churn_dispatch_ns = churn_dispatch_start.elapsed().as_nanos();
        let retained_waiters = args.n - (args.n - rekey_count);
        occupancy = check_occupancy(&flow, &resources, args.capacity, retained_waiters)?;
        for (resource_index, resource) in resources.iter().enumerate() {
            let mut expected: Vec<_> = queue_requests[resource_index]
                .iter()
                .filter(|(_, _, input_index)| {
                    // The first floor(n/2) global waiter positions are rekeyed.
                    *input_index < rekey_count
                })
                .map(|(request, _, input_index)| (*request, *input_index))
                .collect();
            expected.sort_by_key(|(_, index)| *index);
            verify_queue_order(
                &flow,
                *resource,
                &expected.into_iter().map(|(id, _)| id).collect::<Vec<_>>(),
            )?;
        }
    }
    if interruption && (stats.events != args.capacity as usize + 2 * args.n) {
        return Err(format!(
            "interruption event count mismatch: got {}, expected {}",
            stats.events,
            args.capacity as usize + 2 * args.n
        ));
    }
    let (retained_requests, terminal_requests) = request_counts(&flow, &all_requests)?;
    let mut work_counts = [0usize; 5];
    for work in &works {
        let progress = flow
            .work_progress(*work)
            .map_err(|e| format!("work progress readback failed: {e:?}"))?;
        let slot = match progress.state {
            kairo_ecs_des::WorkState::Pending => 0,
            kairo_ecs_des::WorkState::Active => 1,
            kairo_ecs_des::WorkState::Suspended => 2,
            kairo_ecs_des::WorkState::Completed => 3,
            kairo_ecs_des::WorkState::Aborted
            | kairo_ecs_des::WorkState::Cancelled
            | kairo_ecs_des::WorkState::Released => 4,
        };
        work_counts[slot] += 1;
    }
    validate_scenario_counts(
        &args,
        &stats,
        retained_requests,
        terminal_requests,
        works.len(),
        &work_counts,
    )?;
    let total_dispatch_ns = initial_dispatch_ns + churn_dispatch_ns;
    let seconds = total_dispatch_ns as f64 / 1_000_000_000.0;
    let events_per_second = stats.events as f64 / seconds;
    let waiters_per_second = args.n as f64 / seconds;
    println!(
        "{{\"schema\":1,\"status\":\"ok\",\"timing_scope\":\"FlowRuntime step plus lifecycle classification; staging and commit included; correctness readback excluded\",\"scenario\":\"{}\",\"n\":{},\"resources\":{},\"capacity\":{},\"seed\":{},\"setup_ns\":{},\"dispatch_ns\":{},\"events_per_second\":{:.3},\"waiters_per_second\":{:.3},\"initial_dispatch_ns\":{},\"churn_setup_ns\":{},\"churn_dispatch_ns\":{},\"events_completed\":{},\"lifecycle_records\":{},\"preemptions\":{},\"resumptions\":{},\"completions\":{},\"logical_waiters\":{},\"retained_request_count\":{},\"terminal_request_count\":{},\"retained_work_count\":{},\"work_states\":{{\"pending\":{},\"active\":{},\"suspended\":{},\"completed\":{},\"other_terminal\":{}}},\"occupancy\":[{}]}}",
        args.scenario.name(),
        args.n,
        args.resources,
        args.capacity,
        args.seed,
        setup_ns,
        total_dispatch_ns,
        events_per_second,
        waiters_per_second,
        initial_dispatch_ns,
        churn_setup_ns,
        churn_dispatch_ns,
        stats.events,
        stats.records,
        stats.preemptions,
        stats.resumptions,
        stats.completions,
        args.n,
        retained_requests,
        terminal_requests,
        works.len(),
        work_counts[0],
        work_counts[1],
        work_counts[2],
        work_counts[3],
        work_counts[4],
        occupancy
            .iter()
            .enumerate()
            .map(|(index, (active, queued))| format!("{{\"resource_index\":{index},\"active\":{active},\"queued\":{queued}}}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}
fn main() {
    match parse_args().and_then(execute) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("flow_queue_benchmark_v1: {error}");
            std::process::exit(2);
        }
    }
}
