//! Held-out two-station FIFO experiment. See Track 111 evidence for the tape and oracle.

use std::collections::VecDeque;
use std::error::Error;
use std::time::Instant;

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use serde_json::{json, Value};

const STATIONS: usize = 2;
const FINISH: u32 = 0;
const DEADLINE: u32 = 1;
const ARRIVE: u32 = 2;

#[derive(Clone, Copy)]
struct Arrival {
    tick: u64,
    service: u64,
    patience: u64,
}

type Tape = [Vec<Arrival>; STATIONS];

fn read_tape(path: &str) -> Result<Tape, Box<dyn Error>> {
    let mut tape = Tape::default();
    for (index, line) in std::fs::read_to_string(path)?.lines().enumerate() {
        let values = line
            .split('\t')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        let [station, tick, service, patience] = values.as_slice() else {
            return Err(format!("line {} must have four columns", index + 1).into());
        };
        let arrivals = tape.get_mut(*station as usize).ok_or("invalid station")?;
        let tick = u64::from(*tick);
        if *service == 0 || *patience == 0 || arrivals.last().is_some_and(|last| last.tick > tick) {
            return Err(format!(
                "invalid service, patience or arrival order at line {}",
                index + 1
            )
            .into());
        }
        arrivals.push(Arrival {
            tick,
            service: u64::from(*service),
            patience: u64::from(*patience),
        });
    }
    if tape.iter().any(Vec::is_empty) {
        return Err("both stations require arrivals".into());
    }
    Ok(tape)
}

#[derive(Clone, Copy)]
struct Event {
    tick: u64,
    kind: u32,
    customer: usize,
}

struct Action {
    event: Event,
    cancel_patience: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    New,
    Waiting,
    Serving,
    Finished,
    Abandoned,
}

#[derive(Clone, Copy)]
struct Customer {
    status: Status,
    start: Option<u64>,
    exit: Option<u64>,
    patience_cancelled: bool,
}

type TraceRow = (u64, &'static str, usize, Option<usize>, Vec<usize>);

struct Station {
    arrivals: Vec<Arrival>,
    customers: Vec<Customer>,
    active: Option<usize>,
    waiting: VecDeque<usize>,
    trace: Vec<TraceRow>,
    ignored_deadlines: usize,
}

impl Station {
    fn new(arrivals: Vec<Arrival>) -> Self {
        let customers = vec![
            Customer {
                status: Status::New,
                start: None,
                exit: None,
                patience_cancelled: false,
            };
            arrivals.len()
        ];
        Self {
            arrivals,
            customers,
            active: None,
            waiting: VecDeque::new(),
            trace: Vec::new(),
            ignored_deadlines: 0,
        }
    }

    fn trace(&mut self, tick: u64, label: &'static str, customer: usize) {
        self.trace.push((
            tick,
            label,
            customer,
            self.active,
            self.waiting.iter().copied().collect(),
        ));
    }

    fn start(&mut self, tick: u64, customer: usize, cancel_patience: bool) -> Action {
        assert!(self.active.replace(customer).is_none());
        let state = &mut self.customers[customer];
        state.status = Status::Serving;
        state.start = Some(tick);
        state.patience_cancelled = cancel_patience;
        self.trace(tick, "start", customer);
        Action {
            event: Event {
                tick: tick + self.arrivals[customer].service,
                kind: FINISH,
                customer,
            },
            cancel_patience,
        }
    }

    fn apply(&mut self, event: Event) -> Option<Action> {
        let Event {
            tick,
            kind,
            customer,
        } = event;
        match kind {
            ARRIVE => {
                assert!(self.customers[customer].status == Status::New);
                self.trace(tick, "arrive", customer);
                if self.active.is_none() {
                    return Some(self.start(tick, customer, false));
                }
                self.customers[customer].status = Status::Waiting;
                self.waiting.push_back(customer);
                self.trace(tick, "queue", customer);
                Some(Action {
                    event: Event {
                        tick: tick + self.arrivals[customer].patience,
                        kind: DEADLINE,
                        customer,
                    },
                    cancel_patience: false,
                })
            }
            FINISH => {
                assert_eq!(self.active.take(), Some(customer));
                self.customers[customer].status = Status::Finished;
                self.customers[customer].exit = Some(tick);
                self.trace(tick, "finish", customer);
                self.waiting
                    .pop_front()
                    .map(|next| self.start(tick, next, true))
            }
            DEADLINE => {
                let state = &mut self.customers[customer];
                if state.patience_cancelled {
                    self.ignored_deadlines += 1;
                } else {
                    assert!(state.status == Status::Waiting);
                    let index = self.waiting.iter().position(|&id| id == customer).unwrap();
                    self.waiting.remove(index);
                    state.status = Status::Abandoned;
                    state.exit = Some(tick);
                    self.trace(tick, "abandon", customer);
                }
                None
            }
            _ => unreachable!("unknown queue event"),
        }
    }

    fn result(&self) -> Value {
        let customers: Vec<_> = self
            .customers
            .iter()
            .map(|state| {
                let status = match state.status {
                    Status::Finished => "finished",
                    Status::Abandoned => "abandoned",
                    _ => panic!("customer did not reach a terminal state"),
                };
                (status, state.start, state.exit, state.patience_cancelled)
            })
            .collect();
        assert!(self.active.is_none() && self.waiting.is_empty());
        json!({
            "trace": self.trace,
            "customers": customers,
            "active": self.active,
            "waiting": self.waiting,
        })
    }
}

fn request(station: usize, event: Event) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(u128::from(event.tick)),
        priority: event.kind as i32,
        entity: Some(EntityId::new(event.customer as u64, station as u32)),
        kind: EventKind::custom(event.kind),
    }
}

