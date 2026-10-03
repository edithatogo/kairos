use std::collections::BTreeMap;
use std::time::Instant;

use kairo_ecs_core::Scheduler;
use kairo_ecs_pdes::{ConservativeProcess, ConservativeRuntime, LpId, PartitionPlan, RemoteEvent};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};

const LP_COUNTS: [u32; 4] = [4, 8, 16, 32];
const WARMUP_RUNS: usize = 1;
const DEFAULT_REPETITIONS: usize = 5;
const DEFAULT_SEED: u64 = 47_2026;
const STRONG_TOTAL_EVENTS: usize = 2_048;
const WEAK_EVENTS_PER_LP: usize = 128;

#[derive(Clone, Copy, Debug)]
struct Config {
    seed: u64,
    repetitions: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProcessState {
    processed: u64,
    checksum: u64,
}

#[derive(Debug)]
struct WorkloadProcess {
    lp_count: u32,
    state: ProcessState,
}

impl WorkloadProcess {
    fn new(lp_count: u32) -> Self {
        Self {
            lp_count,
            state: ProcessState::default(),
        }
    }
}

impl ConservativeProcess for WorkloadProcess {
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        let Some((&kind, bytes)) = event.event_payload.split_first() else {
            return Vec::new();
        };
        if bytes.len() != 8 {
            return Vec::new();
        }
        let value = u64::from_le_bytes(bytes.try_into().expect("eight-byte payload"));
        self.state.processed += 1;
        self.state.checksum = self.state.checksum.wrapping_add(value);

        if kind == 0 {
            vec![RemoteEvent {
                source_lp: event.dest_lp,
                dest_lp: LpId((event.dest_lp.0 + 1) % self.lp_count),
                tick: event.tick.saturating_add(SimDuration::from_ticks(1)),
                event_payload: encode_payload(1, value),
            }]
        } else {
            Vec::new()
        }
    }
}

fn encode_payload(kind: u8, value: u64) -> Vec<u8> {
    let mut payload = Vec::with_capacity(9);
    payload.push(kind);
    payload.extend_from_slice(&value.to_le_bytes());
    payload
}

fn config() -> Result<Config, String> {
    let mut seed = DEFAULT_SEED;
    let mut repetitions = DEFAULT_REPETITIONS;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // Cargo appends this harness flag when invoking `cargo bench`
            // with `harness = false`.
            "--bench" => {}
            "--seed" => {
                seed = args
                    .next()
                    .ok_or_else(|| "--seed requires an integer".to_string())?
                    .parse()
                    .map_err(|_| "--seed must be an unsigned 64-bit integer".to_string())?;
            }
            "--repetitions" => {
                repetitions = args
                    .next()
                    .ok_or_else(|| "--repetitions requires an integer".to_string())?
                    .parse()
                    .map_err(|_| "--repetitions must be a positive integer".to_string())?;
                if repetitions == 0 || repetitions > 100 {
                    return Err("--repetitions must be between 1 and 100".to_string());
                }
            }
            "--help" | "-h" => {
                println!("Usage: production [--seed N] [--repetitions N]");
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    Ok(Config { seed, repetitions })
}

fn make_inputs(seed: u64, lp_count: u32, event_count: usize) -> Vec<RemoteEvent> {
    let mut state = seed;
    (0..event_count)
        .map(|index| {
            let value = splitmix64(&mut state);
            let lp = (index as u32) % lp_count;
            RemoteEvent {
                source_lp: LpId(lp),
                dest_lp: LpId(lp),
                tick: SimTime::ZERO,
                event_payload: encode_payload(0, value),
            }
        })
        .collect()
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
        .map(|index| (LpId(index), vec![LpId((index + 1) % lp_count)]))
        .collect()
}

fn partition(lp_count: u32) -> Result<PartitionPlan, String> {
    let entities = (0..lp_count)
        .map(|index| EntityId::new(u64::from(index), 0))
        .collect();
    PartitionPlan::from_entities(lp_count, SimDuration::from_ticks(1), entities)
        .map_err(|error| format!("partition setup failed: {error:?}"))
}

fn run_sequential(lp_count: u32, inputs: &[RemoteEvent]) -> Vec<ProcessState> {
    let mut scheduler = Scheduler::new();
    let mut processes = (0..lp_count)
        .map(|_| WorkloadProcess::new(lp_count))
        .collect::<Vec<_>>();
    for event in inputs {
        scheduler.schedule(ScheduleRequest {
            at: event.tick,
            priority: 0,
            entity: Some(EntityId::new(payload_value(event), event.dest_lp.0)),
            kind: EventKind::custom(0),
        });
    }
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        let entity = event
            .entity
            .expect("benchmark events always carry an entity");
        let dest_lp = LpId(entity.generation);
        let initial = event.kind.code() == 0;
        let source_lp = if initial {
            dest_lp
        } else {
            LpId((dest_lp.0 + lp_count - 1) % lp_count)
        };
        let remote = RemoteEvent {
            source_lp,
            dest_lp,
            tick: event.at,
            event_payload: encode_payload(u8::from(!initial), entity.index),
        };
        for outbound in processes[dest_lp.0 as usize].on_event(&remote) {
            scheduler.schedule(ScheduleRequest {
                at: outbound.tick,
                priority: 0,
                entity: Some(EntityId::new(payload_value(&outbound), outbound.dest_lp.0)),
                kind: EventKind::custom(1),
            });
        }
    }
    processes.into_iter().map(|process| process.state).collect()
}

