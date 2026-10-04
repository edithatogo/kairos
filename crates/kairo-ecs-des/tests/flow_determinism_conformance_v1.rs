//! Independent-replication determinism for the Q5.1 Flow boundary/strategy fixture.
//!
//! This exercises scheduler/runtime determinism for fixed, test-local scenario
//! inputs. It does not qualify the engine RNG, PDES, shared-runtime concurrency,
//! or portable checkpointing.
use kairo_ecs_des::{
    FlowDispatch, FlowRuntime, LifecycleRecord, LifecycleSnapshot, LifecycleTransition,
    PreemptionStrategy, RequestId, RequestState, ResourceId, ResourceSnapshot, WorkId,
    WorkProgress, WorkState,
};
use kairo_ecs_types::{EntityId, EventId, SimDuration, SimTime};
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration as WallDuration;

const REPLICATION_SEEDS: [u64; 8] = [0, 1, 7, 42, 0xdead_beef, u64::MAX, 0x1234_5678, 0xabcd_ef01];
const MAX_DISPATCHES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Scenario {
    replication_id: u64,
    seed: u64,
    low_duration: u128,
    interruption_at: u128,
    urgent_duration: u128,
}

impl Scenario {
    fn from_seed(replication_id: u64, seed: u64) -> Self {
        let mut random = SplitMix64(seed ^ replication_id.rotate_left(17));
        let low_duration = 12 + random.next() as u128 % 8;
        let interruption_at = 1 + random.next() as u128 % (low_duration - 1);
        let urgent_duration = 1 + random.next() as u128 % 4;
        Self {
            replication_id,
            seed,
            low_duration,
            interruption_at,
            urgent_duration,
        }
    }
}

/// Test-local input generator only; FlowRuntime has no engine-RNG integration.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalRun {
    replication_id: u64,
    bytes: Vec<u8>,
    digest: u64,
}

#[derive(Clone, Copy)]
enum Assignment {
    Ordered,
    Reversed,
    Permuted,
}