fn run_serial(tape: Tape) -> Value {
    let start = Instant::now();
    let mut scheduler = Scheduler::new();
    let mut stations = tape.map(Station::new);
    let mut deadlines = stations
        .each_ref()
        .map(|model| vec![None; model.arrivals.len()]);
    for (station, model) in stations.iter().enumerate() {
        for (customer, arrival) in model.arrivals.iter().enumerate() {
            scheduler.schedule(request(
                station,
                Event {
                    tick: arrival.tick,
                    kind: ARRIVE,
                    customer,
                },
            ));
        }
    }
    let setup_ns = start.elapsed().as_nanos();
    while let StepOutcome::Dispatched(dispatch) = scheduler.step() {
        let entity = dispatch.entity.expect("event has a customer");
        let station = entity.generation as usize;
        let event = Event {
            tick: u64::try_from(dispatch.at.ticks()).expect("tick fits u64"),
            kind: dispatch.kind.code(),
            customer: entity.index as usize,
        };
        if event.kind == DEADLINE {
            assert_eq!(deadlines[station][event.customer].take(), Some(dispatch.id));
        }
        if let Some(action) = stations[station].apply(event) {
            let deadline = &mut deadlines[station][action.event.customer];
            if action.cancel_patience {
                assert!(scheduler.cancel(deadline.take().expect("waiting customer has a timer")));
            }
            let id = scheduler.schedule(request(station, action.event));
            if action.event.kind == DEADLINE {
                assert!(deadline.replace(id).is_none());
            }
        }
    }
    let total_ns = start.elapsed().as_nanos();
    let stats = scheduler.stats();
    assert_eq!(stats.pending_events, 0);
    assert!(deadlines.iter().flatten().all(Option::is_none));
    assert!(stations.iter().all(|model| model.ignored_deadlines == 0));
    json!({
        "result": stations.each_ref().map(Station::result),
        "metrics": {"setup_ns": setup_ns, "dispatch_ns": total_ns - setup_ns, "total_ns": total_ns},
        "diagnostics": {
            "scheduled": stats.scheduled_events,
            "dispatched": stats.dispatched_events,
            "cancelled_timers": stats.cancelled_events,
        },
    })
}

#[cfg(feature = "pdes")]
mod parallel {
    use std::collections::BTreeMap;

    use kairo_ecs_pdes::{
        ConservativeProcess, ConservativeRuntime, LpId, PartitionPlan, RemoteEvent,
    };
    use kairo_ecs_types::SimDuration;

