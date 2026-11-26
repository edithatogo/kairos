#![cfg(feature = "engine-checkpoint")]

use std::collections::BTreeMap;

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::{DeterministicStream, RngCheckpoint};
use kairo_ecs_state::{EntitySnapshot, World};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime};

const SERVICE: EventKind = EventKind::custom(1);
const ROUTING: EventKind = EventKind::custom(2);
const A: EntityId = EntityId::new(0, 0);
const B: EntityId = EntityId::new(1, 0);
const SUCCESSOR: EntityId = EntityId::new(1, 1);
const ROOT_SEED: u64 = 78121;

type Address = (EntityId, &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Draw {
    event: DispatchedEvent,
    word: u128,
    raw: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Checkpoint {
    scheduler: Vec<u8>,
    world: Vec<u8>,
    streams: BTreeMap<Address, RngCheckpoint>,
    services: BTreeMap<EntityId, u32>,
    draws: Vec<Draw>,
}

struct Run {
    scheduler: Scheduler,
    world: World,
    streams: BTreeMap<Address, DeterministicStream>,
    services: BTreeMap<EntityId, u32>,
    draws: Vec<Draw>,
}

impl Run {
    fn initial() -> (Self, EventId) {
        let mut world = World::new();
        assert_eq!(world.spawn(), A);
        assert_eq!(world.spawn(), B);
        let mut run = Self {
            scheduler: Scheduler::new(),
            world,
            streams: BTreeMap::new(),
            services: BTreeMap::from([(A, 0), (B, 0)]),
            draws: Vec::new(),
        };
        run.schedule(2, A, SERVICE);
        run.schedule(2, B, SERVICE);
        run.schedule(5, A, SERVICE);
        let cancel = run.schedule(5, B, SERVICE);
        (run, cancel)
    }

    fn schedule(&mut self, tick: u128, entity: EntityId, kind: EventKind) -> EventId {
        self.scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(tick),
            priority: 0,
            entity: Some(entity),
            kind,
        })
    }

    fn replace_b(&mut self, pending_service: EventId) {
        assert!(self.scheduler.cancel(pending_service));
        assert!(self.world.despawn(B));
        assert_eq!(self.services.remove(&B), Some(1));
        let successor = self.world.spawn();
        assert_eq!(successor, SUCCESSOR);
        self.services.insert(successor, 0);
        self.schedule(4, successor, ROUTING);
        self.schedule(5, successor, SERVICE);
    }

    fn through(&mut self, tick: u128, seed: u64, mut global: Option<&mut DeterministicStream>) {
        while self
            .scheduler
            .peek_next()
            .is_some_and(|next| next.at.ticks() <= tick)
        {
            let event = self.scheduler.step().into_dispatched().unwrap();
            let entity = event.entity.unwrap();
            assert!(self.world.is_alive(entity), "stale dispatch: {event:?}");
            let purpose = match event.kind {
                SERVICE => {
                    *self.services.get_mut(&entity).unwrap() += 1;
                    "service"
                }
                ROUTING => "routing",
                kind => panic!("unexpected event kind: {kind:?}"),
            };
            let stream = match global.as_deref_mut() {
                Some(stream) => stream,
                None => self.streams.entry((entity, purpose)).or_insert_with(|| {
                    DeterministicStream::from_entity_purpose(seed, entity, purpose)
                }),
            };
            let word = stream.word_pos();
            let raw = stream.next_u64();
            self.draws.push(Draw { event, word, raw });
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            scheduler: self.scheduler.checkpoint_json().unwrap(),
            world: self.world.checkpoint_json().unwrap(),
            streams: self
                .streams
                .iter()
                .map(|(&address, stream)| (address, stream.checkpoint()))
                .collect(),
            services: self.services.clone(),
            draws: self.draws.clone(),
        }
    }

    fn restore(saved: &Checkpoint) -> Self {
        Self {
            scheduler: Scheduler::from_checkpoint_json(&saved.scheduler).unwrap(),
            world: World::from_checkpoint_json(&saved.world).unwrap(),
            streams: saved
                .streams
                .iter()
                .map(|(&address, checkpoint)| {
                    (address, DeterministicStream::restore(checkpoint).unwrap())
                })
                .collect(),
            services: saved.services.clone(),
            draws: saved.draws.clone(),
        }
    }

    fn replay(seed: u64, replace_b: bool, global: Option<&mut DeterministicStream>) -> Self {
        let (mut run, cancel) = Self::initial();
        let mut global = global;
        run.through(2, seed, global.as_deref_mut());
        if replace_b {
            run.replace_b(cancel);
        }
        run.through(5, seed, global);
        run
    }

    fn tape(&self) -> Vec<(u128, EntityId, EventKind, u128)> {
        self.draws
            .iter()
            .map(|row| {
                (
                    row.event.at.ticks(),
                    row.event.entity.unwrap(),
                    row.event.kind,
                    row.word,
                )
            })
            .collect()
    }
}

