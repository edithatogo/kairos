#![cfg(feature = "time-warp")]

use kairo_ecs_pdes::{
    LpId, OptimisticError, OptimisticLimits, OptimisticProcess, OptimisticRuntime,
    OptimisticStateError, PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};
use std::collections::BTreeMap;

const LP: LpId = LpId(0);

#[derive(Default)]
struct Model(Vec<u8>);

impl OptimisticProcess for Model {
    type Snapshot = Vec<u8>;

    fn snapshot(&self) -> Self::Snapshot {
        self.0.clone()
    }

    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.0.clone_from(state);
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.0.extend_from_slice(&event.event_payload);
        Vec::new()
    }
}

fn runtime() -> OptimisticRuntime<Model> {
    OptimisticRuntime::new(
        PartitionPlan::from_entities(1, SimDuration::from_ticks(1), vec![EntityId::new(0, 0)])
            .unwrap(),
        BTreeMap::from([(LP, Vec::new())]),
        BTreeMap::from([(LP, Model::default())]),
        OptimisticLimits::default(),
    )
    .unwrap()
}

fn tick(at: u128) -> Tick {
    Tick::from_ticks(at)
}

#[test]
fn snapshot_excludes_all_events_at_the_cutoff_and_is_stable_across_new_cuts() {
    let mut runtime = runtime();
    for (sequence, at, byte) in [(1, 2, b'a'), (2, 5, b'b'), (3, 5, b'c')] {
        runtime
            .schedule_initial(
                sequence,
                RemoteEvent {
                    source_lp: LP,
                    dest_lp: LP,
                    tick: tick(at),
                    event_payload: vec![byte],
                },
            )
            .unwrap();
    }
    runtime.run_until_with_budget(tick(5), 3).unwrap();
    assert_eq!(runtime.process_at(LP).unwrap().0, b"abc");
    let initial = runtime.committed_snapshot(LP).unwrap();
    assert_eq!(initial.exclusive_cutoff(), Tick::ZERO);
    assert!(initial.state().is_empty());
    runtime.fossil_collect(tick(5)).unwrap();
    let cutoff = runtime.committed_snapshot(LP).unwrap();
    assert_eq!(cutoff.lp_id(), LP);
    assert_eq!(cutoff.exclusive_cutoff(), tick(5));
    assert_eq!(cutoff.state(), b"a");

    runtime.fossil_collect(tick(6)).unwrap();
    let later = runtime.committed_snapshot(LP).unwrap();
    assert_eq!(later.exclusive_cutoff(), tick(6));
    assert_eq!(later.state(), b"abc");
    assert_eq!(cutoff.state(), b"a");
    assert_eq!(
        runtime.committed_snapshot(LpId(7)).unwrap_err(),
        OptimisticError::UnknownLogicalProcess(LpId(7)),
    );
}
