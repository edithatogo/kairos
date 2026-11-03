// SPDX-License-Identifier: Apache-2.0
//! Public synthetic entity model shared by the real MPI and mTLS qualification workers.
use ed25519_dalek::SigningKey;
use kairo_ecs_pdes::external::protocol::{digest, Hash};
use kairo_ecs_pdes::external::{
    ExternalConfiguration, ExternalOptimisticRuntime, PortableModelCodec,
};
use kairo_ecs_pdes::{
    LpId, OptimisticLimits, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
    PartitionPlan, RemoteEvent, Tick,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub tick: String,
    pub sequence: u64,
    pub delta: i64,
    pub anti: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub index: u64,
    pub generation: u32,
    pub component_generation: u64,
    pub value: i64,
    pub rng: u64,
    pub ownership: u64,
    pub pending: Vec<Pending>,
    pub effects: Vec<(u64, i64, u64)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    pub version: u32,
    pub identity: String,
    pub source: u32,
    pub destination: u32,
    pub at: String,
    pub entity: Entity,
}
impl Transfer {
    pub fn validate(&self, destination: u32, at: u128) -> Result<(), String> {
        if self.version != 1
            || self.identity.is_empty()
            || self.identity.len() > 64
            || self.source == self.destination
            || self.destination != destination
            || parse_tick(&self.at)? != at
            || self.entity.index == 0
            || self.entity.generation == 0
            || self.entity.component_generation == 0
            || self.entity.ownership == 0
            || self.entity.pending.len() > 8
            || self.entity.effects.len() > 8
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 8192
        {
            return Err("invalid entity transfer".into());
        }
        let mut identities = BTreeSet::new();
        let mut previous = None;
        for p in &self.entity.pending {
            let tick = parse_tick(&p.tick)?;
            let key = (tick, p.sequence, p.anti);
            if tick <= at
                || !identities.insert((p.sequence, p.anti))
                || previous.is_some_and(|old| old > key)
            {
                return Err("invalid pending entity work".into());
            }
            previous = Some(key);
        }
        for p in self.entity.pending.iter().filter(|p| p.anti) {
            if !self.entity.pending.iter().any(|q| {
                !q.anti && q.sequence == p.sequence && q.tick == p.tick && q.delta == p.delta
            }) {
                return Err("unmatched pending cancellation".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum Operation {
    Adjust,
    Move,
    Transfer {
        data: Transfer,
    },
    Work {
        entity: u64,
        generation: u32,
        ownership: u64,
        sequence: u64,
        delta: i64,
    },
    Ack {
        sequence: u64,
        value: i64,
        rng: u64,
    },
    Final,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub lp: u32,
    pub generation: u64,
    pub rng: u64,
    pub entities: BTreeMap<u64, Entity>,
    pub transfers: BTreeMap<String, Vec<u8>>,
    pub departures: BTreeMap<u64, u64>,
    pub acknowledgements: Vec<(u64, i64, u64)>,
    pub trace: Vec<(String, String)>,
}
fn rng_step(rng: u64) -> u64 {
    rng.wrapping_mul(6364136223846793005).wrapping_add(1)
}
pub fn tick(t: u128) -> String {
    format!("{t:032x}")
}
pub fn parse_tick(s: &str) -> Result<u128, String> {
    if s.len() != 32
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("canonical full-width tick required".into());
    }
    u128::from_str_radix(s, 16).map_err(|e| e.to_string())
}
impl Model {
    pub fn new(lp: u32) -> Self {
        let pending = [
            (45, 3, 5, false),
            (45, 4, 7, false),
            (47, 6, 1000, false),
            (47, 6, 1000, true),
            (50, 9, 11, false),
        ]
        .into_iter()
        .map(|(at, sequence, delta, anti)| Pending {
            tick: tick(at),
            sequence,
            delta,
            anti,
        })
        .collect();
        let entity = Entity {
            index: 11,
            generation: 7,
            component_generation: 13,
            value: 5,
            rng: 73,
            ownership: 0,
            pending,
            effects: vec![],
        };
        Self {
            lp,
            generation: 1,
            rng: 49,
            entities: if lp == 0 {
                BTreeMap::from([(11, entity)])
            } else {
                BTreeMap::new()
            },
            transfers: BTreeMap::new(),
            departures: BTreeMap::new(),
            acknowledgements: vec![],
            trace: vec![],
        }
    }
    pub fn validate_transfer(&self, data: &Transfer, at: u128) -> Result<(), String> {
        data.validate(self.lp, at)?;
        let raw = serde_json::to_vec(data).map_err(|e| e.to_string())?;
        if let Some(old) = self.transfers.get(&data.identity) {
            return if *old == raw {
                Ok(())
            } else {
                Err("conflicting migration identity".into())
            };
        }
        if self.entities.contains_key(&data.entity.index) {
            return Err("entity already owned".into());
        }
        Ok(())
    }
    fn effect(&mut self, event: &RemoteEvent, op: Operation) -> Result<Vec<RemoteEvent>, String> {
        if event.dest_lp.0 != self.lp {
            return Err("model destination binding".into());
        }
        let at = event.tick.ticks();
        let emit = |destination: u32, when: u128, op: Operation| -> Result<RemoteEvent, String> {
            Ok(RemoteEvent {
                source_lp: event.dest_lp,
                dest_lp: LpId(destination),
                tick: Tick::from_ticks(when),
                event_payload: serde_json::to_vec(&op).map_err(|e| e.to_string())?,
            })
        };
        let label;
        let outputs = match op {
            Operation::Adjust => {
                let entity = self.entities.get_mut(&11).ok_or("adjust absent entity")?;
                entity.value = entity
                    .value
                    .checked_add(100)
                    .ok_or("entity value overflow")?;
                entity.rng = rng_step(entity.rng);
                label = "adjust";
                vec![]
            }
            Operation::Move => {
                let mut entity = self.entities.remove(&11).ok_or("move absent entity")?;
                entity.ownership = entity
                    .ownership
                    .checked_add(1)
                    .ok_or("ownership overflow")?;
                let when = if entity.value >= 100 { 25 } else { 30 };
                let data = Transfer {
                    version: 1,
                    identity: "entity11-root20-transfer0".into(),
                    source: self.lp,
                    destination: 1,
                    at: tick(when),
                    entity,
                };
                data.validate(1, when)?;
                self.departures.insert(11, data.entity.ownership);
                label = "departure";
                vec![emit(1, when, Operation::Transfer { data })?]
            }
            Operation::Transfer { data } => {
                if data.source != event.source_lp.0
                    || event.dest_lp.0 != self.lp
                    || data.source != 0
                    || data.destination != 1
                {
                    return Err("transfer source/event ownership binding".into());
                }
                self.validate_transfer(&data, at)?;
                if self.transfers.contains_key(&data.identity) {
                    return Ok(vec![]);
                }
                let raw = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
                let mut entity = data.entity;
                let cancelled = entity
                    .pending
                    .iter()
                    .filter(|p| p.anti)
                    .map(|p| p.sequence)
                    .collect::<BTreeSet<_>>();
                entity
                    .pending
                    .retain(|p| !p.anti && !cancelled.contains(&p.sequence));
                let mut outputs = Vec::new();
                for p in &entity.pending {
                    outputs.push(emit(
                        self.lp,
                        parse_tick(&p.tick)?,
                        Operation::Work {
                            entity: entity.index,
                            generation: entity.generation,
                            ownership: entity.ownership,
                            sequence: p.sequence,
                            delta: p.delta,
                        },
                    )?);
                }
                self.transfers.insert(data.identity, raw);
                self.entities.insert(entity.index, entity);
                label = "restore";
                outputs
            }
            Operation::Work {
                entity,
                generation,
                ownership,
                sequence,
                delta,
            } => {
                let Some(e) = self.entities.get_mut(&entity) else {
                    return Ok(vec![]);
                };
                if e.generation != generation || e.ownership != ownership {
                    return Ok(vec![]);
                }
                let Some(index) = e.pending.iter().position(|p| {
                    !p.anti && p.sequence == sequence && p.delta == delta && p.tick == tick(at)
                }) else {
                    return Ok(vec![]);
                };
                e.pending.remove(index);
                e.value = e.value.checked_add(delta).ok_or("work value overflow")?;
                e.rng = rng_step(e.rng);
                e.effects.push((sequence, e.value, e.rng));
                label = "work";
                vec![emit(
                    0,
                    at.checked_add(1).ok_or("tick overflow")?,
                    Operation::Ack {
                        sequence,
                        value: e.value,
                        rng: e.rng,
                    },
                )?]
            }
            Operation::Ack {
                sequence,
                value,
                rng,
            } => {
                self.acknowledgements.push((sequence, value, rng));
                label = "ack";
                vec![]
            }
            Operation::Final => {
                label = "final";
                vec![]
            }
        };
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("model generation overflow")?;
        self.rng = rng_step(self.rng);
        self.trace.push((tick(at), label.into()));
        Ok(outputs)
    }
}
impl OptimisticProcess for Model {
    type Snapshot = Self;
    fn snapshot(&self) -> Self {
        self.clone()
    }
    fn restore(&mut self, state: &Self) -> Result<(), OptimisticStateError> {
        *self = state.clone();
        Ok(())
    }
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        let Ok(op) = serde_json::from_slice::<Operation>(&event.event_payload) else {
            return vec![];
        };
        if serde_json::to_vec(&op).ok().as_deref() != Some(event.event_payload.as_slice()) {
            return vec![];
        }
        let mut staged = self.clone();
        match staged.effect(event, op) {
            Ok(outputs) => {
                *self = staged;
                outputs
            }
            Err(_) => vec![],
        }
    }
}
pub struct Codec;
impl PortableModelCodec<Model> for Codec {
    fn identity(&self) -> Hash {
        digest(b"track49-entity-v1-complete-rng-generations-pending-cancellation-fixed-lps")
    }
    fn encode_process(&self, p: &Model) -> Result<Vec<u8>, String> {
        serde_json::to_vec(p).map_err(|e| e.to_string())
    }
    fn decode_process(&self, p: &[u8]) -> Result<Model, String> {
        serde_json::from_slice(p).map_err(|e| e.to_string())
    }
    fn encode_snapshot(&self, p: &Model) -> Result<Vec<u8>, String> {
        self.encode_process(p)
    }
    fn decode_snapshot(&self, p: &[u8]) -> Result<Model, String> {
        self.decode_process(p)
    }
}
pub type Runtime = ExternalOptimisticRuntime<Model, Codec>;
pub fn field<T: serde::de::DeserializeOwned>(v: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(
        v.get(key)
            .cloned()
            .ok_or_else(|| format!("missing {key}"))?,
    )
    .map_err(|e| e.to_string())
}
pub fn number(v: &Value, key: &str) -> Result<u128, String> {
    if let Some(n) = v.get(key).and_then(Value::as_u64) {
        return Ok(u128::from(n));
    }
    parse_tick(&field::<String>(v, key)?)
}
pub fn roots() -> Vec<(u64, u128, Operation)> {
    vec![
        (0, 20, Operation::Move),
        (1, 10, Operation::Adjust),
        (
            2,
            45,
            Operation::Work {
                entity: 11,
                generation: 7,
                ownership: 0,
                sequence: 3,
                delta: 5,
            },
        ),
        (
            3,
            45,
            Operation::Work {
                entity: 11,
                generation: 7,
                ownership: 0,
                sequence: 4,
                delta: 7,
            },
        ),
        (
            4,
            47,
            Operation::Work {
                entity: 11,
                generation: 7,
                ownership: 0,
                sequence: 6,
                delta: 1000,
            },
        ),
        (
            5,
            50,
            Operation::Work {
                entity: 11,
                generation: 7,
                ownership: 0,
                sequence: 9,
                delta: 11,
            },
        ),
        (6, 80, Operation::Final),
    ]
}
pub fn describe(n: u32) -> Value {
    json!({"model":Codec.identity(),"public_keys":(0..n).map(|p|SigningKey::from_bytes(&[(p+1) as u8;32]).verifying_key().to_bytes().to_vec()).collect::<Vec<_>>(),
        "roots":roots().into_iter().map(|(sequence,at,op)|json!({"sequence":sequence,"tick":tick(at),"payload":serde_json::to_vec(&op).unwrap()})).collect::<Vec<_>>()})
}
pub fn serial(n: u32) -> Result<Value, String> {
    let partition = PartitionPlan::from_entities(
        n,
        kairo_ecs_types::SimDuration::from_ticks(1),
        (0..n)
            .map(|p| kairo_ecs_types::EntityId::new(u64::from(p) + 1, 0))
            .collect(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut r = OptimisticRuntime::new(
        partition,
        (0..n)
            .map(|p| (LpId(p), (0..n).filter(|q| *q != p).map(LpId).collect()))
            .collect(),
        (0..n).map(|p| (LpId(p), Model::new(p))).collect(),
        OptimisticLimits::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    for (seq, at, op) in roots() {
        r.schedule_initial(
            seq,
            RemoteEvent {
                source_lp: LpId(0),
                dest_lp: LpId(0),
                tick: Tick::from_ticks(at),
                event_payload: serde_json::to_vec(&op).map_err(|e| e.to_string())?,
            },
        )
        .map_err(|e| format!("{e:?}"))?;
    }
    let progress = r
        .run_until_with_budget(Tick::from_ticks(79), 64)
        .map_err(|e| format!("{e:?}"))?;
    let mut messages = Vec::new();
    for m in &progress.published_messages {
        let mut logical = m.logical_id();
        let mut ancestry = Vec::new();
        while let Some((parent, ordinal)) = logical.output_parts() {
            ancestry.push(kairo_ecs_pdes::external::protocol::Frame {
                tick: parent.tick().ticks(),
                source: parent.source_lp().0,
                ordinal,
            });
            logical = parent.logical_id();
        }
        ancestry.reverse();
        let (root_source, root_sequence) = logical.root_parts().ok_or("native root")?;
        messages.push(kairo_ecs_pdes::external::protocol::Message {
            source: m.event().source_lp.0,
            destination: m.event().dest_lp.0,
            tick: m.event().tick.ticks(),
            root_source: root_source.0,
            root_sequence,
            ancestry,
            incarnation: m.incarnation(),
            anti: false,
            payload: m.event().event_payload.clone(),
        });
    }
    Ok(
        json!({"models":(0..n).map(|p|Codec.encode_process(r.process_at(LpId(p)).unwrap()).unwrap()).collect::<Vec<_>>(),"messages":messages}),
    )
}
pub fn open(
    path: &Path,
    config: ExternalConfiguration,
    participant: u32,
) -> Result<Runtime, String> {
    let models = config
        .members
        .iter()
        .find(|m| m.participant == participant)
        .ok_or("participant")?
        .lps
        .iter()
        .map(|lp| (*lp, Model::new(*lp)))
        .collect();
    Runtime::open(
        path,
        config,
        participant,
        SigningKey::from_bytes(&[(participant + 1) as u8; 32]),
        Codec,
        models,
    )
    .map_err(|e| e.to_string())
}
pub fn local(v: &Value, r: &mut Runtime, participant: u32, size: u32) -> Result<Value, String> {
    let op = v.get("op").and_then(Value::as_str).ok_or("op")?;
    let result = match op {
        "describe" => describe(size),
        "serial" => serial(size)?,
        "snapshot" => {
            json!({"checkpoint":r.checkpoint().map_err(|e|e.to_string())?,"model":r.model_bytes(participant),"generation":r.generation(),"gvt":tick(r.gvt()),"active":r.is_active(),"revision":r.revision().map_err(|e|e.to_string())?,"counters":r.counters()})
        }
        "pending" => json!({"packets":r.pending_packets().map_err(|e|e.to_string())?}),
        "propose" => json!({"packet":r.activation_proposal().map_err(|e|e.to_string())?}),
        "join" => {
            json!({"packet":r.join_activation(&field::<Vec<u8>>(v,"packet")?).map_err(|e|e.to_string())?})
        }
        "activate" => {
            r.activate(
                &field::<Vec<u8>>(v, "packet")?,
                &field::<Vec<Vec<u8>>>(v, "joins")?,
            )
            .map_err(|e| e.to_string())?;
            json!({})
        }
        "activation" => json!({"decision":r.retained_activation()}),
        "root" => {
            r.schedule_initial(
                field(v, "sequence")?,
                RemoteEvent {
                    source_lp: LpId(field(v, "source")?),
                    dest_lp: LpId(field(v, "destination")?),
                    tick: Tick::from_ticks(number(v, "tick")?),
                    event_payload: field(v, "payload")?,
                },
            )
            .map_err(|e| e.to_string())?;
            json!({})
        }
        "receive" => {
            json!({"replies":r.receive_packet(&field::<Vec<u8>>(v,"packet")?).map_err(|e|e.to_string())?})
        }
        "recover" => {
            json!({"replies":r.recover_delivery(&field::<Vec<u8>>(v,"packet")?).map_err(|e|e.to_string())?})
        }
        "run" => {
            json!({"used":r.run_until_with_budget(number(v,"horizon")?,field(v,"budget")?).map_err(|e|e.to_string())?})
        }
        "close" => {
            r.close_initial_inputs().map_err(|e| e.to_string())?;
            json!({})
        }
        "pause" => json!({"packet":r.pause_cut().map_err(|e|e.to_string())?}),
        "prepare" => json!({"packet":r.prepare_cut(number(v,"limit")?).map_err(|e|e.to_string())?}),
        "cut" => json!({"packet":r.commit_cut(field(v,"prepares")?).map_err(|e|e.to_string())?}),
        "recover_cut" => {
            r.recover_cut(&field::<Vec<u8>>(v, "packet")?)
                .map_err(|e| e.to_string())?;
            json!({})
        }
        "cut_decision" => json!({"packet":r.retained_cut_decision()}),
        "fault" => {
            r.qualify_pause_next_commit(field(v, "after")?);
            json!({})
        }
        "validate_transfer" => {
            let m: Model = serde_json::from_slice(r.model_bytes(participant).ok_or("model")?)
                .map_err(|e| e.to_string())?;
            m.validate_transfer(&field(v, "data")?, number(v, "tick")?)?;
            json!({})
        }
        "validate_event" => {
            let mut model: Model =
                serde_json::from_slice(r.model_bytes(participant).ok_or("model")?)
                    .map_err(|e| e.to_string())?;
            let payload: Vec<u8> = field(v, "payload")?;
            let operation: Operation =
                serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
            model.effect(
                &RemoteEvent {
                    source_lp: LpId(field(v, "source")?),
                    dest_lp: LpId(participant),
                    tick: Tick::from_ticks(number(v, "tick")?),
                    event_payload: payload,
                },
                operation,
            )?;
            json!({})
        }
        "sign_fixture" => {
            let record: kairo_ecs_pdes::external::protocol::Record = field(v, "record")?;
            json!({"packet":kairo_ecs_pdes::external::protocol::sign(&record,&SigningKey::from_bytes(&[(record.issuer+1)as u8;32]))?})
        }
        _ => return Err("unknown owned fixture command".into()),
    };
    Ok(result)
}