fn verify_replicate(replicate: u64) {
    let seed = DeterministicStream::from_seed_u64(ROOT_SEED, replicate).next_u64();
    let (mut prefix, cancel) = Run::initial();
    prefix.through(2, seed, None);
    assert_eq!(prefix.streams[&(B, "service")].word_pos(), 2);
    let first_cut = prefix.checkpoint();

    let mut control = Run::restore(&first_cut);
    let mut intervention = Run::restore(&first_cut);
    intervention.replace_b(cancel);
    intervention.through(4, seed, None);
    control.through(4, seed, None);
    assert_eq!(control.scheduler.now().ticks(), 2);
    assert_eq!(intervention.scheduler.now().ticks(), 4);
    let mut control_cut = control.checkpoint();
    let intervention_cut = intervention.checkpoint();
    assert_eq!(control_cut, first_cut, "intervention changed the control");

    intervention = Run::restore(&intervention_cut);
    assert_eq!(intervention.scheduler.now().ticks(), 4);
    intervention.through(5, seed, None);
    assert_eq!(
        control.checkpoint(),
        control_cut,
        "intervention advanced the control"
    );
    control = Run::restore(&control_cut);
    assert_eq!(control.scheduler.now().ticks(), 2);
    assert_eq!(control.scheduler.pending_events(), 2);
    assert_eq!(intervention.scheduler.stats().cancelled_events, 1);
    control.through(5, seed, None);

    assert_eq!(
        control.tape(),
        [
            (2, A, SERVICE, 0),
            (2, B, SERVICE, 0),
            (5, A, SERVICE, 2),
            (5, B, SERVICE, 2),
        ]
    );
    assert_eq!(
        intervention.tape(),
        [
            (2, A, SERVICE, 0),
            (2, B, SERVICE, 0),
            (4, SUCCESSOR, ROUTING, 0),
            (5, A, SERVICE, 2),
            (5, SUCCESSOR, SERVICE, 0),
        ]
    );
    assert_eq!(control.services, BTreeMap::from([(A, 2), (B, 2)]));
    assert_eq!(
        intervention.services,
        BTreeMap::from([(A, 2), (SUCCESSOR, 1)])
    );
    assert_eq!(
        control.world.snapshot().entities(),
        &[A, B].map(|id| EntitySnapshot { id })
    );
    assert!(!control.world.is_alive(SUCCESSOR));
    assert_eq!(
        intervention.world.snapshot().entities(),
        &[A, SUCCESSOR].map(|id| EntitySnapshot { id })
    );
    assert!(!intervention.world.is_alive(B));
    assert_eq!(
        control.draws[2], intervention.draws[3],
        "A diverged across arms"
    );
    assert_ne!(intervention.draws[4].raw, intervention.draws[1].raw);
    assert_eq!(
        control.checkpoint(),
        Run::replay(seed, false, None).checkpoint()
    );
    assert_eq!(
        intervention.checkpoint(),
        Run::replay(seed, true, None).checkpoint()
    );

    let cursor = control_cut.streams.get_mut(&(A, "service")).unwrap();
    assert_eq!(cursor.word_pos, 2);
    cursor.word_pos = 0;
    let mut missing_cursor = Run::restore(&control_cut);
    missing_cursor.through(5, seed, None);
    assert_eq!(missing_cursor.draws[..2], control.draws[..2]);
    assert_eq!(missing_cursor.draws[3], control.draws[3]);
    let misdraw = missing_cursor.draws[2];
    assert_eq!(misdraw.event, control.draws[2].event);
    assert_eq!(misdraw.word, control.draws[0].word);
    assert_eq!(misdraw.raw, control.draws[0].raw);
    assert_ne!(misdraw.raw, control.draws[2].raw);

    let mut global = DeterministicStream::from_run_seed(seed);
    let control_global = Run::replay(seed, false, Some(&mut global.clone()));
    let intervention_global = Run::replay(seed, true, Some(&mut global));
    assert_eq!(control_global.draws[..2], intervention_global.draws[..2]);
    let control_a = control_global.draws[2];
    let intervention_a = intervention_global.draws[3];
    assert_eq!(control_a.event, intervention_a.event);
    assert_eq!((control_a.word, intervention_a.word), (4, 6));
    assert_eq!(intervention_global.draws[2].raw, control_a.raw);
    assert_eq!(intervention_a.raw, control_global.draws[3].raw);
    assert_ne!(control_a.raw, intervention_a.raw);
}

#[test]
fn checkpoint_branches_match_full_replay_with_addressed_streams() {
    let mut failures = Vec::new();
    for replicate in 9100..9112 {
        if std::panic::catch_unwind(|| verify_replicate(replicate)).is_ok() {
            println!("replicate {replicate}: replay and both RNG controls passed");
        } else {
            eprintln!("replicate {replicate}: oracle failed");
            failures.push(replicate);
        }
    }
    assert!(failures.is_empty(), "failed replicates: {failures:?}");
}