    use super::*;

    // ConservativeRuntime has no priority field. Three phases preserve the model's
    // same-tick policy, with strictly positive lookahead for all follow-up events.
    fn remote(lp: LpId, event: Event) -> RemoteEvent {
        RemoteEvent {
            source_lp: lp,
            dest_lp: lp,
            tick: SimTime::from_ticks(u128::from(event.tick) * 3 + u128::from(event.kind)),
            event_payload: (event.customer as u64).to_le_bytes().to_vec(),
        }
    }

    impl ConservativeProcess for Station {
        fn on_event(&mut self, remote_event: &RemoteEvent) -> Vec<RemoteEvent> {
            let phase_tick = remote_event.tick.ticks();
            let event = Event {
                tick: u64::try_from(phase_tick / 3).expect("tick fits u64"),
                kind: (phase_tick % 3) as u32,
                customer: u64::from_le_bytes(
                    remote_event
                        .event_payload
                        .as_slice()
                        .try_into()
                        .expect("customer payload"),
                ) as usize,
            };
            // A model-cancelled deadline stays in this runtime's queue. Its
            // eventual dispatch has no effect on state or the observable trace.
            self.apply(event)
                .map(|action| remote(remote_event.dest_lp, action.event))
                .into_iter()
                .collect()
        }
    }

    pub(super) fn run(tape: Tape) -> Result<Value, Box<dyn Error>> {
        let start = Instant::now();
        let mut arrivals = Vec::new();
        let mut horizon = 0;
        let mut states = BTreeMap::new();
        let mut topology = BTreeMap::new();
        let mut entities = Vec::new();
        for (station, input) in tape.into_iter().enumerate() {
            let lp = LpId(station as u32);
            topology.insert(lp, Vec::new());
            entities.push(EntityId::new(station as u64, 0));
            for (customer, arrival) in input.iter().enumerate() {
                arrivals.push(remote(
                    lp,
                    Event {
                        tick: arrival.tick,
                        kind: ARRIVE,
                        customer,
                    },
                ));
                horizon = horizon.max(arrival.tick + arrival.service + arrival.patience);
            }
            states.insert(lp, Station::new(input));
        }
        let partition =
            PartitionPlan::from_entities(STATIONS as u32, SimDuration::from_ticks(1), entities)
                .map_err(|error| format!("invalid partition: {error:?}"))?;
        let mut runtime = ConservativeRuntime::new(partition, topology, states)?;
        for arrival in arrivals {
            runtime.schedule_initial(arrival)?;
        }
        let setup_ns = start.elapsed().as_nanos();
        let report = runtime.run_until(SimTime::from_ticks(u128::from(horizon) * 3 + 2))?;
        let total_ns = start.elapsed().as_nanos();
        assert_eq!(runtime.pending_events(), 0);
        let results: Vec<_> = runtime.processes().values().map(Station::result).collect();
        let ignored_deadlines: usize = runtime
            .processes()
            .values()
            .map(|state| state.ignored_deadlines)
            .sum();
        Ok(json!({
            "result": results,
            "metrics": {"setup_ns": setup_ns, "dispatch_ns": total_ns - setup_ns, "total_ns": total_ns},
            "diagnostics": {
                "dispatched": report.processed_events,
                "remote_events": report.remote_events,
                "worker_count": report.worker_count,
                "pooled": cfg!(feature = "pdes-worker-pool"),
                "ignored_timers": ignored_deadlines,
            },
        }))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let [executor, tape] = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "usage: track111_queueing <serial|pdes> <tape.tsv>")?;
    let tape = read_tape(&tape)?;
    let output = match executor.as_str() {
        "serial" => run_serial(tape),
        #[cfg(feature = "pdes")]
        "pdes" => parallel::run(tape)?,
        #[cfg(not(feature = "pdes"))]
        "pdes" => return Err("pdes executor requires --features pdes".into()),
        _ => return Err(format!("unknown executor: {executor}").into()),
    };
    println!("{output}");
    Ok(())
}
