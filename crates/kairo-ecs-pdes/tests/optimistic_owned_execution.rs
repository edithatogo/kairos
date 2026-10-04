#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    LpId, OptimisticAuthority, OptimisticError, OptimisticLimits, OptimisticOwnedFailurePhase,
    OptimisticOwnedOptions, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot(Vec<u8>);

struct Model {
    values: Vec<u8>,
    invalid_source: bool,
}

impl OptimisticProcess for Model {
    type Snapshot = Snapshot;

    fn snapshot(&self) -> Self::Snapshot {
        Snapshot(self.values.clone())
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.values.clone_from(&snapshot.0);
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.values.push(event.event_payload[0]);
        if event.event_payload[0] == 0 {
            vec![RemoteEvent {
                source_lp: if self.invalid_source {
                    LpId(1)
                } else {
                    LpId(0)
                },
                dest_lp: LpId(0),
                tick: event.tick.checked_add(SimDuration::from_ticks(1)).unwrap(),
                event_payload: vec![1],
            }]
        } else {
            Vec::new()
        }
    }
}

fn runtime(invalid_source: bool) -> OptimisticRuntime<Model> {
    let partition =
        PartitionPlan::from_entities(1, SimDuration::from_ticks(1), vec![EntityId::new(1, 0)])
            .unwrap();
    let topology = BTreeMap::from([(LpId(0), Vec::new())]);
    let namespace = 0x91;
    OptimisticRuntime::new_owned(
        partition,
        topology,
        BTreeMap::from([(
            LpId(0),
            Model {
                values: Vec::new(),
                invalid_source,
            },
        )]),
        OptimisticOwnedOptions {
            simulation_namespace: namespace,
            current_authorities: BTreeMap::from([(
                LpId(0),
                OptimisticAuthority::Scoped {
                    simulation_namespace: namespace,
                    ownership_epoch: 7,
                },
            )]),
            emission_epochs: BTreeMap::from([(LpId(0), 7)]),
            local_limits: OptimisticLimits::default(),
            max_global_lps: 1,
            max_outbox_entries: 8,
            max_transition_entries: 16,
            max_receipt_entries: 16,
        },
    )
    .unwrap()
}

fn root() -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(0),
        dest_lp: LpId(0),
        tick: Tick::from_ticks(5),
        event_payload: vec![0],
    }
}

#[test]
fn owned_positive_round_publishes_local_children_after_complete_staging() {
    let mut runtime = runtime(false);
    runtime.seal_native_peers().unwrap();
    runtime.schedule_initial(1, root()).unwrap();

    let progress = runtime
        .run_owned_until_with_budget(Tick::from_ticks(10), 1)
        .unwrap();

    assert_eq!(progress.budget_used, 1);
    assert_eq!(progress.pending_positives, 1);
    assert_eq!(progress.published_messages.len(), 1);
    assert_eq!(
        progress.published_messages[0].event().event_payload,
        vec![1]
    );
    assert_eq!(runtime.pending_events(LpId(0)).unwrap().len(), 1);
    assert_eq!(runtime.process_at(LpId(0)).unwrap().values, vec![0]);
    let intents = runtime.native_intents().unwrap();
    assert_eq!(intents.len(), 2);
    assert!(intents.iter().all(|intent| intent.is_current()));
    assert!(runtime.initial_inputs_closed());
}

#[test]
fn invalid_staged_output_compensates_without_publishing_or_closing_inputs() {
    let mut runtime = runtime(true);
    runtime.seal_native_peers().unwrap();
    runtime.schedule_initial(1, root()).unwrap();
    let revision = runtime.accounting_revision().unwrap();
    let queued = runtime.pending_events(LpId(0)).unwrap();

    let failure = runtime
        .run_owned_until_with_budget(Tick::from_ticks(10), 1)
        .unwrap_err();

    assert_eq!(failure.phase(), OptimisticOwnedFailurePhase::Compensated);
    assert_eq!(
        failure.cause(),
        &OptimisticError::OutputSourceMismatch {
            lp_id: LpId(0),
            declared: LpId(1),
        }
    );
    assert_eq!(failure.progress().budget_used, 1);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(
        runtime.process_at(LpId(0)).unwrap().values,
        Vec::<u8>::new()
    );
    assert_eq!(runtime.pending_events(LpId(0)).unwrap(), queued);
    assert_eq!(runtime.accounting_revision().unwrap(), revision + 1);
    assert!(!runtime.initial_inputs_closed());
}

