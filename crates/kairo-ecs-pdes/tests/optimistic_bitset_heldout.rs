#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    GenerationBitset, GenerationBitsetSnapshot, LpId, OptimisticLimits, OptimisticMessage,
    OptimisticProcess, OptimisticRuntime, OptimisticStateError, PartitionPlan, RemoteEvent,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};

struct Model {
    membership: GenerationBitset,
    values: [u64; 4],
    rng: u64,
}

#[derive(Clone)]
struct Snapshot {
    membership: GenerationBitsetSnapshot,
    values: [u64; 4],
    rng: u64,
}

impl Model {
    fn new() -> Self {
        let mut membership = GenerationBitset::new(4).unwrap();
        membership.insert(0).unwrap();
        Self {
            membership,
            values: [100, 0, 0, 0],
            rng: 9,
        }
    }

    fn active(&self) -> [bool; 4] {
        std::array::from_fn(|slot| self.membership.is_active(slot as u32).unwrap())
    }
}

impl OptimisticProcess for Model {
    type Snapshot = Snapshot;

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            membership: self.membership.snapshot(),
            values: self.values,
            rng: self.rng,
        }
    }

    fn restore(&mut self, state: &Snapshot) -> Result<(), OptimisticStateError> {
        // Fallible membership restoration precedes publication of logical
        // values/RNG. The snapshot never contains a live handle or epoch.
        self.membership
            .restore(&state.membership)
            .map_err(|error| OptimisticStateError::new(format!("{error:?}")))?;
        self.values = state.values;
        self.rng = state.rng;
        Ok(())
    }

    fn on_event(&mut self, input: &RemoteEvent) -> Vec<RemoteEvent> {
        let slot = input.event_payload[1] as u32;
        match input.event_payload[0] {
            0 => {
                if !self.membership.is_active(slot).unwrap() {
                    self.membership.insert(slot).unwrap();
                }
                self.values[slot as usize] = input.event_payload[2] as u64;
            }
            1 => {
                // Reconstruct authority from logical membership, never from a
                // handle captured in reversible model state.
                self.membership
                    .remove(self.membership.handle(slot).unwrap())
                    .unwrap();
                self.values[slot as usize] = 0;
            }
            _ => panic!("unknown held-out operation"),
        }
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(input.tick.ticks() as u64);
        Vec::new()
    }
}

fn runtime() -> OptimisticRuntime<Model> {
    OptimisticRuntime::new(
        PartitionPlan::from_entities(
            2,
            SimDuration::from_ticks(1),
            vec![EntityId::new(0, 0), EntityId::new(1, 0)],
        )
        .unwrap(),
        BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])]),
        BTreeMap::from([(LpId(0), Model::new()), (LpId(1), Model::new())]),
        OptimisticLimits::default(),
    )
    .unwrap()
}

fn event(tick: u128, operation: u8, slot: u8, value: u8) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(0),
        dest_lp: LpId(1),
        tick: SimTime::from_ticks(tick),
        event_payload: vec![operation, slot, value],
    }
}

fn stage(seq: u64, input: RemoteEvent) -> OptimisticMessage {
    runtime().schedule_initial(seq, input).unwrap()
}

fn drain(rt: &mut OptimisticRuntime<Model>) {
    for _ in 0..16 {
        if !rt
            .run_until_with_budget(SimTime::from_ticks(20), 1)
            .unwrap()
            .budget_exhausted
        {
            assert_eq!(rt.report().replay_pending, 0);
            return;
        }
    }
    panic!("finite bitset fixture did not drain");
}

#[test]
fn real_model_rollback_restores_membership_values_rng_without_reviving_handles() {
    let mut rt = runtime();
    let initial_handle = rt
        .process_at(LpId(1))
        .unwrap()
        .membership
        .handle(0)
        .unwrap();
    let later = rt.schedule_initial(2, event(20, 1, 0, 0)).unwrap();
    drain(&mut rt);
    assert_eq!(
        rt.process_at(LpId(1)).unwrap().active(),
        [false, false, false, false]
    );

    let earlier = stage(1, event(10, 0, 1, 11));
    rt.receive(earlier.clone()).unwrap();
    let progress = rt
        .run_until_with_budget(SimTime::from_ticks(20), 1)
        .unwrap();
    assert_eq!(progress.replay_pending, 1);
    let model = rt.process_at(LpId(1)).unwrap();
    assert_eq!(model.active(), [true, true, false, false]);
    assert_eq!(model.values, [100, 11, 0, 0]);
    assert_eq!(model.rng, 1936993793492482207);
    assert!(model.membership.validate(initial_handle).is_err());
    let speculative_handle = model.membership.handle(1).unwrap();

    drain(&mut rt);
    let model = rt.process_at(LpId(1)).unwrap();
    assert_eq!(model.active(), [false, true, false, false]);
    assert_eq!(model.values, [0, 11, 0, 0]);
    assert_eq!(model.rng, 2202233241294083335);
    assert!(model.membership.validate(speculative_handle).is_err());

    rt.receive(earlier.as_anti()).unwrap();
    drain(&mut rt);
    let model = rt.process_at(LpId(1)).unwrap();
    assert_eq!(model.active(), [false, false, false, false]);
    assert_eq!(model.values, [0, 0, 0, 0]);
    assert_eq!(model.rng, 1936993793492482217);

    rt.receive(later.as_anti()).unwrap();
    drain(&mut rt);
    let model = rt.process_at(LpId(1)).unwrap();
    assert_eq!(model.active(), [true, false, false, false]);
    assert_eq!(model.values, [100, 0, 0, 0]);
    assert_eq!(model.rng, 9);
    assert!(model.membership.validate(initial_handle).is_err());
    assert!(model.membership.validate(speculative_handle).is_err());
    let fresh = model.membership.handle(0).unwrap();
    assert_eq!(model.membership.validate(fresh).unwrap(), 0);
    assert!(rt
        .process_at(LpId(0))
        .unwrap()
        .membership
        .validate(fresh)
        .is_err());
}

#[test]
fn bitset_restore_error_prevents_publication_of_staged_model_values_and_rng() {
    let mut model = Model::new();
    let handle = model.membership.handle(0).unwrap();
    let invalid = Snapshot {
        membership: GenerationBitset::new(8).unwrap().snapshot(),
        values: [999; 4],
        rng: 777,
    };
    assert!(model.restore(&invalid).is_err());
    assert_eq!(model.values, [100, 0, 0, 0]);
    assert_eq!(model.rng, 9);
    assert_eq!(model.active(), [true, false, false, false]);
    assert_eq!(model.membership.validate(handle).unwrap(), 0);
}
