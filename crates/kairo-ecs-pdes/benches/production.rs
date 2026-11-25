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
const PHOLD_INITIAL_EVENTS: usize = 256;
const PHOLD_GENERATIONS: u8 = 8;

#[derive(Clone, Copy, Debug)]
enum Workload {
    Ring,
    Phold {
        lookahead: u128,
        remote_percent: u64,
    },
}

impl Workload {
    fn lookahead(self) -> SimDuration {
        SimDuration::from_ticks(match self {
            Self::Ring => 1,
            Self::Phold { lookahead, .. } => lookahead,
        })
    }

    fn generations(self) -> u8 {
        match self {
            Self::Ring => 2,
            Self::Phold { .. } => PHOLD_GENERATIONS,
        }
    }

    fn horizon(self) -> SimTime {
        SimTime::from_ticks(match self {
            Self::Ring => 1,
            Self::Phold { lookahead, .. } => (lookahead + 3) * u128::from(PHOLD_GENERATIONS - 1),
        })
    }

    fn successor(self, current: LpId, lp_count: u32, value: u64) -> (LpId, SimDuration, u64) {
        match self {
            Self::Ring => (LpId((current.0 + 1) % lp_count), self.lookahead(), value),
            Self::Phold {
                lookahead,
                remote_percent,
            } => {
                let mut rng = value;
                let remote = splitmix64(&mut rng) % 100 < remote_percent;
                let offset = 1 + (splitmix64(&mut rng) % u64::from(lp_count - 1)) as u32;
                let destination = if remote {
                    LpId((current.0 + offset) % lp_count)
                } else {
                    current
                };
                let delay = lookahead + u128::from(splitmix64(&mut rng) % 4);
                (
                    destination,
                    SimDuration::from_ticks(delay),
                    splitmix64(&mut rng),
                )
            }
        }
    }

