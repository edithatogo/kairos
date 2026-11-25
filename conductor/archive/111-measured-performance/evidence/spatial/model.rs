//! Scratch Track 111 spatial agent and discrete-event model.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Write};
use std::time::Instant;

use kairo_ecs_abm::ContinuousSpatialIndex2D;
use kairo_ecs_core::Scheduler;
use kairo_ecs_pdes::{ConservativeProcess, ConservativeRuntime, LpId, PartitionPlan, RemoteEvent};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};

const REGIONS: usize = 4;
const AGENTS: usize = 96;
const MAX_COORDINATE: i16 = 15;
const MOVES: u8 = 8;
const MOVE: u8 = 0;
const SIGNAL: u8 = 1;
const SIGNAL_DELAY: u64 = 2;
const HORIZON: u64 = 4 + (MOVES as u64 - 1) * 5 + SIGNAL_DELAY;

#[derive(Clone, Copy)]
struct Start {
    x: i16,
    y: i16,
    wake: u64,
}

type Tape = [Vec<Start>; REGIONS];

fn read_tape(path: &str) -> Result<Tape, Box<dyn Error>> {
    let mut tape = Tape::default();
    for row in std::fs::read_to_string(path)?.lines() {
        let fields = row
            .split('\t')
            .map(str::parse::<i16>)
            .collect::<Result<Vec<_>, _>>()?;
        let [region, x, y, wake] = fields.as_slice() else {
            return Err("expected region, x, y, wake".into());
        };
        if !(0..=MAX_COORDINATE).contains(x)
            || !(0..=MAX_COORDINATE).contains(y)
            || !(0..=4).contains(wake)
        {
            return Err("invalid initial coordinates or wake time".into());
        }
        tape.get_mut(*region as usize)
            .ok_or("invalid region")?
            .push(Start {
                x: *x,
                y: *y,
                wake: *wake as u64,
            });
    }
    if tape.iter().any(|region| region.len() != AGENTS) {
        return Err("expected 96 agents in each of four regions".into());
    }
    Ok(tape)
}

#[derive(Clone, Copy)]
struct Agent {
    x: i16,
    y: i16,
    moves: u8,
    signals: u64,
}

#[derive(Clone, Copy)]
struct Event {
    at: u64,
    source: LpId,
    dest: LpId,
    kind: u8,
    actor: u8,
}

impl Event {
    fn remote(self) -> RemoteEvent {
        RemoteEvent {
            source_lp: self.source,
            dest_lp: self.dest,
            tick: SimTime::from_ticks(u128::from(self.at)),
            event_payload: vec![self.kind, self.actor],
        }
    }

    fn request(self) -> ScheduleRequest {
        ScheduleRequest {
            at: SimTime::from_ticks(u128::from(self.at)),
            // Match ConservativeRuntime's tick/source/insertion order within each region.
            priority: self.source.0 as i32,
            entity: Some(entity(self.dest, self.actor)),
            kind: EventKind::custom(u32::from(self.kind)),
        }
    }
}

fn entity(region: LpId, actor: u8) -> EntityId {
    EntityId::new(u64::from(actor), region.0)
}

fn initial_events(region: LpId, tape: &[Start]) -> impl Iterator<Item = Event> + '_ {
    tape.iter().enumerate().map(move |(actor, start)| Event {
        at: start.wake,
        source: region,
        dest: region,
        kind: MOVE,
        actor: actor as u8,
    })
}

struct Region {
    agents: Vec<Agent>,
    space: ContinuousSpatialIndex2D,
    trace: Vec<(Event, Agent, Option<u8>)>,
}

impl Region {
    fn new(region: LpId, tape: &[Start]) -> Self {
        let mut space = ContinuousSpatialIndex2D::new();
        let agents = tape
            .iter()
            .enumerate()
            .map(|(actor, start)| {
                space
                    .insert(
                        entity(region, actor as u8),
                        f64::from(start.x),
                        f64::from(start.y),
                    )
                    .unwrap();
                Agent {
                    x: start.x,
                    y: start.y,
                    moves: 0,
                    signals: 0,
                }
            })
            .collect();
        Self {
            agents,
            space,
            trace: Vec::new(),
        }
    }

    fn apply(&mut self, event: Event) -> Vec<Event> {
        let actor = &mut self.agents[usize::from(event.actor)];
        let mut peer = None;
        let mut emitted = Vec::new();
        match event.kind {
            MOVE => {
                let id = entity(event.dest, event.actor);
                let nearby = self
                    .space
                    .locate_within_distance(f64::from(actor.x), f64::from(actor.y), 2.0)
                    .unwrap()
                    .into_iter()
                    .map(|(other, _)| other)
                    .find(|&other| other != id);
                let [dx, dy] = if let Some(other) = nearby {
                    let (x, y) = self.space.position(other).unwrap();
                    peer = Some(other.index as u8);
                    [(x as i16 - actor.x).signum(), (y as i16 - actor.y).signum()]
                } else {
                    [0, 1]
                };
                let (dx, dy) = if dx == 0 && dy == 0 { (1, 0) } else { (dx, dy) };
                let direction = if actor.signals.is_multiple_of(2) {
                    1
                } else {
                    -1
                };
                actor.x = (actor.x + direction * dx).clamp(0, MAX_COORDINATE);
                actor.y = (actor.y + direction * dy).clamp(0, MAX_COORDINATE);
                self.space
                    .insert(id, f64::from(actor.x), f64::from(actor.y))
                    .unwrap();
                actor.moves += 1;
                if let Some(neighbor) = peer {
                    emitted.push(Event {
                        at: event.at + SIGNAL_DELAY,
                        source: event.dest,
                        dest: LpId((event.dest.0 + 1) % REGIONS as u32),
                        kind: SIGNAL,
                        actor: neighbor,
                    });
                }
                if actor.moves < MOVES {
                    emitted.push(Event {
                        at: event.at + 2 + u64::from(event.actor % 4),
                        source: event.dest,
                        ..event
                    });
                }
            }
            SIGNAL => actor.signals += 1,
            _ => unreachable!("unknown spatial event"),
        }
        self.trace.push((event, *actor, peer));
        emitted
    }
}

