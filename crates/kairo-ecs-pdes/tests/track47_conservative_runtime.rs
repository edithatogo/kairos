use std::cell::RefCell;
use std::rc::Rc;

use kairo_ecs_pdes::{
    deadlock_stress_report, run_partitioned_reference, run_sequential_reference,
    scaling_smoke_samples, validate_conservative_pdes, LogicalProcess, LpId, NullMessage,
    ParityWorkload, PartitionPlan, PdesError, PdesMessage, PdesScheduler, PdesTransport,
    RemoteEvent, ThreadChannelTransport, Tick, WorldSegment,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};

#[derive(Debug)]
struct ObservedLpState {
    local_time: Tick,
    processed_until: Vec<Tick>,
    advanced_to: Vec<Tick>,
    received: Vec<RemoteEvent>,
}

impl ObservedLpState {
    fn new(local_time: Tick) -> Self {
        Self {
            local_time,
            processed_until: Vec::new(),
            advanced_to: Vec::new(),
            received: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct ObservedLp {
    state: Rc<RefCell<ObservedLpState>>,
    lookahead: SimDuration,
    outbound: Vec<RemoteEvent>,
}

impl ObservedLp {
    fn new(
        state: Rc<RefCell<ObservedLpState>>,
        lookahead: SimDuration,
        outbound: Vec<RemoteEvent>,
    ) -> Self {
        Self {
            state,
            lookahead,
            outbound,
        }
    }
}

impl LogicalProcess for ObservedLp {
    fn init(&mut self, _lp_id: LpId, _world_segment: &WorldSegment) {}

    fn process_local_events(&mut self, until: Tick) {
        let mut state = self.state.borrow_mut();
        state.processed_until.push(until);
        state.local_time = until;
    }

    fn schedule_remote_event(&mut self) -> Vec<RemoteEvent> {
        std::mem::take(&mut self.outbound)
    }

    fn receive_remote_events(&mut self, events: Vec<RemoteEvent>) {
        self.state.borrow_mut().received.extend(events);
    }

    fn advance_to(&mut self, tick: Tick) {
        let mut state = self.state.borrow_mut();
        state.advanced_to.push(tick);
        state.local_time = state.local_time.max(tick);
    }

    fn local_time(&self) -> Tick {
        self.state.borrow().local_time
    }

    fn lookahead(&self) -> SimDuration {
        self.lookahead
    }
}

fn state_at(tick: u128) -> Rc<RefCell<ObservedLpState>> {
    Rc::new(RefCell::new(ObservedLpState::new(SimTime::from_ticks(
        tick,
    ))))
}

fn segment(id: LpId, lp_count: u32) -> WorldSegment {
    WorldSegment {
        id,
        lp_count,
        entities: Vec::new(),
    }
}

fn is_monotonic(times: &[Tick]) -> bool {
    times.windows(2).all(|pair| pair[0] <= pair[1])
}

#[test]
fn conservative_lookahead_rejects_early_remote_events_and_allows_boundary_events() {
    let early_event = RemoteEvent {
        source_lp: LpId(0),
        dest_lp: LpId(1),
        tick: SimTime::from_ticks(4),
        event_payload: b"early".to_vec(),
    };
    let early_state = state_at(0);
    let mut scheduler = PdesScheduler::new(ThreadChannelTransport::new([LpId(0), LpId(1)]));
    scheduler
        .add_lp(
            LpId(0),
            segment(LpId(0), 2),
            vec![LpId(1)],
            Box::new(ObservedLp::new(
                Rc::clone(&early_state),
                SimDuration::from_ticks(2),
                vec![early_event],
            )),
        )
        .unwrap();
    scheduler
        .add_lp(
            LpId(1),
            segment(LpId(1), 2),
            vec![LpId(0)],
            Box::new(ObservedLp::new(
                state_at(0),
                SimDuration::from_ticks(2),
                Vec::new(),
            )),
        )
        .unwrap();

    assert_eq!(
        scheduler.step_until(SimTime::from_ticks(3)),
        Err(PdesError::LookaheadViolation {
            lp_id: LpId(0),
            event_tick: SimTime::from_ticks(4),
            minimum_tick: SimTime::from_ticks(5),
        })
    );
    assert_eq!(
        early_state.borrow().processed_until,
        vec![SimTime::from_ticks(3)]
    );

    let boundary_event = RemoteEvent {
        source_lp: LpId(0),
        dest_lp: LpId(1),
        tick: SimTime::from_ticks(5),
        event_payload: b"boundary".to_vec(),
    };
    let boundary_state = state_at(0);
    let mut scheduler = PdesScheduler::new(ThreadChannelTransport::new([LpId(0), LpId(1)]));
    scheduler
        .add_lp(
            LpId(0),
            segment(LpId(0), 2),
            vec![LpId(1)],
            Box::new(ObservedLp::new(
                Rc::clone(&boundary_state),
                SimDuration::from_ticks(2),
                vec![boundary_event],
            )),
        )
        .unwrap();
    scheduler
        .add_lp(
            LpId(1),
            segment(LpId(1), 2),
            vec![LpId(0)],
            Box::new(ObservedLp::new(
                state_at(0),
                SimDuration::from_ticks(2),
                Vec::new(),
            )),
        )
        .unwrap();

    scheduler.step_until(SimTime::from_ticks(3)).unwrap();

    assert_eq!(
        boundary_state.borrow().processed_until,
        vec![SimTime::from_ticks(3)]
    );
    assert_eq!(scheduler.gvt(), SimTime::from_ticks(3));
    assert_eq!(scheduler.transport().pending_messages(), 3);
}

#[test]
fn sequential_partitioned_parity_covers_des_abm_and_mixed_workloads() {
    let workloads = [
        ParityWorkload {
            lp_count: 2,
            ticks: 96,
            entities_per_lp: 1,
        },
        ParityWorkload {
            lp_count: 8,
            ticks: 48,
            entities_per_lp: 16,
        },
        ParityWorkload {
            lp_count: 5,
            ticks: 128,
            entities_per_lp: 3,
        },
    ];

    for workload in &workloads {
        let sequential = run_sequential_reference(workload);
        let partitioned = run_partitioned_reference(workload);

        assert_eq!(partitioned.final_state, sequential.final_state);
        assert!(partitioned.remote_events > 0);
        assert!(partitioned.null_messages > 0);
        assert!(is_monotonic(&partitioned.gvt_history));
        assert_eq!(
            partitioned.gvt_history.last(),
            Some(&SimTime::from_ticks(workload.ticks))
        );
    }

    let evidence = validate_conservative_pdes(&workloads).unwrap();
    assert_eq!(evidence.len(), workloads.len());
    assert!(evidence.iter().all(|item| item.final_state_parity));
    assert!(evidence.iter().all(|item| item.gvt_monotonic));
    assert!(evidence.iter().all(|item| item.gvt_reaches_final_tick));
    assert!(evidence.iter().all(|item| item.deadlock_smoke));
}

#[test]
fn partition_plan_and_null_messages_preserve_safe_time_progression() {
    let plan = PartitionPlan::from_entities(
        4,
        SimDuration::from_ticks(3),
        vec![
            EntityId::new(8, 0),
            EntityId::new(1, 0),
            EntityId::new(3, 1),
            EntityId::new(3, 0),
            EntityId::new(7, 0),
            EntityId::new(2, 0),
        ],
    )
    .unwrap();

    assert_eq!(plan.lp_count(), 4);
    assert_eq!(plan.lookahead(), SimDuration::from_ticks(3));
    assert_eq!(
        plan.segments(),
        &[
            WorldSegment {
                id: LpId(0),
                lp_count: 4,
                entities: vec![EntityId::new(1, 0), EntityId::new(7, 0)],
            },
            WorldSegment {
                id: LpId(1),
                lp_count: 4,
                entities: vec![EntityId::new(2, 0), EntityId::new(8, 0)],
            },
            WorldSegment {
                id: LpId(2),
                lp_count: 4,
                entities: vec![EntityId::new(3, 0)],
            },
            WorldSegment {
                id: LpId(3),
                lp_count: 4,
                entities: vec![EntityId::new(3, 1)],
            },
        ]
    );
    assert_eq!(plan.owner_of(EntityId::new(8, 0)), Some(LpId(1)));
    assert_eq!(plan.owner_of(EntityId::new(9, 0)), None);

    let safe_state = state_at(0);
    let mut transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
    transport
        .send(
            LpId(0),
            PdesMessage::Null(NullMessage {
                source_lp: LpId(1),
                dest_lp: LpId(0),
                safe_time: SimTime::from_ticks(4),
            }),
        )
        .unwrap();
    let mut scheduler = PdesScheduler::new(transport);
    scheduler
        .add_lp(
            LpId(0),
            segment(LpId(0), 2),
            Vec::new(),
            Box::new(ObservedLp::new(
                Rc::clone(&safe_state),
                SimDuration::from_ticks(1),
                Vec::new(),
            )),
        )
        .unwrap();

    scheduler.step_until(SimTime::from_ticks(10)).unwrap();

    assert_eq!(
        safe_state.borrow().processed_until,
        vec![SimTime::from_ticks(4)]
    );
    assert_eq!(safe_state.borrow().local_time, SimTime::from_ticks(4));
    assert_eq!(scheduler.gvt(), SimTime::from_ticks(4));

    let stale_state = state_at(7);
    let mut transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
    transport
        .send(
            LpId(0),
            PdesMessage::Null(NullMessage {
                source_lp: LpId(1),
                dest_lp: LpId(0),
                safe_time: SimTime::from_ticks(2),
            }),
        )
        .unwrap();
    let mut scheduler = PdesScheduler::new(transport);
    scheduler
        .add_lp(
            LpId(0),
            segment(LpId(0), 2),
            Vec::new(),
            Box::new(ObservedLp::new(
                Rc::clone(&stale_state),
                SimDuration::from_ticks(1),
                Vec::new(),
            )),
        )
        .unwrap();

    scheduler.step_until(SimTime::from_ticks(10)).unwrap();

    assert_eq!(
        stale_state.borrow().processed_until,
        vec![SimTime::from_ticks(7)]
    );
    assert_eq!(stale_state.borrow().local_time, SimTime::from_ticks(7));
    assert_eq!(scheduler.gvt(), SimTime::from_ticks(7));
}

#[test]
fn deadlock_stress_and_scaling_smoke_have_progress_without_speedup_claims() {
    let stress = deadlock_stress_report().unwrap();

    assert_eq!(stress.gvt_history.len(), 10_000);
    assert_eq!(
        stress.gvt_history.last(),
        Some(&SimTime::from_ticks(10_000))
    );
    assert!(is_monotonic(&stress.gvt_history));
    assert!(stress.remote_events > 0);
    assert!(stress.null_messages > 0);

    let evidence = validate_conservative_pdes(&[ParityWorkload::stress()]).unwrap();
    assert_eq!(evidence.len(), 1);
    assert!(evidence[0].deadlock_smoke);
    assert_eq!(evidence[0].gvt_samples, 10_000);

    let samples = scaling_smoke_samples(&[4, 8, 16, 32]).unwrap();
    assert_eq!(samples.len(), 4);
    assert!(samples.iter().all(|sample| sample.final_state_parity));
    assert!(samples
        .iter()
        .all(|sample| !sample.hardware_speedup_claimed));
    assert!(samples.iter().all(|sample| sample.gvt_samples == 256));
}
