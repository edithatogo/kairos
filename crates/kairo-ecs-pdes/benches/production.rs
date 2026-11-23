use std::collections::BTreeMap;
use std::time::Instant;

use kairo_ecs_core::Scheduler;
use kairo_ecs_pdes::{ConservativeProcess, ConservativeRuntime, LpId, PartitionPlan, RemoteEvent};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};
use serde_json::json;
use sha2::{Digest, Sha256};

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
    event_work: Option<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProcessState {
    processed: u64,
    checksum: u64,
}

#[derive(Debug)]
struct WorkloadProcess {
    lp_count: u32,
    event_work: u32,
    state: ProcessState,
}

impl WorkloadProcess {
    fn new(lp_count: u32, event_work: u32) -> Self {
        Self {
            lp_count,
            event_work,
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
        let mut mixed = value;
        for _ in 0..self.event_work {
            mixed = splitmix64(&mut mixed);
        }
        self.state.processed += 1;
        let checksum = self.state.checksum.rotate_left(9).wrapping_add(mixed);
        self.state.checksum = checksum ^ event.tick.ticks() as u64 ^ u64::from(kind);

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
    let mut event_work = None;
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
            "--event-work" => {
                event_work = Some(
                    args.next()
                        .ok_or_else(|| "--event-work requires an integer".to_string())?
                        .parse()
                        .map_err(|_| {
                            "--event-work must be an unsigned 32-bit integer".to_string()
                        })?,
                );
            }
            "--help" | "-h" => {
                println!("Usage: production [--seed N] [--repetitions N] [--event-work N]");
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    Ok(Config {
        seed,
        repetitions,
        event_work,
    })
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

fn tape_sha256(inputs: &[RemoteEvent]) -> String {
    let mut hash = Sha256::new();
    for event in inputs {
        hash.update(event.source_lp.0.to_le_bytes());
        hash.update(event.dest_lp.0.to_le_bytes());
        hash.update(event.tick.ticks().to_le_bytes());
        hash.update((event.event_payload.len() as u64).to_le_bytes());
        hash.update(&event.event_payload);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

struct Phases {
    setup_ns: u128,
    dispatch_ns: u128,
}

fn run_sequential(
    lp_count: u32,
    inputs: &[RemoteEvent],
    event_work: u32,
) -> (Vec<ProcessState>, Phases) {
    let setup = Instant::now();
    let mut scheduler = Scheduler::new();
    let mut processes = (0..lp_count)
        .map(|_| WorkloadProcess::new(lp_count, event_work))
        .collect::<Vec<_>>();
    for event in inputs {
        scheduler.schedule(ScheduleRequest {
            at: event.tick,
            priority: 0,
            entity: Some(EntityId::new(payload_value(event), event.dest_lp.0)),
            kind: EventKind::custom(0),
        });
    }
    let setup_ns = setup.elapsed().as_nanos();
    let dispatch = Instant::now();
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
    let dispatch_ns = dispatch.elapsed().as_nanos();
    let states = processes.into_iter().map(|process| process.state).collect();
    (
        states,
        Phases {
            setup_ns,
            dispatch_ns,
        },
    )
}

fn run_pdes(
    lp_count: u32,
    inputs: &[RemoteEvent],
    event_work: u32,
) -> Result<(Vec<ProcessState>, serde_view::Counters, Phases), String> {
    let setup = Instant::now();
    let processes = (0..lp_count)
        .map(|index| (LpId(index), WorkloadProcess::new(lp_count, event_work)))
        .collect();
    let mut runtime = ConservativeRuntime::new(partition(lp_count)?, topology(lp_count), processes)
        .map_err(|error| format!("runtime setup failed: {error:?}"))?;
    for event in inputs {
        runtime
            .schedule_initial(event.clone())
            .map_err(|error| format!("initial scheduling failed: {error:?}"))?;
    }
    let setup_ns = setup.elapsed().as_nanos();
    let dispatch = Instant::now();
    let report = runtime
        .run_until(SimTime::from_ticks(1))
        .map_err(|error| format!("runtime failed: {error:?}"))?;
    let dispatch_ns = dispatch.elapsed().as_nanos();
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
    Ok((
        states,
        counters,
        Phases {
            setup_ns,
            dispatch_ns,
        },
    ))
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
    let event_work = config.event_work.unwrap_or(0);
    let (profiles, lp_counts): (&[&str], &[u32]) = if config.event_work.is_some() {
        (&["strong"], &[4])
    } else {
        (&["strong", "weak"], &LP_COUNTS)
    };
    let mut rows = Vec::new();
    for &scaling in profiles {
        for &lp_count in lp_counts {
            let event_count = if scaling == "strong" {
                STRONG_TOTAL_EVENTS
            } else {
                WEAK_EVENTS_PER_LP * lp_count as usize
            };
            let inputs = make_inputs(config.seed, lp_count, event_count);
            let sequential = || run_sequential(lp_count, &inputs, event_work);
            let parallel = || run_pdes(lp_count, &inputs, event_work);
            for _ in 0..WARMUP_RUNS {
                let _ = sequential();
                let _ = parallel().expect("benchmark warmup must complete");
            }
            let (sequential_expected, _) = sequential();
            let mut sequential_samples = Vec::with_capacity(config.repetitions);
            let mut pdes_samples = Vec::with_capacity(config.repetitions);
            let mut sequential_setup = Vec::with_capacity(config.repetitions);
            let mut pdes_setup = Vec::with_capacity(config.repetitions);
            let mut sequential_dispatch = Vec::with_capacity(config.repetitions);
            let mut pdes_dispatch = Vec::with_capacity(config.repetitions);
            let mut sequential_throughput = Vec::with_capacity(config.repetitions);
            let mut pdes_throughput = Vec::with_capacity(config.repetitions);
            let mut counters = serde_view::Counters::default();
            for sample in 0..config.repetitions {
                let (sequential_ns, sequential, pdes_ns, result) = if sample % 2 == 0 {
                    let (sequential_ns, sequential) = elapsed_ns(sequential);
                    let (pdes_ns, result) = elapsed_ns(parallel);
                    (sequential_ns, sequential, pdes_ns, result)
                } else {
                    let (pdes_ns, result) = elapsed_ns(parallel);
                    let (sequential_ns, sequential) = elapsed_ns(sequential);
                    (sequential_ns, sequential, pdes_ns, result)
                };
                let (sequential, sequential_phases) = sequential;
                let (observed, observed_counters, pdes_phases) =
                    result.expect("PDES run must complete");
                assert_eq!(
                    sequential, sequential_expected,
                    "sequential baseline drifted"
                );
                assert_eq!(observed, sequential_expected, "final state parity failed");
                sequential_samples.push(sequential_ns);
                pdes_samples.push(pdes_ns);
                sequential_setup.push(sequential_phases.setup_ns);
                pdes_setup.push(pdes_phases.setup_ns);
                sequential_dispatch.push(sequential_phases.dispatch_ns);
                pdes_dispatch.push(pdes_phases.dispatch_ns);
                let processed = (event_count * 2) as f64;
                sequential_throughput.push(processed * 1_000_000_000.0 / sequential_ns as f64);
                pdes_throughput.push(processed * 1_000_000_000.0 / pdes_ns as f64);
                counters = observed_counters;
            }
            rows.push(json!({
                "scaling": scaling,
                "lp_count": lp_count,
                "initial_events": event_count,
                "expected_processed_events": event_count * 2,
                "seed": config.seed,
                "event_work": event_work,
                "input_tape_sha256": tape_sha256(&inputs),
                "ordered_state": sequential_expected.iter().map(|state| [state.processed, state.checksum]).collect::<Vec<_>>(),
                "sequential_ns": sequential_samples,
                "pdes_ns": pdes_samples,
                "sequential_setup_ns": sequential_setup,
                "pdes_setup_ns": pdes_setup,
                "sequential_dispatch_ns": sequential_dispatch,
                "pdes_dispatch_ns": pdes_dispatch,
                "sequential_events_per_second": sequential_throughput,
                "pdes_events_per_second": pdes_throughput,
                "parity": true,
                "runtime_counters": {
                    "processed_events": counters.processed,
                    "remote_events": counters.remote_events,
                    "emitted_events": counters.emitted_events,
                    "null_messages": counters.null_messages,
                    "rounds": counters.rounds,
                    "gvt_ticks": counters.gvt_ticks,
                    "worker_count": counters.worker_count,
                },
            }));
        }
    }
    let schema = if config.event_work.is_some() {
        "kairoecs.pdes.crossover.v1"
    } else {
        "kairoecs.pdes.benchmark.v1"
    };
    println!(
        "{}",
        json!({
            "schema_version": schema,
            "seed": config.seed,
            "repetitions": config.repetitions,
            "warmup_runs": WARMUP_RUNS,
            "strong_total_events": STRONG_TOTAL_EVENTS,
            "weak_events_per_lp": WEAK_EVENTS_PER_LP,
            "rows": rows,
        })
    );
}