impl ConservativeProcess for Region {
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        let [kind, actor] = event.event_payload[..] else {
            unreachable!("invalid internal payload");
        };
        self.apply(Event {
            at: event.tick.ticks() as u64,
            source: event.source_lp,
            dest: event.dest_lp,
            kind,
            actor,
        })
        .into_iter()
        .map(Event::remote)
        .collect()
    }
}

struct Timings {
    setup: u128,
    total: u128,
    processed: u64,
    remote: u64,
    workers: usize,
}

fn setup(tape: &Tape) -> BTreeMap<LpId, Region> {
    tape.iter()
        .enumerate()
        .map(|(region, tape)| {
            let lp = LpId(region as u32);
            (lp, Region::new(lp, tape))
        })
        .collect()
}

fn serial(tape: &Tape) -> io::Result<()> {
    let clock = Instant::now();
    let mut regions = setup(tape);
    let mut scheduler = Scheduler::new();
    for (region, tape) in tape.iter().enumerate() {
        for event in initial_events(LpId(region as u32), tape) {
            scheduler.schedule(event.request());
        }
    }
    let setup = clock.elapsed().as_nanos();
    let mut remote = 0;
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        let id = event.entity.unwrap();
        let dest = LpId(id.generation);
        let decoded = Event {
            at: event.at.ticks() as u64,
            source: LpId(event.priority as u32),
            dest,
            kind: event.kind.code() as u8,
            actor: id.index as u8,
        };
        remote += u64::from(decoded.source != decoded.dest);
        for output in regions.get_mut(&dest).unwrap().apply(decoded) {
            scheduler.schedule(output.request());
        }
    }
    let total = clock.elapsed().as_nanos();
    emit(
        Timings {
            setup,
            total,
            processed: scheduler.stats().dispatched_events,
            remote,
            workers: 1,
        },
        &regions,
    )
}

fn pooled(tape: &Tape) -> Result<(), Box<dyn Error>> {
    let clock = Instant::now();
    let regions = setup(tape);
    let ids = regions.keys().map(|lp| entity(*lp, 0)).collect();
    let topology = regions
        .keys()
        .map(|&lp| (lp, vec![LpId((lp.0 + 1) % REGIONS as u32)]))
        .collect();
    let partition = PartitionPlan::from_entities(REGIONS as u32, SimDuration::from_ticks(1), ids)
        .map_err(|error| format!("invalid partition: {error:?}"))?;
    let mut runtime = ConservativeRuntime::new(partition, topology, regions)?;
    for (region, tape) in tape.iter().enumerate() {
        for event in initial_events(LpId(region as u32), tape) {
            runtime.schedule_initial(event.remote())?;
        }
    }
    let setup = clock.elapsed().as_nanos();
    let report = runtime.run_until(SimTime::from_ticks(u128::from(HORIZON)))?;
    let total = clock.elapsed().as_nanos();
    assert_eq!(runtime.pending_events(), 0);
    let timings = Timings {
        setup,
        total,
        processed: report.processed_events,
        remote: report.remote_events,
        workers: report.worker_count,
    };
    emit(timings, runtime.processes())?;
    Ok(())
}

fn emit(times: Timings, states: &BTreeMap<LpId, Region>) -> io::Result<()> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "{}\t{}\t{}\t{}\t{}\t{}",
        times.setup,
        times.total - times.setup,
        times.total,
        times.processed,
        times.remote,
        times.workers
    )?;
    for (&lp, region) in states {
        assert_eq!(region.space.len(), AGENTS);
        for &(event, agent, peer) in &region.trace {
            writeln!(
                out,
                "event\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                lp.0,
                event.at,
                event.source.0,
                event.kind,
                event.actor,
                agent.x,
                agent.y,
                agent.moves,
                agent.signals,
                peer.map_or(-1, i16::from)
            )?;
        }
        for (id, agent) in region.agents.iter().enumerate() {
            assert_eq!(
                region.space.position(entity(lp, id as u8)),
                Some((f64::from(agent.x), f64::from(agent.y)))
            );
            writeln!(
                out,
                "final\t{}\t{}\t{}\t{}\t{}\t{}",
                lp.0, id, agent.x, agent.y, agent.moves, agent.signals
            )?;
        }
    }
    out.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let [engine, tape] = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "usage: spatial111 <serial|pooled> <tape.tsv>")?;
    let tape = read_tape(&tape)?;
    match engine.as_str() {
        "serial" => Ok(serial(&tape)?),
        "pooled" => pooled(&tape),
        _ => Err("unknown executor".into()),
    }
}
