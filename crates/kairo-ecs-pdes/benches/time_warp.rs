use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use kairo_ecs_pdes::{
    ConservativeProcess, ConservativeRuntime, LpId, OptimisticLimits, OptimisticProcess,
    OptimisticRunProgress, OptimisticRuntime, OptimisticRuntimeReport, PartitionPlan, RemoteEvent,
    RuntimeReport, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};

const LP_COUNTS: [u32; 2] = [4, 8];
const ROOTS_PER_LP: usize = 8;
const MAX_HOPS: u8 = 8;
const WARMUP_RUNS: usize = 1;
const REPEATS: usize = 5;
const SEED: u64 = 48_2027;
const SPARSE_PERCENT: u8 = 10;
const DENSE_PERCENT: u8 = 80;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EventMark {
    tick: Tick,
    source: LpId,
    destination: LpId,
    payload: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProcessState {
    processed: u64,
    value: u64,
    rng_state: u64,
    trace: Vec<EventMark>,
}

#[derive(Clone, Debug)]
struct WorkloadProcess {
    lp_count: u32,
    emit_percent: u8,
    state: ProcessState,
}

impl WorkloadProcess {
    fn new(lp_count: u32, emit_percent: u8, seed: u64, lp_id: LpId) -> Self {
        Self {
            lp_count,
            emit_percent,
            state: ProcessState {
                rng_state: seed ^ (u64::from(lp_id.0).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
                ..ProcessState::default()
            },
        }
    }

    fn handle(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        let (root, remaining_hops, payload_value) = decode_payload(&event.event_payload);
        let draw = splitmix64(&mut self.state.rng_state);
        self.state.processed += 1;
        self.state.value = self
            .state
            .value
            .rotate_left(11)
            .wrapping_mul(0x9e37_79b1_85eb_ca87)
            .wrapping_add(payload_value ^ draw ^ event.tick.ticks() as u64);
        self.state.trace.push(EventMark {
            tick: event.tick,
            source: event.source_lp,
            destination: event.dest_lp,
            payload: event.event_payload.clone(),
        });

        if remaining_hops == 0 || draw % 100 >= u64::from(self.emit_percent) {
            return Vec::new();
        }

        let mut next_value = payload_value ^ splitmix64(&mut self.state.rng_state);
        next_value = next_value.rotate_left((root % 63) as u32);
        let next_tick = event
            .tick
            .checked_add(SimDuration::from_ticks(1))
            .expect("bounded benchmark horizon leaves tick headroom");
        vec![RemoteEvent {
            source_lp: event.dest_lp,
            dest_lp: LpId((event.dest_lp.0 + 1) % self.lp_count),
            tick: next_tick,
            event_payload: encode_payload(root, remaining_hops - 1, next_value),
        }]
    }
}

impl ConservativeProcess for WorkloadProcess {
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.handle(event)
    }
}

impl OptimisticProcess for WorkloadProcess {
    type Snapshot = ProcessState;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }

    fn restore(
        &mut self,
        state: &Self::Snapshot,
    ) -> Result<(), kairo_ecs_pdes::OptimisticStateError> {
        self.state = state.clone();
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.handle(event)
    }
}

#[derive(Clone, Debug)]
struct Workload {
    lp_count: u32,
    emit_percent: u8,
    horizon: Tick,
    inputs: Vec<RemoteEvent>,
}

