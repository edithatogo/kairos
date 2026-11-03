// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let p = std::env::temp_dir().join(format!(
            "cap48-external-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Model {
    value: i64,
    rng: u64,
    generation: u64,
    trace: Vec<(u128, Vec<u8>)>,
}
impl Model {
    fn new() -> Self {
        Self {
            value: 0,
            rng: 48,
            generation: 1,
            trace: vec![],
        }
    }
}
impl OptimisticProcess for Model {
    type Snapshot = Self;
    fn snapshot(&self) -> Self {
        self.clone()
    }
    fn restore(&mut self, s: &Self) -> std::result::Result<(), super::super::OptimisticStateError> {
        *self = s.clone();
        Ok(())
    }
    fn on_event(&mut self, e: &RemoteEvent) -> Vec<RemoteEvent> {
        self.generation += 1;
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.trace.push((e.tick.ticks(), e.event_payload.clone()));
        match e.event_payload[0] {
            1 => vec![RemoteEvent {
                source_lp: e.dest_lp,
                dest_lp: LpId(1),
                tick: Tick::from_ticks(if self.value >= 100 { 25 } else { 30 }),
                event_payload: vec![3, self.value as u8],
            }],
            2 => {
                self.value += 100;
                vec![]
            }
            3 => {
                self.value += e.event_payload[1] as i64;
                vec![RemoteEvent {
                    source_lp: e.dest_lp,
                    dest_lp: LpId(0),
                    tick: Tick::from_ticks(e.tick.ticks() + 5),
                    event_payload: vec![4],
                }]
            }
            4 => {
                self.value += 10;
                vec![]
            }
            _ => vec![],
        }
    }
}
struct Codec;
impl PortableModelCodec<Model> for Codec {
    fn identity(&self) -> Hash {
        digest(b"cap48-model-v1-complete-rng-generations-trace")
    }
    fn encode_process(&self, p: &Model) -> std::result::Result<Vec<u8>, String> {
        serde_json::to_vec(p).map_err(|e| e.to_string())
    }
    fn decode_process(&self, b: &[u8]) -> std::result::Result<Model, String> {
        serde_json::from_slice(b).map_err(|e| e.to_string())
    }
    fn encode_snapshot(&self, s: &Model) -> std::result::Result<Vec<u8>, String> {
        self.encode_process(s)
    }
    fn decode_snapshot(&self, b: &[u8]) -> std::result::Result<Model, String> {
        self.decode_process(b)
    }
}
type Runtime = ExternalOptimisticRuntime<Model, Codec>;
fn config() -> ExternalConfiguration {
    ExternalConfiguration {
        simulation: [48; 16],
        model: Codec.identity(),
        members: (0..2)
            .map(|p| ExternalMember {
                participant: p,
                public_key: SigningKey::from_bytes(&[p as u8 + 1; 32])
                    .verifying_key()
                    .to_bytes()
                    .to_vec(),
                certificate_hash: [p as u8 + 1; 32],
                lps: vec![p],
            })
            .collect(),
        topology: vec![(0, vec![1]), (1, vec![0])],
    }
}
fn open(d: &Temp, id: u32) -> Runtime {
    Runtime::open(
        &d.0,
        config(),
        id,
        SigningKey::from_bytes(&[id as u8 + 1; 32]),
        Codec,
        BTreeMap::from([(id, Model::new())]),
    )
    .unwrap()
}
fn activate(a: &mut Runtime, b: &mut Runtime) {
    let p = a.activation_proposal().unwrap();
    let joins = vec![
        a.join_activation(&p).unwrap(),
        b.join_activation(&p).unwrap(),
    ];
    a.activate(&p, &joins).unwrap();
    b.activate(&p, &joins).unwrap();
}
fn deliver(sender: &mut Runtime, receiver: &mut Runtime, p: &[u8]) {
    for reply in receiver.receive_packet(p).unwrap() {
        sender.receive_packet(&reply).unwrap();
    }
}
fn self_deliver(a: &mut Runtime, p: &[u8]) {
    for reply in a.receive_packet(p).unwrap() {
        a.receive_packet(&reply).unwrap();
    }
}
fn root(a: &mut Runtime, seq: u64, tick: u128, op: u8) -> Vec<u8> {
    a.schedule_initial(
        seq,
        RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(0),
            tick: Tick::from_ticks(tick),
            event_payload: vec![op],
        },
    )
    .unwrap();
    a.pending_packets().unwrap().last().unwrap().clone()
}
#[test]
fn standby_complete_join_exact_admission_and_stale_tls_fail_closed() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    assert!(matches!(
        a.run_until_with_budget(40, 1),
        Err(ExternalError::Standby)
    ));
    let proposal = a.activation_proposal().unwrap();
    let ja = a.join_activation(&proposal).unwrap();
    assert!(a.activate(&proposal, std::slice::from_ref(&ja)).is_err());
    let jb = b.join_activation(&proposal).unwrap();
    a.activate(&proposal, &[ja.clone(), jb.clone()]).unwrap();
    b.activate(&proposal, &[ja, jb]).unwrap();
    let p = root(&mut a, 0, 20, 1);
    assert!(a.receive_from([9; 32], &p).is_err());
    let response = a.receive_from([1; 32], &p).unwrap();
    let r = a.revision().unwrap();
    assert_eq!(a.receive_packet(&p).unwrap(), response);
    assert_eq!(a.revision().unwrap(), r);
    drop(a);
    let mut a = open(&da, 0);
    assert!(!a.is_active());
    activate(&mut a, &mut b);
    assert_eq!(a.generation(), 2);
    assert_eq!(a.receive_packet(&p).unwrap(), response);
    let mut bad = a.auth(&p).unwrap();
    bad.sequence += 1;
    let altered = protocol::sign(&bad, &a.key).unwrap();
    assert!(a.receive_packet(&altered).is_err());
}
#[test]
fn actual_old30_new25_waits_for_descendant_retirement_and_survives_restart() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    let p20 = root(&mut a, 0, 20, 1);
    let p10 = root(&mut a, 1, 10, 2);
    self_deliver(&mut a, &p20);
    a.run_until_with_budget(100, 1).unwrap();
    let old = a
        .pending_packets()
        .unwrap()
        .into_iter()
        .find(|p| matches!(a.auth(p).unwrap().body,Body::Delivery{message} if message.tick==30))
        .unwrap();
    deliver(&mut a, &mut b, &old);
    b.run_until_with_budget(100, 1).unwrap();
    let child = b.pending_packets().unwrap()[0].clone();
    deliver(&mut b, &mut a, &child);
    a.run_until_with_budget(100, 1).unwrap();
    self_deliver(&mut a, &p10);
    a.run_until_with_budget(100, 3).unwrap();
    assert!(a
        .image
        .outgoing
        .iter()
        .any(|o| o.message.tick == 25 && !o.dependencies.is_empty()));
    assert!(!a
        .pending_packets()
        .unwrap()
        .iter()
        .any(|p| matches!(a.auth(p).unwrap().body,Body::Delivery{message} if message.tick==25)));
    let anti=a.pending_packets().unwrap().into_iter().find(|p|matches!(a.auth(p).unwrap().body,Body::Delivery{message} if message.anti&&message.tick==30)).unwrap();
    deliver(&mut a, &mut b, &anti);
    b.run_until_with_budget(100, 1).unwrap();
    assert!(b
        .image
        .reversals
        .iter()
        .any(|r| r.applied && r.receipt.is_none()));
    let saved = b.checkpoint().unwrap();
    drop(b);
    let mut b = open(&db, 1);
    assert_eq!(
        b.image.processes,
        serde_json::from_slice::<Image>(&saved).unwrap().processes
    );
    activate(&mut a, &mut b);
    let descendant = b
        .pending_packets()
        .unwrap()
        .into_iter()
        .find(|p| matches!(b.auth(p).unwrap().body,Body::Delivery{message} if message.anti))
        .unwrap();
    let replies = a.recover_delivery(&descendant).unwrap();
    for reply in replies {
        b.receive_packet(&reply).unwrap();
    }
    a.run_until_with_budget(100, 4).unwrap();
    deliver(&mut b, &mut a, &descendant);
    deliver(&mut a, &mut b, &anti);
    let replacement=a.pending_packets().unwrap().into_iter().find(|p|matches!(a.auth(p).unwrap().body,Body::Delivery{message} if !message.anti&&message.tick==25)).unwrap();
    let replies = b.recover_delivery(&replacement).unwrap();
    for reply in replies {
        a.receive_packet(&reply).unwrap();
    }
    b.run_until_with_budget(100, 4).unwrap();
    for p in b.pending_packets().unwrap() {
        let replies = a
            .recover_delivery(&p)
            .or_else(|_| a.receive_packet(&p))
            .unwrap();
        for r in replies {
            b.receive_packet(&r).unwrap();
        }
    }
    a.run_until_with_budget(100, 4).unwrap();
    let ma: Model = serde_json::from_slice(a.model_bytes(0).unwrap()).unwrap();
    let mb: Model = serde_json::from_slice(b.model_bytes(1).unwrap()).unwrap();
    assert_eq!(ma.value, 110);
    assert_eq!(mb.value, 100);
    assert!(ma.trace.iter().all(|(tick, _)| *tick != 35));
    assert!(mb.trace.iter().all(|(tick, _)| *tick != 30));
    assert!(a.counters().1 > 0 && b.counters().1 > 0);
    let partition = crate::PartitionPlan::from_entities(
        2,
        kairo_ecs_types::SimDuration::from_ticks(1),
        vec![
            kairo_ecs_types::EntityId::new(1, 0),
            kairo_ecs_types::EntityId::new(2, 0),
        ],
    )
    .unwrap();
    let mut serial = super::super::OptimisticRuntime::new(
        partition,
        BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])]),
        BTreeMap::from([(LpId(0), Model::new()), (LpId(1), Model::new())]),
        super::super::OptimisticLimits::default(),
    )
    .unwrap();
    for (sequence, tick, op) in [(0, 20, 1), (1, 10, 2)] {
        serial
            .schedule_initial(
                sequence,
                RemoteEvent {
                    source_lp: LpId(0),
                    dest_lp: LpId(0),
                    tick: Tick::from_ticks(tick),
                    event_payload: vec![op],
                },
            )
            .unwrap();
    }
    let published = serial
        .run_until_with_budget(Tick::from_ticks(100), 64)
        .unwrap()
        .published_messages;
    for native in published {
        let actual = a
            .image
            .outgoing
            .iter()
            .chain(&b.image.outgoing)
            .find(|o| {
                !o.message.anti
                    && o.message.tick == native.event().tick.ticks()
                    && o.message.source == native.event().source_lp.0
                    && o.message.destination == native.event().dest_lp.0
                    && o.message.payload == native.event().event_payload
            })
            .unwrap();
        assert_eq!(logical_key(&actual.message), native.order_key());
    }

    assert_eq!(&ma, serial.process_at(LpId(0)).unwrap());
    assert_eq!(&mb, serial.process_at(LpId(1)).unwrap());
}