fn time(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn duration(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn identity_context(template: &u64) -> u64 {
    *template
}

fn all_dispatches(flow: &mut FlowRuntime, resource: ResourceId) -> Vec<FlowDispatch> {
    let mut dispatches = Vec::new();
    for _ in 0..MAX_DISPATCHES {
        match flow.step().expect("Flow dispatch must succeed") {
            Some(dispatch) => {
                assert!(
                    dispatch.error.is_none(),
                    "unexpected dispatch error: {dispatch:?}"
                );
                assert_record_capacity(&dispatch.records);
                dispatches.push(dispatch);
                assert_capacity_conserved(flow, resource);
            }
            None => return dispatches,
        }
    }
    panic!("Flow runtime exceeded the {MAX_DISPATCHES}-dispatch fixture bound");
}

fn assert_capacity_conserved(flow: &FlowRuntime, resource: ResourceId) {
    let snapshot = flow.resource(resource).expect("resource remains present");
    assert_eq!(
        snapshot.available as usize + snapshot.allocations.len(),
        snapshot.total as usize
    );
    assert_eq!(snapshot.active.len(), snapshot.allocations.len());

    let active_requests: BTreeSet<_> = snapshot
        .allocations
        .iter()
        .map(|allocation| allocation.request)
        .collect();
    assert_eq!(active_requests.len(), snapshot.allocations.len());
    assert_eq!(
        snapshot
            .active
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
        snapshot.active.len()
    );
    for request in &snapshot.queued {
        assert!(
            !active_requests.contains(request),
            "request appears in active and waiting indexes"
        );
        assert!(matches!(
            flow.request(*request)
                .expect("queued request remains present")
                .state,
            RequestState::Queued | RequestState::Suspended
        ));
    }
}

// Keep the check for record-level capacity metadata beside the live snapshot check.
fn assert_record_capacity(records: &[LifecycleRecord]) {
    for record in records {
        assert!(record.snapshot.active_count <= record.snapshot.capacity);
    }
}

fn run_replication_strategy(scenario: Scenario, strategy: PreemptionStrategy) -> CanonicalRun {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().expect("actor creation succeeds");
    let resource = flow.create_resource(1).expect("resource creation succeeds");
    let low = if strategy == PreemptionStrategy::Restart {
        flow.create_restartable_work(
            owner,
            duration(scenario.low_duration),
            "q51.low.v1",
            scenario.seed,
            identity_context,
        )
        .expect("restartable low work creation succeeds")
    } else {
        flow.create_work(
            owner,
            duration(scenario.low_duration),
            "q51.low.v1",
            scenario.seed,
        )
        .expect("low work creation succeeds")
    };
    let low_request = flow
        .acquire(resource)
        .owner(owner)
        .at(time(0))
        .priority(9)
        .timed_work(low)
        .preemptible(strategy)
        .submit()
        .expect("low request submission succeeds");

    let urgent = flow
        .create_work(
            owner,
            duration(scenario.urgent_duration),
            "q51.urgent.v1",
            scenario.seed ^ 0x5171_2026,
        )
        .expect("urgent work creation succeeds");
    let urgent_request = flow
        .acquire(resource)
        .owner(owner)
        .at(time(scenario.interruption_at))
        .priority(1)
        .timed_work(urgent)
        .can_preempt(true)
        .submit()
        .expect("urgent request submission succeeds");

    let dispatches = all_dispatches(&mut flow, resource);
    let records: Vec<_> = dispatches
        .iter()
        .flat_map(|dispatch| dispatch.records.iter().cloned())
        .collect();
    let urgent_progress = flow
        .work_progress(urgent)
        .expect("urgent progress is readable");
    let low_progress = flow.work_progress(low).expect("low progress is readable");
    let urgent_state = flow
        .request(urgent_request)
        .expect("urgent request remains");
    let low_state = flow.request(low_request).expect("low request remains");
    let resource_state = flow.resource(resource).expect("resource remains");
    let expected_urgent_end = scenario.interruption_at + scenario.urgent_duration;
    assert_eq!(urgent_progress.state, WorkState::Completed);
    assert_eq!(
        urgent_progress.useful_elapsed,
        duration(scenario.urgent_duration)
    );
    assert_eq!(
        urgent_progress.cumulative_busy,
        duration(scenario.urgent_duration)
    );
    assert_eq!(urgent_progress.remaining, duration(0));
    assert_eq!(urgent_state.state, RequestState::Completed);
    assert_eq!(
        terminal_time(&records, urgent_request, LifecycleTransition::Completed),
        expected_urgent_end
    );
    assert_eq!(urgent_progress.segment_started_at, None);
    assert_eq!(urgent_progress.completion_at, None);

    let (expected_low_state, expected_low_useful, expected_low_busy, expected_low_end) =
        match strategy {
            PreemptionStrategy::Suspend => (
                WorkState::Completed,
                scenario.low_duration,
                scenario.low_duration,
                scenario.low_duration + scenario.urgent_duration,
            ),
            PreemptionStrategy::Abort => (
                WorkState::Aborted,
                scenario.interruption_at,
                scenario.interruption_at,
                scenario.interruption_at,
            ),
            PreemptionStrategy::Restart => (
                WorkState::Completed,
                scenario.low_duration,
                scenario.low_duration + scenario.interruption_at,
                scenario.interruption_at + scenario.urgent_duration + scenario.low_duration,
            ),
        };
    assert!(scenario.interruption_at < scenario.low_duration);
    assert_eq!(low_progress.state, expected_low_state);
    assert_eq!(low_progress.useful_elapsed, duration(expected_low_useful));
    assert_eq!(low_progress.cumulative_busy, duration(expected_low_busy));
    assert_eq!(
        low_progress.remaining,
        duration(if strategy == PreemptionStrategy::Abort {
            scenario.low_duration - scenario.interruption_at
        } else {
            0
        })
    );
    assert_eq!(
        low_progress.attempt_revision,
        u64::from(strategy == PreemptionStrategy::Restart)
    );
    assert_eq!(
        low_state.state,
        match strategy {
            PreemptionStrategy::Abort => RequestState::Aborted,
            _ => RequestState::Completed,
        }
    );
    assert_eq!(
        terminal_time(
            &records,
            low_request,
            match strategy {
                PreemptionStrategy::Abort => LifecycleTransition::Aborted,
                _ => LifecycleTransition::Completed,
            }
        ),
        expected_low_end
    );
    assert_eq!(low_progress.segment_started_at, None);
    assert_eq!(low_progress.completion_at, None);
    let low_transitions: Vec<_> = records
        .iter()
        .filter(|record| record.request == low_request)
        .map(|record| (record.transition, record.at.ticks()))
        .collect();
    let expected_low_transitions = match strategy {
        PreemptionStrategy::Suspend => vec![
            (LifecycleTransition::Queued, 0),
            (LifecycleTransition::Granted, 0),
            (LifecycleTransition::Preempted, scenario.interruption_at),
            (
                LifecycleTransition::Resumed,
                scenario.interruption_at + scenario.urgent_duration,
            ),
            (
                LifecycleTransition::Completed,
                scenario.low_duration + scenario.urgent_duration,
            ),
        ],
        PreemptionStrategy::Abort => vec![
            (LifecycleTransition::Queued, 0),
            (LifecycleTransition::Granted, 0),
            (LifecycleTransition::Preempted, scenario.interruption_at),
            (LifecycleTransition::Aborted, scenario.interruption_at),
        ],
        PreemptionStrategy::Restart => vec![
            (LifecycleTransition::Queued, 0),
            (LifecycleTransition::Granted, 0),
            (LifecycleTransition::Preempted, scenario.interruption_at),
            (
                LifecycleTransition::Restarted,
                scenario.interruption_at + scenario.urgent_duration,
            ),
            (
                LifecycleTransition::Completed,
                scenario.interruption_at + scenario.urgent_duration + scenario.low_duration,
            ),
        ],
    };
    assert_eq!(low_transitions, expected_low_transitions);
    let urgent_transitions: Vec<_> = records
        .iter()
        .filter(|record| record.request == urgent_request)
        .map(|record| (record.transition, record.at.ticks()))
        .collect();
    assert_eq!(
        urgent_transitions,
        vec![
            (LifecycleTransition::Queued, scenario.interruption_at),
            (LifecycleTransition::Granted, scenario.interruption_at),
            (
                LifecycleTransition::Completed,
                scenario.interruption_at + scenario.urgent_duration,
            ),
        ]
    );
    assert_eq!(resource_state.total, 1);
    assert_eq!(resource_state.available, 1);
    assert!(resource_state.active.is_empty());
    assert!(resource_state.allocations.is_empty());
    assert!(resource_state.queued.is_empty());

    let low_context = *flow.work_context::<u64>(low).expect("low context remains");
    let urgent_context = *flow
        .work_context::<u64>(urgent)
        .expect("urgent context remains");
    assert_eq!(low_context, scenario.seed);
    assert_eq!(urgent_context, scenario.seed ^ 0x5171_2026);
    let bytes = encode_run(
        scenario,
        strategy,
        &records,
        &TraceTerminal {
            low_request,
            low_state: &low_state,
            low_work: low,
            low_progress: &low_progress,
            low_context,
            urgent_request,
            urgent_state: &urgent_state,
            urgent_work: urgent,
            urgent_progress: &urgent_progress,
            urgent_context,
            resource,
            resource_state: &resource_state,
        },
    );
    CanonicalRun {
        replication_id: scenario.replication_id,
        digest: fnv1a64(&bytes),
        bytes,
    }
}

fn run_replication(scenario: Scenario) -> Vec<CanonicalRun> {
    [
        PreemptionStrategy::Suspend,
        PreemptionStrategy::Abort,
        PreemptionStrategy::Restart,
    ]
    .into_iter()
    .map(|strategy| run_replication_strategy(scenario, strategy))
    .collect()
}

fn terminal_time(
    records: &[LifecycleRecord],
    request: RequestId,
    transition: LifecycleTransition,
) -> u128 {
    records
        .iter()
        .find(|record| record.request == request && record.transition == transition)
        .expect("expected terminal lifecycle record exists")
        .at
        .ticks()
}

fn scenarios() -> Vec<Scenario> {
    REPLICATION_SEEDS
        .iter()
        .enumerate()
        .map(|(index, seed)| Scenario::from_seed(10_001 + index as u64, *seed))
        .collect()
}

struct ReleaseGates(Vec<Sender<()>>);

impl Drop for ReleaseGates {
    fn drop(&mut self) {
        // Unblock every finished worker if an assertion unwinds before normal release.
        for sender in &self.0 {
            let _ = sender.send(());
        }
    }
}

fn run_batch(
    worker_count: usize,
    assignment: Assignment,
) -> (Vec<CanonicalRun>, Vec<u64>, Vec<usize>) {
    assert!(worker_count > 0);
    let cases = scenarios();
    let ordered_cases: Vec<_> = match assignment {
        Assignment::Ordered => cases,
        Assignment::Reversed => cases.into_iter().rev().collect(),
        Assignment::Permuted => {
            let mut rotated = cases;
            rotated.rotate_left(3);
            rotated
        }
    };
    let mut buckets = vec![Vec::new(); worker_count];
    for (position, scenario) in ordered_cases.into_iter().enumerate() {
        let worker = match assignment {
            Assignment::Ordered => position % worker_count,
            Assignment::Reversed => (position * 3 + 1) % worker_count,
            Assignment::Permuted => (position * 5 + 2) % worker_count,
        };
        buckets[worker].push(scenario);
    }

    let (mut collected, completion_order) = thread::scope(|scope| {
        let (ready_sender, ready_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        let mut gates = ReleaseGates(Vec::with_capacity(worker_count));
        let mut handles = Vec::with_capacity(worker_count);
        for (worker, bucket) in buckets.into_iter().enumerate() {
            let (release_sender, release_receiver) = mpsc::channel();
            gates.0.push(release_sender);
            let ready_sender = ready_sender.clone();
            let result_sender = result_sender.clone();
            handles.push(scope.spawn(move || {
                // Each FlowRuntime is built and consumed here, on its own worker.
                let output: Vec<_> = bucket.into_iter().flat_map(run_replication).collect();
                ready_sender
                    .send(worker)
                    .expect("coordinator remains available until workers are ready");
                release_receiver
                    .recv()
                    .expect("coordinator releases each ready worker");
                result_sender
                    .send((worker, output))
                    .expect("coordinator remains available until output is collected");
            }));
        }
        drop(ready_sender);
        drop(result_sender);

        let mut ready_workers = BTreeSet::new();
        for _ in 0..worker_count {
            let worker = ready_receiver
                .recv_timeout(WallDuration::from_secs(10))
                .expect("all workers finish before the bounded ready timeout");
            assert!(ready_workers.insert(worker), "worker reported ready twice");
        }
        assert_eq!(ready_workers.len(), worker_count);
        let collection_order: Vec<_> = match assignment {
            Assignment::Ordered => (0..worker_count).collect(),
            Assignment::Reversed => (0..worker_count).rev().collect(),
            Assignment::Permuted => (0..worker_count)
                .map(|position| (position * 3 + 1) % worker_count)
                .collect(),
        };
        let mut collected = Vec::new();
        let mut actual_completion_order = Vec::with_capacity(worker_count);
        for expected_worker in &collection_order {
            gates.0[*expected_worker]
                .send(())
                .expect("ready worker release gate is connected");
            let (worker, output) = result_receiver
                .recv_timeout(WallDuration::from_secs(10))
                .expect("released worker submits output before bounded timeout");
            assert_eq!(worker, *expected_worker);
            actual_completion_order.push(worker);
            collected.extend(output);
        }
        assert_eq!(actual_completion_order, collection_order);
        for handle in handles {
            handle.join().expect("worker thread exits cleanly");
        }
        (collected, actual_completion_order)
    });

    let collected_ids: Vec<_> = collected.iter().map(|run| run.replication_id).collect();
    collected.sort_by_key(|run| run.replication_id);
    println!(
        "Q51_BATCH workers={worker_count} assignment={} traces={} completion_order={completion_order:?} aggregate_fnv1a64={:016x}",
        assignment.label(),
        collected.len(),
        aggregate_digest(&collected)
    );
    (collected, collected_ids, completion_order)
}

impl Assignment {
    fn label(self) -> &'static str {
        match self {
            Self::Ordered => "ordered",
            Self::Reversed => "reversed",
            Self::Permuted => "permuted",
        }
    }
}

fn aggregate_digest(runs: &[CanonicalRun]) -> u64 {
    let mut encoded = b"KAIROS-Q51-BATCH\0".to_vec();
    put_u32(&mut encoded, 1);
    put_len(&mut encoded, runs.len());
    for run in runs {
        put_len(&mut encoded, run.bytes.len());
        encoded.extend_from_slice(&run.bytes);
    }
    fnv1a64(&encoded)
}

#[test]
fn independent_replications_match_across_repeat_and_worker_schedules() {
    let cases = scenarios();
    let distinct_inputs: BTreeSet<_> = cases
        .iter()
        .map(|case| {
            (
                case.low_duration,
                case.interruption_at,
                case.urgent_duration,
            )
        })
        .collect();
    assert!(
        distinct_inputs.len() >= 4,
        "seeds must change scenario inputs"
    );

    let (baseline, baseline_collection, _) = run_batch(1, Assignment::Ordered);
    let (repeat, _, _) = run_batch(1, Assignment::Ordered);
    assert_runs_equal(&baseline, &repeat);
    assert!(baseline_collection
        .windows(2)
        .all(|pair| pair[0] <= pair[1]));

    let (reverse_serial, reverse_serial_collection, reverse_serial_completion) =
        run_batch(1, Assignment::Reversed);
    assert_ne!(baseline_collection, reverse_serial_collection);
    assert_eq!(reverse_serial_completion, vec![0]);
    assert_runs_equal(&baseline, &reverse_serial);

    for worker_count in [1, 2, 4] {
        for assignment in [
            Assignment::Ordered,
            Assignment::Reversed,
            Assignment::Permuted,
        ] {
            let (candidate, _, actual_completion) = run_batch(worker_count, assignment);
            assert_eq!(actual_completion.len(), worker_count);
            assert_runs_equal(&baseline, &candidate);
        }
    }
}

fn assert_runs_equal(expected: &[CanonicalRun], actual: &[CanonicalRun]) {
    assert_eq!(expected.len(), actual.len());
    for (expected, actual) in expected.iter().zip(actual) {
        assert_eq!(expected.replication_id, actual.replication_id);
        assert_eq!(expected.digest, fnv1a64(&expected.bytes));
        assert_eq!(actual.digest, fnv1a64(&actual.bytes));
        assert_eq!(expected.bytes, actual.bytes);
        assert_eq!(expected.digest, actual.digest);
    }
}

struct TraceTerminal<'a> {
    low_request: RequestId,
    low_state: &'a kairo_ecs_des::ResourceRequest,
    low_work: WorkId,
    low_progress: &'a WorkProgress,
    low_context: u64,
    urgent_request: RequestId,
    urgent_state: &'a kairo_ecs_des::ResourceRequest,
    urgent_work: WorkId,
    urgent_progress: &'a WorkProgress,
    urgent_context: u64,
    resource: ResourceId,
    resource_state: &'a ResourceSnapshot,
}

fn encode_run(
    scenario: Scenario,
    strategy: PreemptionStrategy,
    records: &[LifecycleRecord],
    terminal: &TraceTerminal<'_>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"KAIROS-Q51-FLOW-TRACE\0");
    put_u32(&mut bytes, 1);
    put_u64(&mut bytes, scenario.replication_id);
    put_u64(&mut bytes, scenario.seed);
    put_u128(&mut bytes, scenario.low_duration);
    put_u128(&mut bytes, scenario.interruption_at);
    put_u128(&mut bytes, scenario.urgent_duration);
    put_u8(&mut bytes, strategy_tag(strategy));
    put_len(&mut bytes, records.len());
    for record in records {
        encode_record(&mut bytes, record);
    }
    put_request_id(&mut bytes, terminal.low_request);
    encode_request_state(&mut bytes, terminal.low_state);
    put_work_id(&mut bytes, terminal.low_work);
    encode_progress(&mut bytes, terminal.low_progress);
    put_u64(&mut bytes, terminal.low_context);
    put_request_id(&mut bytes, terminal.urgent_request);
    encode_request_state(&mut bytes, terminal.urgent_state);
    put_work_id(&mut bytes, terminal.urgent_work);
    encode_progress(&mut bytes, terminal.urgent_progress);
    put_u64(&mut bytes, terminal.urgent_context);
    put_resource_id(&mut bytes, terminal.resource);
    encode_resource_snapshot(&mut bytes, terminal.resource_state);
    bytes
}

fn encode_record(bytes: &mut Vec<u8>, record: &LifecycleRecord) {
    put_request_id(bytes, record.request);
    put_resource_id(bytes, record.resource);
    put_u128(bytes, record.at.ticks());
    put_u8(bytes, request_state_tag(record.state));
    put_option_lease(bytes, record.lease);
    put_event_id(bytes, record.causal_event_id);
    put_u32(bytes, record.transition_ordinal);
    put_u8(bytes, transition_tag(record.transition));
    encode_lifecycle_snapshot(bytes, &record.snapshot);
}

fn encode_lifecycle_snapshot(bytes: &mut Vec<u8>, snapshot: &LifecycleSnapshot) {
    put_entity_id(bytes, snapshot.owner);
    put_option_work_id(bytes, snapshot.work);
    put_i32(bytes, snapshot.priority_level);
    put_u32(bytes, snapshot.capacity);
    put_u32(bytes, snapshot.queue_len);
    put_u32(bytes, snapshot.active_count);
    put_option_strategy(bytes, snapshot.strategy);
    put_option_request_id(bytes, snapshot.preemptor_request);
    put_option_lease(bytes, snapshot.causal_lease);
    match &snapshot.progress {
        Some(progress) => {
            put_u8(bytes, 1);
            encode_progress(bytes, progress);
        }
        None => put_u8(bytes, 0),
    }
}

fn encode_request_state(bytes: &mut Vec<u8>, request: &kairo_ecs_des::ResourceRequest) {
    put_resource_id(bytes, request.resource);
    put_entity_id(bytes, request.owner);
    match request.admission_sequence {
        Some(sequence) => {
            put_u8(bytes, 1);
            put_u64(bytes, sequence);
        }
        None => put_u8(bytes, 0),
    }
    put_u8(bytes, request_state_tag(request.state));
    put_option_lease(bytes, request.lease);
    put_i32(bytes, request.priority_level);
    put_option_work_id(bytes, request.work);
    put_u128(bytes, request.submitted_at.ticks());
    match request.deadline {
        Some(deadline) => {
            put_u8(bytes, 1);
            put_u128(bytes, deadline.ticks());
        }
        None => put_u8(bytes, 0),
    }
    put_u8(bytes, u8::from(request.timed));
    put_u8(bytes, u8::from(request.can_preempt));
    put_option_strategy(bytes, request.preemptible);
}

fn encode_progress(bytes: &mut Vec<u8>, progress: &WorkProgress) {
    put_u128(bytes, progress.original_duration.ticks());
    put_u128(bytes, progress.useful_elapsed.ticks());
    put_u128(bytes, progress.remaining.ticks());
    put_u128(bytes, progress.cumulative_busy.ticks());
    put_u64(bytes, progress.attempt_revision);
    put_u64(bytes, progress.execution_revision);
    put_u8(bytes, work_state_tag(progress.state));
    put_option_time(bytes, progress.segment_started_at);
    put_option_time(bytes, progress.completion_at);
}

fn encode_resource_snapshot(bytes: &mut Vec<u8>, snapshot: &ResourceSnapshot) {
    put_u32(bytes, snapshot.total);
    put_u32(bytes, snapshot.available);
    put_len(bytes, snapshot.queued.len());
    for request in &snapshot.queued {
        put_request_id(bytes, *request);
    }
    put_len(bytes, snapshot.active.len());
    for lease in &snapshot.active {
        put_lease(bytes, *lease);
    }
    put_len(bytes, snapshot.allocations.len());
    for allocation in &snapshot.allocations {
        put_lease(bytes, allocation.lease);
        put_request_id(bytes, allocation.request);
        put_entity_id(bytes, allocation.owner);
        put_option_work_id(bytes, allocation.work);
        put_i32(bytes, allocation.priority_level);
        put_u128(bytes, allocation.granted_at.ticks());
        put_u128(bytes, allocation.segment_started_at.ticks());
        put_option_time(bytes, allocation.completion_at);
    }
}

fn put_option_time(bytes: &mut Vec<u8>, value: Option<SimTime>) {
    match value {
        Some(value) => {
            put_u8(bytes, 1);
            put_u128(bytes, value.ticks());
        }
        None => put_u8(bytes, 0),
    }
}

fn put_option_strategy(bytes: &mut Vec<u8>, value: Option<PreemptionStrategy>) {
    match value {
        Some(value) => {
            put_u8(bytes, 1);
            put_u8(bytes, strategy_tag(value));
        }
        None => put_u8(bytes, 0),
    }
}

fn put_option_work_id(bytes: &mut Vec<u8>, value: Option<WorkId>) {
    match value {
        Some(value) => {
            put_u8(bytes, 1);
            put_work_id(bytes, value);
        }
        None => put_u8(bytes, 0),
    }
}

fn put_option_request_id(bytes: &mut Vec<u8>, value: Option<RequestId>) {
    match value {
        Some(value) => {
            put_u8(bytes, 1);
            put_request_id(bytes, value);
        }
        None => put_u8(bytes, 0),
    }
}

fn put_option_lease(bytes: &mut Vec<u8>, value: Option<kairo_ecs_des::LeaseId>) {
    match value {
        Some(value) => {
            put_u8(bytes, 1);
            put_lease(bytes, value);
        }
        None => put_u8(bytes, 0),
    }
}

fn put_lease(bytes: &mut Vec<u8>, value: kairo_ecs_des::LeaseId) {
    put_request_id(bytes, value.request_id());
    put_u64(bytes, value.revision());
}

fn put_entity_id(bytes: &mut Vec<u8>, value: EntityId) {
    put_u64(bytes, value.index);
    put_u32(bytes, value.generation);
}

fn put_event_id(bytes: &mut Vec<u8>, value: EventId) {
    put_u64(bytes, value.index);
    put_u32(bytes, value.generation);
}

fn put_request_id(bytes: &mut Vec<u8>, value: RequestId) {
    put_entity_id(bytes, value.entity_id());
}

fn put_resource_id(bytes: &mut Vec<u8>, value: ResourceId) {
    put_entity_id(bytes, value.entity_id());
}

fn put_work_id(bytes: &mut Vec<u8>, value: WorkId) {
    put_entity_id(bytes, value.entity_id());
}

fn put_len(bytes: &mut Vec<u8>, value: usize) {
    put_u64(
        bytes,
        u64::try_from(value).expect("fixture lengths fit u64"),
    );
}

fn put_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_u128(bytes: &mut Vec<u8>, value: u128) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn strategy_tag(value: PreemptionStrategy) -> u8 {
    match value {
        PreemptionStrategy::Suspend => 1,
        PreemptionStrategy::Abort => 2,
        PreemptionStrategy::Restart => 3,
    }
}

fn request_state_tag(value: RequestState) -> u8 {
    match value {
        RequestState::Pending => 1,
        RequestState::Queued => 2,
        RequestState::Active => 3,
        RequestState::Released => 4,
        RequestState::Cancelled => 5,
        RequestState::TimedOut => 6,
        RequestState::Suspended => 7,
        RequestState::Completed => 8,
        RequestState::Aborted => 9,
    }
}

fn work_state_tag(value: WorkState) -> u8 {
    match value {
        WorkState::Pending => 1,
        WorkState::Active => 2,
        WorkState::Suspended => 3,
        WorkState::Completed => 4,
        WorkState::Aborted => 5,
        WorkState::Cancelled => 6,
        WorkState::Released => 7,
    }
}

fn transition_tag(value: LifecycleTransition) -> u8 {
    match value {
        LifecycleTransition::Queued => 1,
        LifecycleTransition::Granted => 2,
        LifecycleTransition::Released => 3,
        LifecycleTransition::Cancelled => 4,
        LifecycleTransition::TimedOut => 5,
        LifecycleTransition::Preempted => 6,
        LifecycleTransition::Resumed => 7,
        LifecycleTransition::Restarted => 8,
        LifecycleTransition::Completed => 9,
        LifecycleTransition::Aborted => 10,
    }
}

/// FNV-1a is an explicit local regression checksum; encoded-byte equality is the oracle.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