    fn serial_request(self, event: &RemoteEvent) -> ScheduleRequest {
        ScheduleRequest {
            at: event.tick,
            // PDES orders each destination's ties by source and then insertion.
            // Strictly positive delays preserve each source's emission order.
            priority: match self {
                Self::Ring => 0,
                Self::Phold { .. } => event.source_lp.0 as i32,
            },
            entity: Some(EntityId::new(payload_value(event), event.dest_lp.0)),
            kind: EventKind::custom(u32::from(event.event_payload[0])),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Config {
    seed: u64,
    repetitions: usize,
    event_work: Option<u32>,
    phold: bool,
    executor: Option<Executor>,
}

#[derive(Clone, Copy, Debug)]
enum Executor {
    Serial,
    Pooled,
}

impl Executor {
    fn name(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::Pooled => "pooled",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProcessState {
    processed: u64,
    checksum: u64,
    remote_emitted: u64,
}

#[derive(Debug)]
struct WorkloadProcess {
    lp_count: u32,
    event_work: u32,
    workload: Workload,
    state: ProcessState,
}

impl WorkloadProcess {
    fn new(lp_count: u32, event_work: u32, workload: Workload) -> Self {
        Self {
            lp_count,
            event_work,
            workload,
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

        if kind + 1 < self.workload.generations() {
            let (dest_lp, delay, next_value) =
                self.workload.successor(event.dest_lp, self.lp_count, value);
            self.state.remote_emitted += u64::from(dest_lp != event.dest_lp);
            vec![RemoteEvent {
                source_lp: event.dest_lp,
                dest_lp,
                tick: event.tick.saturating_add(delay),
                event_payload: encode_payload(kind + 1, next_value),
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
    let mut phold = false;
    let mut executor = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // Cargo appends this harness flag when invoking `cargo bench`
            // with `harness = false`.
            "--bench" => {}
            "--phold" => phold = true,
            "--executor" => {
                executor = Some(match args.next().as_deref() {
                    Some("serial") => Executor::Serial,
                    Some("pooled") if cfg!(feature = "pdes-worker-pool") => Executor::Pooled,
                    Some("pooled") => return Err("pooled requires pdes-worker-pool".to_string()),
                    _ => return Err("--executor requires serial or pooled".to_string()),
                });
            }
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
                println!(
                    "Usage: production [--seed N] [--repetitions N] [--event-work N] [--phold [--executor serial|pooled]]"
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    if executor.is_some() && !phold {
        return Err("--executor requires --phold".to_string());
    }
    Ok(Config {
        seed,
        repetitions,
        event_work,
        phold,
        executor,
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

fn topology(lp_count: u32, workload: Workload) -> BTreeMap<LpId, Vec<LpId>> {
    (0..lp_count)
        .map(|index| {
            let neighbors = match workload {
                Workload::Ring => vec![LpId((index + 1) % lp_count)],
                Workload::Phold { .. } => {
                    (0..lp_count).filter(|&lp| lp != index).map(LpId).collect()
                }
            };
            (LpId(index), neighbors)
        })
        .collect()
}

fn partition(lp_count: u32, workload: Workload) -> Result<PartitionPlan, String> {
    let entities = (0..lp_count)
        .map(|index| EntityId::new(u64::from(index), 0))
        .collect();
    PartitionPlan::from_entities(lp_count, workload.lookahead(), entities)
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
    workload: Workload,
) -> (Vec<ProcessState>, Phases) {
    let setup = Instant::now();
    let mut scheduler = Scheduler::new();
    let mut processes = (0..lp_count)
        .map(|_| WorkloadProcess::new(lp_count, event_work, workload))
        .collect::<Vec<_>>();
    for event in inputs {
        scheduler.schedule(workload.serial_request(event));
    }
    let setup_ns = setup.elapsed().as_nanos();
    let dispatch = Instant::now();
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        let entity = event
            .entity
            .expect("benchmark events always carry an entity");
        let dest_lp = LpId(entity.generation);
        let kind = event.kind.code() as u8;
        let source_lp = match workload {
            Workload::Ring if kind == 0 => dest_lp,
            Workload::Ring => LpId((dest_lp.0 + lp_count - 1) % lp_count),
            Workload::Phold { .. } => LpId(event.priority as u32),
        };
        let remote = RemoteEvent {
            source_lp,
            dest_lp,
            tick: event.at,
            event_payload: encode_payload(kind, entity.index),
        };
        for outbound in processes[dest_lp.0 as usize].on_event(&remote) {
            scheduler.schedule(workload.serial_request(&outbound));
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
    workload: Workload,
) -> Result<(Vec<ProcessState>, serde_view::Counters, Phases), String> {
    let setup = Instant::now();
    let processes = (0..lp_count)
        .map(|index| {
            (
                LpId(index),
                WorkloadProcess::new(lp_count, event_work, workload),
            )
        })
        .collect();
    let mut runtime = ConservativeRuntime::new(
        partition(lp_count, workload)?,
        topology(lp_count, workload),
        processes,
    )
    .map_err(|error| format!("runtime setup failed: {error:?}"))?;
    for event in inputs {
        runtime
            .schedule_initial(event.clone())
            .map_err(|error| format!("initial scheduling failed: {error:?}"))?;
    }
    let setup_ns = setup.elapsed().as_nanos();
    let dispatch = Instant::now();
    let report = runtime
        .run_until(workload.horizon())
        .map_err(|error| format!("runtime failed: {error:?}"))?;
    let dispatch_ns = dispatch.elapsed().as_nanos();
    assert_eq!(runtime.pending_events(), 0, "benchmark left scheduled work");
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
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

fn selected_phold(
    executor: Executor,
    config: Config,
    lp_count: u32,
    inputs: &[RemoteEvent],
    workload: Workload,
) -> serde_json::Value {
    let Workload::Phold {
        lookahead,
        remote_percent,
    } = workload
    else {
        unreachable!("selected executor requires PHOLD");
    };
    let event_work = config.event_work.unwrap_or(0);
    let run = || match executor {
        Executor::Serial => {
            let (states, phases) = run_sequential(lp_count, inputs, event_work, workload);
            (states, phases, 1)
        }
        Executor::Pooled => {
            let (states, counters, phases) = run_pdes(lp_count, inputs, event_work, workload)
                .expect("pooled PHOLD must complete");
            (states, phases, counters.worker_count)
        }
    };
    let (first_ns, (expected, _, worker_count)) = elapsed_ns(run);
    let processed = expected.iter().map(|state| state.processed).sum::<u64>();
    assert_eq!(
        processed,
        (inputs.len() * usize::from(workload.generations())) as u64
    );
    let timed = (0..config.repetitions)
        .map(|_| {
            let (total_ns, (states, phases, workers)) = elapsed_ns(run);
            assert_eq!(states, expected, "selected executor state drifted");
            assert_eq!(
                workers, worker_count,
                "selected executor worker count drifted"
            );
            json!({
                "total_ns": total_ns,
                "setup_ns": phases.setup_ns,
                "dispatch_ns": phases.dispatch_ns,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "lp_count": lp_count,
        "lookahead_ticks": lookahead,
        "remote_percent_target": remote_percent,
        "event_work": event_work,
        "input_tape_sha256": tape_sha256(inputs),
        "ordered_state": expected.iter().map(|state| [state.processed, state.checksum]).collect::<Vec<_>>(),
        "processed_events": processed,
        "worker_count": worker_count,
        "first_run_ns": first_ns,
        "timed": timed,
    })
}

fn main() {
    let config = config().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    let event_work = config.event_work.unwrap_or(0);
    let cases = if config.phold {
        [(4, 1, 25), (4, 1, 75), (4, 8, 75), (8, 8, 75)]
            .into_iter()
            .chain(config.executor.map(|_| (32, 8, 75)))
            .map(|(lp_count, lookahead, remote_percent)| {
                (
                    "strong",
                    lp_count,
                    Workload::Phold {
                        lookahead,
                        remote_percent,
                    },
                )
            })
            .collect::<Vec<_>>()
    } else if config.event_work.is_some() {
        vec![("strong", 4, Workload::Ring)]
    } else {
        ["strong", "weak"]
            .into_iter()
            .flat_map(|scaling| LP_COUNTS.map(|lp_count| (scaling, lp_count, Workload::Ring)))
            .collect()
    };
    let mut rows = Vec::new();
    for (scaling, lp_count, workload) in cases {
        let event_count = match workload {
            Workload::Phold { .. } => PHOLD_INITIAL_EVENTS,
            Workload::Ring if scaling == "strong" => STRONG_TOTAL_EVENTS,
            Workload::Ring => WEAK_EVENTS_PER_LP * lp_count as usize,
        };
        let inputs = make_inputs(config.seed, lp_count, event_count);
        if let Some(executor) = config.executor {
            rows.push(selected_phold(
                executor, config, lp_count, &inputs, workload,
            ));
            continue;
        }
        let sequential = || run_sequential(lp_count, &inputs, event_work, workload);
        let parallel = || run_pdes(lp_count, &inputs, event_work, workload);
        let (sequence_warmup_ns, (sequential_expected, sequence_warmup)) = elapsed_ns(sequential);
        let (parallel_warmup_ns, warmup_result) = elapsed_ns(parallel);
        let (warmup_state, counters, parallel_warmup) =
            warmup_result.expect("benchmark warmup must complete");
        assert_eq!(
            warmup_state, sequential_expected,
            "warmup state parity failed"
        );
        let expected_processed = (event_count * usize::from(workload.generations())) as u64;
        assert_eq!(counters.processed, expected_processed);
        assert_eq!(
            counters.emitted_events,
            expected_processed - event_count as u64
        );
        assert_eq!(
            counters.remote_events,
            sequential_expected
                .iter()
                .map(|state| state.remote_emitted)
                .sum::<u64>()
        );
        let mut sequential_samples = Vec::with_capacity(config.repetitions);
        let mut pdes_samples = Vec::with_capacity(config.repetitions);
        let mut sequential_setup = Vec::with_capacity(config.repetitions);
        let mut pdes_setup = Vec::with_capacity(config.repetitions);
        let mut sequential_dispatch = Vec::with_capacity(config.repetitions);
        let mut pdes_dispatch = Vec::with_capacity(config.repetitions);
        let mut sequential_throughput = Vec::with_capacity(config.repetitions);
        let mut pdes_throughput = Vec::with_capacity(config.repetitions);
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
            assert_eq!(observed_counters, counters, "runtime counters drifted");
            sequential_samples.push(sequential_ns);
            pdes_samples.push(pdes_ns);
            sequential_setup.push(sequential_phases.setup_ns);
            pdes_setup.push(pdes_phases.setup_ns);
            sequential_dispatch.push(sequential_phases.dispatch_ns);
            pdes_dispatch.push(pdes_phases.dispatch_ns);
            let processed = expected_processed as f64;
            sequential_throughput.push(processed * 1_000_000_000.0 / sequential_ns as f64);
            pdes_throughput.push(processed * 1_000_000_000.0 / pdes_ns as f64);
        }
        let mut row = json!({
            "scaling": scaling,
            "lp_count": lp_count,
            "initial_events": event_count,
            "expected_processed_events": expected_processed,
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
        });
        if let Workload::Phold {
            lookahead,
            remote_percent,
        } = workload
        {
            row["workload"] = json!("phold");
            row["generations"] = json!(PHOLD_GENERATIONS);
            row["lookahead_ticks"] = json!(lookahead);
            row["horizon_ticks"] = json!(workload.horizon().ticks());
            row["remote_percent_target"] = json!(remote_percent);
            row["first_run"] = json!({
                "sequential_ns": sequence_warmup_ns,
                "sequential_setup_ns": sequence_warmup.setup_ns,
                "sequential_dispatch_ns": sequence_warmup.dispatch_ns,
                "pdes_ns": parallel_warmup_ns,
                "pdes_setup_ns": parallel_warmup.setup_ns,
                "pdes_dispatch_ns": parallel_warmup.dispatch_ns,
            });
        }
        rows.push(row);
    }
    if let Some(executor) = config.executor {
        println!(
            "{}",
            json!({
                "schema_version": "kairoecs.pdes.phold.selected.v1",
                "executor": executor.name(),
                "seed": config.seed,
                "repetitions": config.repetitions,
                "warmup_runs": WARMUP_RUNS,
                "rows": rows,
            })
        );
        return;
    }
    let schema = if config.phold {
        "kairoecs.pdes.phold.v1"
    } else if config.event_work.is_some() {
        "kairoecs.pdes.crossover.v1"
    } else {
        "kairoecs.pdes.benchmark.v1"
    };
    let executor = if cfg!(feature = "pdes-worker-pool") {
        "pooled"
    } else {
        "scoped"
    };
    let mut evidence = json!({
        "schema_version": schema,
        "pdes_executor": executor,
        "seed": config.seed,
        "repetitions": config.repetitions,
        "warmup_runs": WARMUP_RUNS,
        "rows": rows,
    });
    if !config.phold {
        evidence["strong_total_events"] = json!(STRONG_TOTAL_EVENTS);
        evidence["weak_events_per_lp"] = json!(WEAK_EVENTS_PER_LP);
    }
    println!("{evidence}");
}