#[test]
fn complete_cut_freezes_all_model_work_and_applies_remotely_after_restart() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    a.close_initial_inputs().unwrap();
    b.close_initial_inputs().unwrap();
    let pause = a.pause_cut().unwrap();
    b.receive_packet(&pause).unwrap();
    assert!(a.pending_packets().is_err());
    assert!(b.run_until_with_budget(40, 1).is_err());
    assert!(b.prepare_cut(u128::MAX).is_err());
    let pa = a.prepare_cut(40).unwrap();
    let pb = b.prepare_cut(40).unwrap();
    let ar = a.revision().unwrap();
    assert_eq!(a.prepare_cut(40).unwrap(), pa);
    assert_eq!(a.revision().unwrap(), ar);
    assert!(a.commit_cut(vec![pa.clone()]).is_err());
    let decision = a.commit_cut(vec![pa, pb]).unwrap();
    assert_eq!(a.gvt(), 40);
    drop(b);
    let mut b = open(&db, 1);
    assert!(!b.is_active());
    b.recover_cut(&decision).unwrap();
    assert_eq!(b.gvt(), 40);
    assert!(!b.is_active());
    activate(&mut a, &mut b);
    assert_eq!(b.gvt(), 40);
}
#[test]
fn partial_activation_decision_catches_up_before_next_successor_join() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    let p = a.activation_proposal().unwrap();
    let joins = vec![
        a.join_activation(&p).unwrap(),
        b.join_activation(&p).unwrap(),
    ];
    a.activate(&p, &joins).unwrap();
    drop(a);
    let mut a = open(&da, 0);
    assert_eq!(a.generation(), 2);
    let (decision, original_joins) = a.retained_activation().unwrap();
    a.activate(&decision, &original_joins).unwrap();
    assert!(!a.is_active());
    b.activate(&decision, &original_joins).unwrap();
    assert_eq!(b.generation(), 2);
    activate(&mut a, &mut b);
    assert_eq!(a.generation(), 3);
    assert_eq!(b.generation(), 3);
}
#[test]
fn cut_rejects_open_inputs_and_new_live_delivery_after_pause() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    let p = root(&mut a, 0, 40, 9);
    let pause = a.pause_cut().unwrap();
    b.receive_packet(&pause).unwrap();
    assert!(a.prepare_cut(40).is_err());
    assert!(a.receive_packet(&p).is_err());
    assert_eq!(a.gvt(), 0);
    assert_eq!(b.gvt(), 0);
}
#[test]
fn original_unadmitted_send_is_authorized_across_multiple_successor_joins() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    let original = root(&mut a, 0, 20, 1);
    drop(a);
    let mut a = open(&da, 0);
    activate(&mut a, &mut b);
    drop(a);
    let mut a = open(&da, 0);
    activate(&mut a, &mut b);
    assert_eq!(a.generation(), 3);
    assert!(a.receive_packet(&original).is_err());
    let replies = a.recover_delivery(&original).unwrap();
    let revision = a.revision().unwrap();
    for p in replies {
        a.receive_packet(&p).unwrap();
    }
    let revision = a.revision().unwrap().max(revision);
    let replies2 = a.recover_delivery(&original).unwrap();
    assert_eq!(replies2.len(), 1);
    assert_eq!(a.revision().unwrap(), revision);
    a.run_until_with_budget(100, 1).unwrap();
    assert_eq!(a.counters().0, 1);
}
#[test]
fn structurally_corrupt_but_store_hashed_images_reject() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    for broken in 0..3 {
        let d = Temp::new();
        let mut a = open(&d, 0);
        let mut image = a.image.clone();
        match broken {
            0 => image.next_incarnation.push(image.next_incarnation[0]),
            1 => image.barriers.push([9; 32]),
            _ => image.processes[0].replay.push([9; 32]),
        };
        let prior = a.store.load().unwrap();
        a.store
            .commit(
                &Unit {
                    id: [broken + 1; 16],
                    previous_revision: prior.revision,
                    previous_digest: prior.digest,
                    sections: vec![("engine".into(), bytes(&image).unwrap())],
                    facts: vec![],
                    outbox: vec![],
                },
                CrashPoint::None,
            )
            .unwrap();
        drop(a);
        assert!(Runtime::open(
            &d.0,
            config(),
            0,
            SigningKey::from_bytes(&[1; 32]),
            Codec,
            BTreeMap::from([(0, Model::new())])
        )
        .is_err());
    }
}
#[test]
fn structural_ancestry_and_same_tick_order_match_native_contract() {
    let mut root = Message {
        source: 0,
        destination: 1,
        tick: 20,
        root_source: 0,
        root_sequence: 7,
        ancestry: vec![],
        incarnation: 0,
        anti: false,
        payload: vec![],
    };
    let native_root = super::super::OptimisticMessage::try_from_parts(
        RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: Tick::from_ticks(20),
            event_payload: vec![],
        },
        super::super::LogicalEventId::root(LpId(0), 7),
        0,
        super::super::OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(logical_key(&root), native_root.order_key());
    root.source = 1;
    root.destination = 0;
    root.tick = 30;
    root.ancestry.push(Frame {
        tick: 20,
        source: 0,
        ordinal: 0,
    });
    root.validate().unwrap();
    let native_child_id = super::super::LogicalEventId::child(&native_root.order_key(), 0).unwrap();
    let native_child = super::super::OptimisticMessage::try_from_parts(
        RemoteEvent {
            source_lp: LpId(1),
            dest_lp: LpId(0),
            tick: Tick::from_ticks(30),
            event_payload: vec![],
        },
        native_child_id,
        0,
        super::super::OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(logical_key(&root), native_child.order_key());
    let same_tick_root = Message {
        source: 1,
        destination: 0,
        tick: 30,
        root_source: 1,
        root_sequence: 100,
        ancestry: vec![],
        incarnation: 1,
        anti: false,
        payload: vec![],
    };
    assert!(logical_key(&same_tick_root) < logical_key(&root)); // Native Root variant precedes Output.
    root.source = 0;
    root.tick = 40;
    root.ancestry.push(Frame {
        tick: 30,
        source: 1,
        ordinal: 2,
    });
    root.validate().unwrap();
    let grandchild_id = super::super::LogicalEventId::child(&native_child.order_key(), 2).unwrap();
    let native_grandchild = super::super::OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(40),
        LpId(0),
        grandchild_id,
    )
    .unwrap();
    assert_eq!(logical_key(&root), native_grandchild);
}
#[test]
fn signed_receipt_binds_actual_fact_position_revision_and_complete_accounted_state() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    let packet = root(&mut a, 0, 20, 1);
    let replies = a.receive_packet(&packet).unwrap();
    let statement = a.auth(&replies[0]).unwrap();
    let binding = statement.body.durable().unwrap();
    let (revision, raw, sections) = a.store.fact_at(binding.position).unwrap();
    assert_eq!(binding.revision, revision);
    assert_eq!(raw, replies[0]);
    let historical: Image = serde_json::from_slice(&sections[0].1).unwrap();
    assert_eq!(binding.state, Runtime::core_digest(&historical).unwrap());
    assert_eq!(binding.effect, FactEffect::QueuedPositive);
    let before = a.revision().unwrap();
    assert_eq!(a.receive_packet(&packet).unwrap(), replies);
    assert_eq!(a.revision().unwrap(), before);
    let mut forged = statement;
    forged.body.durable_mut().unwrap().position += 1;
    let forged = protocol::sign(&forged, &a.key).unwrap();
    let mut corrupt = a.image.clone();
    corrupt.proofs[0] = forged.clone();
    corrupt
        .facts
        .iter_mut()
        .find(|f| f.packet == packet)
        .unwrap()
        .responses[0] = forged;
    let state = a.store.load().unwrap();
    a.store
        .commit(
            &Unit {
                id: [77; 16],
                previous_revision: state.revision,
                previous_digest: state.digest,
                sections: vec![("engine".into(), bytes(&corrupt).unwrap())],
                facts: vec![],
                outbox: vec![],
            },
            CrashPoint::None,
        )
        .unwrap();
    drop(a);
    assert!(Runtime::open(
        &da.0,
        config(),
        0,
        SigningKey::from_bytes(&[1; 32]),
        Codec,
        BTreeMap::from([(0, Model::new())])
    )
    .is_err());
}
#[test]
fn committed_image_cannot_use_candidate_future_receipt_reservations() {
    let _serial = super::super::external_store::FIXTURE_SERIAL.lock().unwrap();
    let da = Temp::new();
    let db = Temp::new();
    let mut a = open(&da, 0);
    let mut b = open(&db, 1);
    activate(&mut a, &mut b);
    let p = root(&mut a, 0, 20, 1);
    let responses = a.receive_packet(&p).unwrap();
    let prior = a.store.load().unwrap();
    let mut record = a.auth(&responses[0]).unwrap();
    let binding = record.body.durable_mut().unwrap();
    binding.revision = prior.revision + 2;
    binding.position = a.store.next_fact_position().unwrap();
    let forged = protocol::sign(&record, &a.key).unwrap();
    let mut image = a.image.clone();
    image.proofs[0] = forged.clone();
    image
        .facts
        .iter_mut()
        .find(|f| f.packet == p)
        .unwrap()
        .responses[0] = forged;
    a.store
        .commit(
            &Unit {
                id: [88; 16],
                previous_revision: prior.revision,
                previous_digest: prior.digest,
                sections: vec![("engine".into(), bytes(&image).unwrap())],
                facts: vec![],
                outbox: vec![],
            },
            CrashPoint::None,
        )
        .unwrap();
    drop(a);
    assert!(Runtime::open(
        &da.0,
        config(),
        0,
        SigningKey::from_bytes(&[1; 32]),
        Codec,
        BTreeMap::from([(0, Model::new())])
    )
    .is_err());
}