struct Cascade(Vec<u8>);
impl OptimisticProcess for Cascade {
    type Snapshot = Vec<u8>;
    fn snapshot(&self) -> Vec<u8> {
        self.0.clone()
    }
    fn restore(&mut self, value: &Vec<u8>) -> Result<(), OptimisticStateError> {
        self.0.clone_from(value);
        Ok(())
    }
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.0.push(event.event_payload[0]);
        if event.event_payload[0] == 2 {
            vec![RemoteEvent {
                source_lp: event.dest_lp,
                dest_lp: LpId(2),
                tick: Tick::from_ticks(30),
                event_payload: vec![3],
            }]
        } else {
            Vec::new()
        }
    }
}
fn cascade_group() -> Vec<OptimisticRuntime<Cascade>> {
    let partition = PartitionPlan::from_entities(
        3,
        SimDuration::from_ticks(1),
        vec![
            EntityId::new(1, 0),
            EntityId::new(2, 0),
            EntityId::new(3, 0),
        ],
    )
    .unwrap();
    let topology = BTreeMap::from([
        (LpId(0), vec![LpId(1)]),
        (LpId(1), vec![LpId(2)]),
        (LpId(2), Vec::new()),
    ]);
    let authorities = (0..3)
        .map(|id| {
            (
                LpId(id),
                OptimisticAuthority::Scoped {
                    simulation_namespace: 717,
                    ownership_epoch: 1,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut group = (0..3)
        .map(|id| {
            OptimisticRuntime::new_owned(
                partition.clone(),
                topology.clone(),
                BTreeMap::from([(LpId(id), Cascade(Vec::new()))]),
                OptimisticOwnedOptions {
                    simulation_namespace: 717,
                    current_authorities: authorities.clone(),
                    emission_epochs: BTreeMap::from([(LpId(id), 1)]),
                    local_limits: OptimisticLimits::default(),
                    max_global_lps: 3,
                    max_outbox_entries: 16,
                    max_transition_entries: 32,
                    max_receipt_entries: 32,
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let peers = group
        .iter()
        .map(|runtime| runtime.native_accounting_authority().unwrap())
        .collect::<Vec<_>>();
    for runtime in &mut group {
        for peer in &peers {
            runtime.register_native_peer(peer.clone()).unwrap();
        }
        runtime.seal_native_peers().unwrap();
    }
    group
}
fn cascade_input(source: u32, destination: u32, tick: u128, value: u8) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
        tick: Tick::from_ticks(tick),
        event_payload: vec![value],
    }
}
#[test]
fn suffix_control_and_executed_anti_are_each_one_atomic_budget_unit() {
    let mut group = cascade_group();
    group[0]
        .schedule_initial(1, cascade_input(0, 1, 5, 1))
        .unwrap();
    group[1]
        .schedule_initial(2, cascade_input(1, 1, 20, 2))
        .unwrap();
    group[1]
        .run_owned_until_with_budget(Tick::from_ticks(40), 1)
        .unwrap();
    let old = group[1].ready_native_sends().unwrap().remove(0);
    let old_ack = group[2].admit_native(&old).unwrap();
    group[2]
        .run_owned_until_with_budget(Tick::from_ticks(40), 1)
        .unwrap();
    let straggler = group[0].ready_native_sends().unwrap().remove(0);
    let root_ack = group[1].admit_native(&straggler).unwrap();
    group[0].acknowledge_native_admission(root_ack).unwrap();
    let rollback = group[1]
        .run_owned_until_with_budget(Tick::from_ticks(40), 1)
        .unwrap();
    assert_eq!(rollback.budget_used, 1);
    assert_eq!(rollback.replay_pending, 1);
    assert_eq!(group[1].process_at(LpId(1)).unwrap().0, Vec::<u8>::new());
    assert_eq!(rollback.published_messages.len(), 1);
    assert_eq!(group[1].ready_native_sends().unwrap().len(), 1); // anti only; old P retired
    group[1]
        .run_owned_until_with_budget(Tick::from_ticks(40), 2)
        .unwrap();
    assert_eq!(group[1].process_at(LpId(1)).unwrap().0, vec![1, 2]);
    assert_eq!(group[1].accounting_snapshot().unwrap().blocked_count(), 1);
    let request = group[1]
        .pending_native_retirement_requests()
        .unwrap()
        .remove(0);
    let anti_ack = group[2].receive_native_retirement(&request).unwrap();
    let applied = group[2]
        .run_owned_until_with_budget(Tick::from_ticks(40), 1)
        .unwrap();
    assert_eq!(applied.budget_used, 1);
    assert_eq!(group[2].process_at(LpId(2)).unwrap().0, Vec::<u8>::new());
    let proof = group[2].applied_native_retirements().unwrap().remove(0);
    assert_eq!(
        proof.applied_effect(),
        kairo_ecs_pdes::NativeRetirementEffect::RolledBackExecuted
    );
    group[1]
        .acknowledge_native_retirement(proof.clone())
        .unwrap();
    let revision = group[1].accounting_revision().unwrap();
    group[1].acknowledge_native_retirement(proof).unwrap();
    group[1].acknowledge_native_admission(old_ack).unwrap(); // lost P ACK closes through proof
    group[1].acknowledge_native_admission(anti_ack).unwrap();
    assert_eq!(group[1].accounting_revision().unwrap(), revision);
    let new = group[1].ready_native_sends().unwrap().remove(0);
    let ack = group[2].admit_native(&new).unwrap();
    group[1].acknowledge_native_admission(ack).unwrap();
    group[2]
        .run_owned_until_with_budget(Tick::from_ticks(40), 1)
        .unwrap();
    group[0].close_initial_inputs().unwrap();
    let report = OptimisticRuntime::fossil_collect_native_group(
        &mut group.iter_mut().collect::<Vec<_>>(),
        Tick::from_ticks(31),
    )
    .unwrap();
    assert!(report.cleanup_failures().is_empty());
    assert_eq!(group[1].native_intents().unwrap().len(), 1); // lifetime root reservation only
    assert_eq!(
        group[1]
            .accounting_snapshot()
            .unwrap()
            .retained_receipt_count(),
        0
    );
    let revisions = group
        .iter()
        .map(|runtime| runtime.accounting_revision().unwrap())
        .collect::<Vec<_>>();
    OptimisticRuntime::fossil_collect_native_group(
        &mut group.iter_mut().collect::<Vec<_>>(),
        Tick::from_ticks(31),
    )
    .unwrap();
    assert_eq!(
        revisions,
        group
            .iter()
            .map(|runtime| runtime.accounting_revision().unwrap())
            .collect::<Vec<_>>()
    );
}