fn make_workload(lp_count: u32, emit_percent: u8) -> Workload {
    let mut inputs = Vec::with_capacity(lp_count as usize * ROOTS_PER_LP);
    for lp in 0..lp_count {
        for root_index in 0..ROOTS_PER_LP {
            let root_sequence = u64::from(lp) * ROOTS_PER_LP as u64 + root_index as u64;
            let mut root_state = SEED ^ root_sequence;
            let payload_value = splitmix64(&mut root_state);
            inputs.push(RemoteEvent {
                source_lp: LpId(lp),
                dest_lp: LpId(lp),
                tick: SimTime::from_ticks(u128::from(lp) * 64 + (root_index as u128) * 4),
                event_payload: encode_payload(root_sequence, MAX_HOPS, payload_value),
            });
        }
    }
    inputs.sort_by_key(|event| (event.tick, event.source_lp, event.event_payload.clone()));
    let horizon = inputs
        .last()
        .expect("every workload has roots")
        .tick
        .checked_add(SimDuration::from_ticks(u128::from(MAX_HOPS) + 2))
        .expect("fixed workload horizon must fit");
    Workload {
        lp_count,
        emit_percent,
        horizon,
        inputs,
    }
}

fn encode_payload(root: u64, remaining_hops: u8, value: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(17);
    bytes.extend_from_slice(&root.to_le_bytes());
    bytes.push(remaining_hops);
    bytes.extend_from_slice(&value.to_le_bytes());
    bytes
}