fn run_pdes(
    lp_count: u32,
    inputs: &[RemoteEvent],
) -> Result<(Vec<ProcessState>, serde_view::Counters), String> {
    let processes = (0..lp_count)
        .map(|index| (LpId(index), WorkloadProcess::new(lp_count)))
        .collect();
    let mut runtime = ConservativeRuntime::new(partition(lp_count)?, topology(lp_count), processes)
        .map_err(|error| format!("runtime setup failed: {error:?}"))?;
    for event in inputs {
        runtime
            .schedule_initial(event.clone())
            .map_err(|error| format!("initial scheduling failed: {error:?}"))?;
    }
    let report = runtime
        .run_until(SimTime::from_ticks(1))
        .map_err(|error| format!("runtime failed: {error:?}"))?;
    let states = runtime
        .processes()
        .values()
        .map(|process| process.state.clone())
        .collect();
    let counters = serde_view::Counters {
        processed: report.processed_events,
        remote_events: report.remote_events,
        emitted_events: report.emitted_events,
        null_messages: report.null_messages,
        rounds: report.rounds,
        gvt_ticks: report.gvt_history.last().map_or(0, |tick| tick.ticks()),
        worker_count: report.worker_count,
    };
    Ok((states, counters))
}

fn payload_value(event: &RemoteEvent) -> u64 {
    u64::from_le_bytes(
        event.event_payload[1..]
            .try_into()
            .expect("benchmark payload is validated at construction"),
    )
}

mod serde_view {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Counters {
        pub processed: u64,
        pub remote_events: u64,
        pub null_messages: u64,
        pub emitted_events: u64,
        pub rounds: u64,
        pub gvt_ticks: u128,
        pub worker_count: usize,
    }
}

fn elapsed_ns<F: FnOnce() -> R, R>(action: F) -> (u128, R) {
    let started = Instant::now();
    let result = action();
    (started.elapsed().as_nanos(), result)
}

fn main() {
    let config = config().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    let mut rows = Vec::new();
    for scaling in ["strong", "weak"] {
        for lp_count in LP_COUNTS {
            let event_count = if scaling == "strong" {
                STRONG_TOTAL_EVENTS
            } else {
                WEAK_EVENTS_PER_LP * lp_count as usize
            };
            let inputs = make_inputs(config.seed, lp_count, event_count);
            for _ in 0..WARMUP_RUNS {
                let _ = run_sequential(lp_count, &inputs);
                let _ = run_pdes(lp_count, &inputs).expect("benchmark warmup must complete");
            }
            let sequential_expected = run_sequential(lp_count, &inputs);
            let mut sequential_samples = Vec::with_capacity(config.repetitions);
            let mut pdes_samples = Vec::with_capacity(config.repetitions);
            let mut sequential_throughput = Vec::with_capacity(config.repetitions);
            let mut pdes_throughput = Vec::with_capacity(config.repetitions);
            let mut counters = serde_view::Counters::default();
            for sample in 0..config.repetitions {
                let (sequential_ns, sequential, pdes_ns, result) = if sample % 2 == 0 {
                    let (sequential_ns, sequential) =
                        elapsed_ns(|| run_sequential(lp_count, &inputs));
                    let (pdes_ns, result) = elapsed_ns(|| run_pdes(lp_count, &inputs));
                    (sequential_ns, sequential, pdes_ns, result)
                } else {
                    let (pdes_ns, result) = elapsed_ns(|| run_pdes(lp_count, &inputs));
                    let (sequential_ns, sequential) =
                        elapsed_ns(|| run_sequential(lp_count, &inputs));
                    (sequential_ns, sequential, pdes_ns, result)
                };
                let (observed, observed_counters) = result.expect("PDES run must complete");
                assert_eq!(
                    sequential, sequential_expected,
                    "sequential baseline drifted"
                );
                assert_eq!(observed, sequential_expected, "final state parity failed");
                sequential_samples.push(sequential_ns);
                pdes_samples.push(pdes_ns);
                let processed = (event_count * 2) as f64;
                sequential_throughput.push(processed * 1_000_000_000.0 / sequential_ns as f64);
                pdes_throughput.push(processed * 1_000_000_000.0 / pdes_ns as f64);
                counters = observed_counters;
            }
            rows.push(format!(
                "{{\"scaling\":\"{scaling}\",\"lp_count\":{lp_count},\"initial_events\":{event_count},\"expected_processed_events\":{},\"seed\":{},\"sequential_ns\":{},\"pdes_ns\":{},\"sequential_events_per_second\":{},\"pdes_events_per_second\":{},\"parity\":true,\"runtime_counters\":{{\"processed_events\":{},\"remote_events\":{},\"emitted_events\":{},\"null_messages\":{},\"rounds\":{},\"gvt_ticks\":{},\"worker_count\":{}}}}}",
                event_count * 2,
                config.seed,
                json_u128_array(&sequential_samples),
                json_u128_array(&pdes_samples),
                json_f64_array(&sequential_throughput),
                json_f64_array(&pdes_throughput),
                counters.processed,
                counters.remote_events,
                counters.emitted_events,
                counters.null_messages,
                counters.rounds,
                counters.gvt_ticks,
                counters.worker_count,
            ));
        }
    }
    println!(
        "{{\"schema_version\":\"kairoecs.pdes.benchmark.v1\",\"seed\":{},\"repetitions\":{},\"warmup_runs\":{},\"strong_total_events\":{},\"weak_events_per_lp\":{},\"rows\":[{}]}}",
        config.seed,
        config.repetitions,
        WARMUP_RUNS,
        STRONG_TOTAL_EVENTS,
        WEAK_EVENTS_PER_LP,
        rows.join(",")
    );
}

fn json_u128_array(values: &[u128]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(u128::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_f64_array(values: &[f64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    )
}