fn decode_payload(bytes: &[u8]) -> (u64, u8, u64) {
    assert_eq!(bytes.len(), 17, "benchmark payload is internally generated");
    let root = u64::from_le_bytes(bytes[0..8].try_into().expect("fixed payload"));
    let remaining_hops = bytes[8];
    let value = u64::from_le_bytes(bytes[9..17].try_into().expect("fixed payload"));
    (root, remaining_hops, value)
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn topology(lp_count: u32) -> BTreeMap<LpId, Vec<LpId>> {
    (0..lp_count)
        .map(|lp| (LpId(lp), vec![LpId((lp + 1) % lp_count)]))
        .collect()
}

fn partition(lp_count: u32) -> PartitionPlan {
    let entities = (0..lp_count)
        .map(|lp| EntityId::new(u64::from(lp), 0))
        .collect();
    PartitionPlan::from_entities(lp_count, SimDuration::from_ticks(1), entities)
        .expect("benchmark partition is valid")
}

fn processes(lp_count: u32, emit_percent: u8) -> BTreeMap<LpId, WorkloadProcess> {
    (0..lp_count)
        .map(|lp| {
            (
                LpId(lp),
                WorkloadProcess::new(lp_count, emit_percent, SEED, LpId(lp)),
            )
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RunState {
    per_lp: Vec<ProcessState>,
    trace: Vec<EventMark>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ConservativeCounters {
    processed: u64,
    remote: u64,
    emitted: u64,
    null_messages: u64,
    rounds: u64,
    spawned_worker_cohort_max: usize,
}

fn prepare_conservative(workload: &Workload) -> ConservativeRuntime<WorkloadProcess> {
    let mut runtime = ConservativeRuntime::new(
        partition(workload.lp_count),
        topology(workload.lp_count),
        processes(workload.lp_count, workload.emit_percent),
    )
    .expect("conservative runtime setup must succeed");
    for event in &workload.inputs {
        runtime
            .schedule_initial(event.clone())
            .expect("fixed inputs must be route-valid");
    }
    runtime
}

fn observe_conservative(
    runtime: &ConservativeRuntime<WorkloadProcess>,
    report: RuntimeReport,
) -> (RunState, ConservativeCounters) {
    let per_lp = runtime
        .processes()
        .values()
        .map(|process| process.state.clone())
        .collect::<Vec<_>>();
    let mut trace = per_lp
        .iter()
        .flat_map(|state| state.trace.iter().cloned())
        .collect::<Vec<_>>();
    trace.sort();
    (
        RunState { per_lp, trace },
        ConservativeCounters {
            processed: report.processed_events,
            remote: report.remote_events,
            emitted: report.emitted_events,
            null_messages: report.null_messages,
            rounds: report.rounds,
            spawned_worker_cohort_max: report.worker_count,
        },
    )
}

fn run_conservative(workload: &Workload) -> (RunState, ConservativeCounters) {
    let mut runtime = prepare_conservative(workload);
    let report = runtime
        .run_until(workload.horizon)
        .expect("bounded conservative workload must complete");
    observe_conservative(&runtime, report)
}

fn timed_conservative(workload: &Workload) -> (u128, (RunState, ConservativeCounters)) {
    let mut runtime = prepare_conservative(workload);
    let (elapsed, result) = elapsed_ns(|| runtime.run_until(workload.horizon));
    let report = result.expect("bounded conservative workload must complete");
    (elapsed, observe_conservative(&runtime, report))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct OptimisticCounters {
    executions: u64,
    first_attempt_executions: u64,
    extra_executions: u64,
    replay_executions: u64,
    rollback_attempts: u64,
    rolled_back_events: u64,
    max_rollback_depth: usize,
    canceled_sends: u64,
    fossil_collected_events: u64,
    checkpoints_before_fossil: usize,
    fossil_collected_checkpoints: usize,
    committed_logical_ids: usize,
}

fn prepare_optimistic(workload: &Workload) -> OptimisticRuntime<WorkloadProcess> {
    let mut runtime = OptimisticRuntime::new(
        partition(workload.lp_count),
        topology(workload.lp_count),
        processes(workload.lp_count, workload.emit_percent),
        OptimisticLimits::default(),
    )
    .expect("optimistic runtime setup must succeed");
    for (sequence, event) in workload.inputs.iter().enumerate() {
        runtime
            .schedule_initial(sequence as u64, event.clone())
            .expect("fixed inputs must be route-valid");
    }
    runtime
}

fn observe_optimistic(
    mut runtime: OptimisticRuntime<WorkloadProcess>,
    workload: &Workload,
    progress: OptimisticRunProgress,
) -> (RunState, OptimisticCounters) {
    assert!(
        !progress.budget_exhausted,
        "benchmark work budget exhausted"
    );
    assert_eq!(progress.pending_positives, 0);
    assert_eq!(progress.pending_antis, 0);
    assert_eq!(progress.replay_pending, 0);

    let pre_fossil_report: OptimisticRuntimeReport = runtime.report();
    let commit_floor = workload
        .horizon
        .checked_add(SimDuration::from_ticks(1))
        .expect("fixed commit floor must fit");
    let fossil = runtime
        .fossil_collect(commit_floor)
        .expect("completed workload can be committed through its horizon");
    let report = runtime.report();
    let per_lp = (0..workload.lp_count)
        .map(|lp| {
            runtime
                .process_at(LpId(lp))
                .expect("partition LP must exist")
                .state
                .clone()
        })
        .collect::<Vec<_>>();
    let mut trace = fossil
        .collected
        .iter()
        .map(|entry| EventMark {
            tick: entry.event.tick,
            source: entry.event.source_lp,
            destination: entry.event.dest_lp,
            payload: entry.event.event_payload.clone(),
        })
        .collect::<Vec<_>>();
    trace.sort();
    let logical_identities = fossil
        .collected
        .iter()
        .map(|entry| (entry.event.source_lp, entry.logical_id.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        logical_identities.len(),
        fossil.collected.len(),
        "committed (actual source LP, logical ID) pairs must be unique"
    );
    let processed = per_lp.iter().map(|state| state.processed).sum::<u64>();
    assert_eq!(fossil.collected.len() as u64, processed);
    let committed = u64::try_from(logical_identities.len())
        .expect("benchmark committed event count fits the runtime counters");
    assert!(report.executions >= committed);
    let first_attempt_executions = report.executions - report.replay_executions;
    assert_eq!(
        report.executions,
        first_attempt_executions + report.replay_executions
    );
    let extra_executions = report.executions - committed;
    assert!(
        report.rollback_attempts > 0,
        "workload must exercise rollback"
    );
    assert!(
        report.rolled_back_events > 0,
        "workload must roll back work"
    );
    assert!(report.replay_executions > 0, "workload must replay work");
    (
        RunState { per_lp, trace },
        OptimisticCounters {
            executions: report.executions,
            first_attempt_executions,
            extra_executions,
            replay_executions: report.replay_executions,
            rollback_attempts: report.rollback_attempts,
            rolled_back_events: report.rolled_back_events,
            max_rollback_depth: report.max_rollback_depth,
            canceled_sends: report.canceled_sends,
            fossil_collected_events: report.fossil_collected_events,
            checkpoints_before_fossil: pre_fossil_report.checkpoints,
            fossil_collected_checkpoints: fossil.collected_checkpoints,
            committed_logical_ids: logical_identities.len(),
        },
    )
}

fn run_optimistic(workload: &Workload) -> (RunState, OptimisticCounters) {
    let mut runtime = prepare_optimistic(workload);
    let progress = runtime
        .run_until_with_budget(workload.horizon, 100_000)
        .expect("bounded optimistic workload must complete");
    observe_optimistic(runtime, workload, progress)
}

fn timed_optimistic(workload: &Workload) -> (u128, (RunState, OptimisticCounters)) {
    let mut runtime = prepare_optimistic(workload);
    let (elapsed, result) = elapsed_ns(|| runtime.run_until_with_budget(workload.horizon, 100_000));
    let progress = result.expect("bounded optimistic workload must complete");
    (elapsed, observe_optimistic(runtime, workload, progress))
}

fn verify_pair(workload: &Workload) -> (RunState, ConservativeCounters, OptimisticCounters, usize) {
    let (conservative, conservative_counters) = run_conservative(workload);
    let (optimistic, optimistic_counters) = run_optimistic(workload);
    assert_eq!(
        optimistic.per_lp, conservative.per_lp,
        "final model state and per-LP RNG snapshots must match"
    );
    assert_eq!(
        optimistic.trace, conservative.trace,
        "fossil-committed optimistic event graph must match conservative model trace"
    );
    assert_eq!(
        optimistic_counters.committed_logical_ids,
        conservative.trace.len(),
        "each committed event must have one unique logical identity"
    );
    let expected_committed_events = conservative.trace.len();
    (
        conservative,
        conservative_counters,
        optimistic_counters,
        expected_committed_events,
    )
}

fn elapsed_ns<F: FnOnce() -> R, R>(action: F) -> (u128, R) {
    let started = Instant::now();
    let result = action();
    (started.elapsed().as_nanos(), result)
}

fn json_u128s(values: &[u128]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(u128::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_events(events: &[RemoteEvent]) -> String {
    let rows = events
        .iter()
        .map(|event| {
            let (root, remaining_hops, value) = decode_payload(&event.event_payload);
            format!(
                "{{\"source\":{},\"destination\":{},\"tick\":{},\"root\":{},\"remaining_hops\":{},\"value\":{}}}",
                event.source_lp.0,
                event.dest_lp.0,
                event.tick.ticks(),
                root,
                remaining_hops,
                value
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

fn main() {
    let mut rows = Vec::new();
    for lp_count in LP_COUNTS {
        for (profile, emit_percent) in [("sparse", SPARSE_PERCENT), ("dense", DENSE_PERCENT)] {
            let workload = make_workload(lp_count, emit_percent);
            let (
                expected_state,
                preflight_conservative,
                preflight_optimistic,
                expected_committed_events,
            ) = verify_pair(&workload);
            assert!(
                preflight_optimistic.rollback_attempts > 0,
                "workload must exercise rollback before it is benchmarked"
            );
            assert!(
                preflight_optimistic.fossil_collected_checkpoints > 0,
                "workload must exercise checkpoint fossil collection"
            );

            for _ in 0..WARMUP_RUNS {
                let (conservative, conservative_counters) = run_conservative(&workload);
                let (optimistic, optimistic_counters) = run_optimistic(&workload);
                assert_eq!(conservative, expected_state);
                assert_eq!(optimistic, conservative);
                assert_eq!(conservative_counters, preflight_conservative);
                assert_eq!(optimistic_counters, preflight_optimistic);
            }

            let mut conservative_ns = Vec::with_capacity(REPEATS);
            let mut optimistic_ns = Vec::with_capacity(REPEATS);
            let mut conservative_counters = preflight_conservative;
            let mut optimistic_counters = preflight_optimistic;
            for repeat in 0..REPEATS {
                let (conservative_time, conservative_result, optimistic_time, optimistic_result) =
                    if repeat % 2 == 0 {
                        let (conservative_time, conservative_result) =
                            timed_conservative(&workload);
                        let (optimistic_time, optimistic_result) = timed_optimistic(&workload);
                        (
                            conservative_time,
                            conservative_result,
                            optimistic_time,
                            optimistic_result,
                        )
                    } else {
                        let (optimistic_time, optimistic_result) = timed_optimistic(&workload);
                        let (conservative_time, conservative_result) =
                            timed_conservative(&workload);
                        (
                            conservative_time,
                            conservative_result,
                            optimistic_time,
                            optimistic_result,
                        )
                    };
                assert_eq!(conservative_result.0, expected_state);
                assert_eq!(optimistic_result.0, conservative_result.0);
                assert_eq!(conservative_result.1, preflight_conservative);
                assert_eq!(optimistic_result.1, preflight_optimistic);
                conservative_ns.push(conservative_time);
                optimistic_ns.push(optimistic_time);
                conservative_counters = conservative_result.1;
                optimistic_counters = optimistic_result.1;
            }

            rows.push(format!(
                "{{\"profile\":\"{profile}\",\"emit_percent\":{emit_percent},\"lp_count\":{lp_count},\"roots_per_lp\":{ROOTS_PER_LP},\"max_hops\":{MAX_HOPS},\"horizon\":{},\"seed\":{SEED},\"expected_committed_events\":{expected_committed_events},\"input_events\":{},\"conservative_ns\":{},\"optimistic_ns\":{},\"parity\":true,\"conservative_counters\":{{\"processed_events\":{},\"remote_events\":{},\"emitted_events\":{},\"null_messages\":{},\"rounds\":{},\"spawned_worker_cohort_max\":{}}},\"optimistic_counters\":{{\"executions\":{},\"first_attempt_executions\":{},\"extra_executions\":{},\"replay_executions\":{},\"rollback_attempts\":{},\"rolled_back_events\":{},\"max_rollback_depth\":{},\"canceled_sends\":{},\"fossil_collected_events\":{},\"checkpoints_before_fossil\":{},\"fossil_collected_checkpoints\":{},\"committed_logical_ids\":{}}}}}",
                workload.horizon.ticks(),
                json_events(&workload.inputs),
                json_u128s(&conservative_ns),
                json_u128s(&optimistic_ns),
                conservative_counters.processed,
                conservative_counters.remote,
                conservative_counters.emitted,
                conservative_counters.null_messages,
                conservative_counters.rounds,
                conservative_counters.spawned_worker_cohort_max,
                optimistic_counters.executions,
                optimistic_counters.first_attempt_executions,
                optimistic_counters.extra_executions,
                optimistic_counters.replay_executions,
                optimistic_counters.rollback_attempts,
                optimistic_counters.rolled_back_events,
                optimistic_counters.max_rollback_depth,
                optimistic_counters.canceled_sends,
                optimistic_counters.fossil_collected_events,
                optimistic_counters.checkpoints_before_fossil,
                optimistic_counters.fossil_collected_checkpoints,
                optimistic_counters.committed_logical_ids,
            ));
        }
    }
    println!(
        "{{\"schema_version\":\"kairoecs.pdes.time_warp_benchmark.v1\",\"seed\":{SEED},\"warmup_runs\":{WARMUP_RUNS},\"repetitions\":{REPEATS},\"timed_boundary\":\"runtime_run_call\",\"excluded_costs\":[\"process construction\",\"partition and topology construction\",\"initial event scheduling\",\"final state/report extraction\",\"parity and logical ID validation\",\"fossil collection\"],\"profiles\":[{}]}}",
        rows.join(",")
    );
}
