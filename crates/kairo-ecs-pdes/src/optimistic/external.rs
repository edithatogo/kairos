// SPDX-License-Identifier: Apache-2.0
//! Durable, separately authenticated Time Warp participants. Native capabilities
//! never enter this boundary. A reopened participant is always standby.
pub mod protocol;
use super::external_store::{CrashPoint, Store, Unit};
use super::{LpId, OptimisticProcess, RemoteEvent, Tick};
use ed25519_dalek::SigningKey;
use protocol::{digest, Body, DurableBinding, FactEffect, Frame, Hash, Message, Record};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageValidation {
    Committed,
    Candidate,
}
const MAX_ITEMS: usize = 64;
const MAX_IMAGE: usize = 1024 * 1024;
type Result<T> = std::result::Result<T, ExternalError>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalError {
    Rejected(String),
    Model(String),
    Storage(String),
    Standby,
    Paused,
    Fenced,
}
impl std::fmt::Display for ExternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ExternalError {}
fn reject(s: impl Into<String>) -> ExternalError {
    ExternalError::Rejected(s.into())
}
/// The codec must include every model field, component generation, RNG state and
/// reversible output in process and historical snapshots. No external effects.
pub trait PortableModelCodec<P: OptimisticProcess> {
    fn identity(&self) -> Hash;
    fn encode_process(&self, p: &P) -> std::result::Result<Vec<u8>, String>;
    fn decode_process(&self, b: &[u8]) -> std::result::Result<P, String>;
    fn encode_snapshot(&self, s: &P::Snapshot) -> std::result::Result<Vec<u8>, String>;
    fn decode_snapshot(&self, b: &[u8]) -> std::result::Result<P::Snapshot, String>;
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExternalMember {
    pub participant: u32,
    pub public_key: Vec<u8>,
    pub certificate_hash: Hash,
    pub lps: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExternalConfiguration {
    pub simulation: [u8; 16],
    pub model: Hash,
    pub members: Vec<ExternalMember>,
    pub topology: Vec<(u32, Vec<u32>)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Execution {
    message: Message,
    before: Vec<u8>,
    outputs: Vec<Message>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProcessImage {
    lp: u32,
    initial: Vec<u8>,
    current: Vec<u8>,
    pending: Vec<Message>,
    history: Vec<Execution>,
    tombstones: Vec<Message>,
    replay: Vec<Hash>,
    epoch: u64,
    fossil: u128,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Outgoing {
    message: Message,
    packet: Vec<u8>,
    dependencies: Vec<Hash>,
    admitted: bool,
    retired: bool,
    suppressed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Fact {
    packet: Vec<u8>,
    responses: Vec<Vec<u8>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Reversal {
    delivery: Hash,
    packet: Vec<u8>,
    dependencies: Vec<Hash>,
    applied: bool,
    receipt: Option<Vec<u8>>,
    effect: FactEffect,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Image {
    version: u32,
    context: Hash,
    participant: u32,
    generation: u64,
    active: bool,
    paused: Option<u64>,
    pause_packet: Option<Vec<u8>>,
    prepared: Option<Vec<u8>>,
    joining: Option<Hash>,
    processes: Vec<ProcessImage>,
    next_incarnation: Vec<(u32, Option<u64>)>,
    roots: Vec<(u32, u64)>,
    known: Vec<Message>,
    outgoing: Vec<Outgoing>,
    facts: Vec<Fact>,
    proofs: Vec<Vec<u8>>,
    reversals: Vec<Reversal>,
    barriers: Vec<Hash>,
    sequence: Option<u64>,
    gvt: u128,
    cut: u64,
    executions: u64,
    rollbacks: u64,
    replays: u64,
    last_horizon: Option<u128>,
    initial_open: bool,
    poisoned: bool,
    recovery: Vec<(u32, Vec<Hash>)>,
    activation: Option<Vec<u8>>,
    proposal: Option<Vec<u8>>,
    joins: Vec<Vec<u8>>,
    cut_decision: Option<Vec<u8>>,
}
/// One fixed verifier, store and staged engine. Transport cannot mint a proof.
pub struct ExternalOptimisticRuntime<P: OptimisticProcess, C: PortableModelCodec<P>> {
    store: Store,
    config: ExternalConfiguration,
    context: Hash,
    participant: u32,
    key: SigningKey,
    codec: C,
    image: Image,
    fenced: bool,
    fault: CrashPoint,
    marker: PhantomData<fn() -> P>,
}
struct ImageWriter(Vec<u8>);
impl std::io::Write for ImageWriter {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if b.len() > MAX_IMAGE.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("image bound"));
        }
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn bytes<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    let mut out = ImageWriter(Vec::new());
    serde_json::to_writer(&mut out, v).map_err(|e| reject(e.to_string()))?;
    Ok(out.0)
}

fn image_unit_digest(revision: u64, raw: &[u8]) -> Result<Hash> {
    use sha2::{Digest, Sha256};
    use std::io::Write;

    if raw.len() > MAX_IMAGE {
        return Err(reject("image bound"));
    }
    // The unchanged flat JSON tuple is bounded by 4 * MAX_IMAGE + 24 bytes.
    // Its byte-array encoding is a hash preimage, not another retained image.
    // Count first to preserve protocol::digest's explicit length prefix, then
    // stream the same tuple without a second retained-image allocation.
    struct LengthWriter(usize);
    impl std::io::Write for LengthWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > (4 * MAX_IMAGE + 24).saturating_sub(self.0) {
                return Err(std::io::Error::other("unit digest preimage bound"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut length = LengthWriter(0);
    serde_json::to_writer(&mut length, &(revision, raw)).map_err(|e| reject(e.to_string()))?;
    struct DigestWriter {
        hash: Sha256,
        remaining: usize,
    }
    impl std::io::Write for DigestWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.remaining {
                return Err(std::io::Error::other("unit digest length mismatch"));
            }
            self.hash.update(bytes);
            self.remaining -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut hash = Sha256::new();
    hash.update(b"KAIROS-EXTERNAL-DIGEST-V1\0");
    hash.update((length.0 as u64).to_be_bytes());
    let mut writer = DigestWriter {
        hash,
        remaining: length.0,
    };
    // Serde emits many small numeric fragments. Keep SHA updates in bounded
    // chunks while preserving the exact preimage and checking the final flush.
    let mut buffered = std::io::BufWriter::with_capacity(8 * 1024, &mut writer);
    serde_json::to_writer(&mut buffered, &(revision, raw)).map_err(|e| reject(e.to_string()))?;
    buffered.flush().map_err(|e| reject(e.to_string()))?;
    drop(buffered);
    if writer.remaining != 0 {
        return Err(reject("unit digest length mismatch"));
    }
    Ok(writer.hash.finalize().into())
}

fn identity(m: &Message) -> Hash {
    digest(
        &serde_json::to_vec(&(
            m.source,
            m.root_source,
            m.root_sequence,
            &m.ancestry,
            m.incarnation,
        ))
        .expect("fixed data"),
    )
}
fn positive(m: &Message) -> Message {
    let mut p = m.clone();
    p.anti = false;
    p
}
fn packet_id(p: &[u8]) -> Hash {
    digest(p)
}
fn finished(image: &Image, id: &Hash) -> bool {
    image
        .outgoing
        .iter()
        .any(|o| packet_id(&o.packet) == *id && o.message.anti && o.retired)
}
impl<P: OptimisticProcess, C: PortableModelCodec<P>> ExternalOptimisticRuntime<P, C> {
    pub fn open(
        path: &Path,
        config: ExternalConfiguration,
        participant: u32,
        key: SigningKey,
        codec: C,
        models: BTreeMap<u32, P>,
    ) -> Result<Self> {
        catch_unwind(AssertUnwindSafe(|| {
            Self::open_inner(path, config, participant, key, codec, models)
        }))
        .map_err(|_| ExternalError::Model("model/codec construction panicked".into()))?
    }
    fn open_inner(
        path: &Path,
        config: ExternalConfiguration,
        participant: u32,
        key: SigningKey,
        codec: C,
        models: BTreeMap<u32, P>,
    ) -> Result<Self> {
        if config.members.is_empty()
            || config.members.len() > MAX_ITEMS
            || config.model != codec.identity()
            || config.simulation == [0; 16]
        {
            return Err(reject("configuration"));
        }
        let mut members = BTreeSet::new();
        let mut lps = BTreeSet::new();
        let mut own = BTreeSet::new();
        for m in &config.members {
            if !members.insert(m.participant)
                || m.public_key.len() != 32
                || m.certificate_hash == [0; 32]
                || m.lps.is_empty()
            {
                return Err(reject("membership"));
            }
            for lp in &m.lps {
                if !lps.insert(*lp) {
                    return Err(reject("LP overlap"));
                }
                if m.participant == participant {
                    own.insert(*lp);
                }
            }
        }
        if config.members.first().map(|m| m.participant) != Some(0)
            || own.is_empty()
            || own != models.keys().copied().collect()
            || lps.len() > MAX_ITEMS
        {
            return Err(reject("owned set"));
        }
        let member = config
            .members
            .iter()
            .find(|m| m.participant == participant)
            .ok_or_else(|| reject("member"))?;
        if member.public_key != key.verifying_key().to_bytes() {
            return Err(reject("signing identity"));
        }
        let mut routes = BTreeSet::new();
        for (lp, neighbors) in &config.topology {
            if !lps.contains(lp)
                || !routes.insert(*lp)
                || neighbors.len() > MAX_ITEMS
                || neighbors.iter().collect::<BTreeSet<_>>().len() != neighbors.len()
                || neighbors.iter().any(|p| !lps.contains(p))
            {
                return Err(reject("topology"));
            }
        }
        if routes != lps {
            return Err(reject("topology coverage"));
        }
        let context = digest(&bytes(&config)?);
        let mut processes = Vec::new();
        for (lp, p) in models {
            let b = codec.encode_process(&p).map_err(ExternalError::Model)?;
            let s = codec
                .encode_snapshot(&p.snapshot())
                .map_err(ExternalError::Model)?;
            if b.len() > 65536 || s.len() > 65536 {
                return Err(reject("model bound"));
            }
            processes.push(ProcessImage {
                lp,
                initial: b.clone(),
                current: b,
                pending: vec![],
                history: vec![],
                tombstones: vec![],
                replay: vec![],
                epoch: 0,
                fossil: 0,
            });
        }
        let initial = Image {
            version: 1,
            context,
            participant,
            generation: 0,
            active: false,
            paused: None,
            pause_packet: None,
            prepared: None,
            joining: None,
            next_incarnation: own.iter().map(|p| (*p, Some(0))).collect(),
            processes,
            roots: vec![],
            known: vec![],
            outgoing: vec![],
            facts: vec![],
            proofs: vec![],
            reversals: vec![],
            barriers: vec![],
            sequence: Some(0),
            gvt: 0,
            cut: 0,
            executions: 0,
            rollbacks: 0,
            replays: 0,
            last_horizon: None,
            initial_open: true,
            poisoned: false,
            recovery: vec![],
            activation: None,
            proposal: None,
            joins: vec![],
            cut_decision: None,
        };
        let store = Store::open(
            path,
            context,
            vec!["engine".into()],
            vec![("engine".into(), bytes(&initial)?)],
        )
        .map_err(|e| ExternalError::Storage(e.to_string()))?;
        let saved = store
            .load()
            .map_err(|e| ExternalError::Storage(e.to_string()))?;
        let raw = &saved.sections[0].1;
        if raw.len() > MAX_IMAGE {
            return Err(reject("image bound"));
        }
        let mut image: Image = serde_json::from_slice(raw).map_err(|e| reject(e.to_string()))?;
        if bytes(&image)? != *raw
            || image.context != context
            || image.participant != participant
            || image.version != 1
        {
            return Err(reject("image identity"));
        }
        image.active = false; // Never resurrect a previously active lease from disk.
        let result = Self {
            store,
            config,
            context,
            participant,
            key,
            codec,
            image,
            fenced: false,
            fault: CrashPoint::None,
            marker: PhantomData,
        };
        result.validate(&result.image, ImageValidation::Committed)?;
        Ok(result)
    }
    fn keys(&self) -> BTreeMap<u32, Vec<u8>> {
        self.config
            .members
            .iter()
            .map(|m| (m.participant, m.public_key.clone()))
            .collect()
    }
    fn owner(&self, lp: u32) -> Result<u32> {
        self.config
            .members
            .iter()
            .find(|m| m.lps.contains(&lp))
            .map(|m| m.participant)
            .ok_or_else(|| reject("unknown LP"))
    }
    fn auth(&self, p: &[u8]) -> Result<Record> {
        protocol::verify(p, self.context, &self.keys()).map_err(reject)
    }
    fn sign(&self, image: &mut Image, mut body: Body) -> Result<Vec<u8>> {
        let sequence = image.sequence.ok_or_else(|| reject("sequence exhausted"))?;
        image.sequence = sequence.checked_add(1);
        if let Some(binding) = body.durable_mut() {
            binding.revision = self
                .revision()?
                .checked_add(1)
                .ok_or_else(|| reject("revision exhausted"))?;
            let pending = image
                .proofs
                .iter()
                .filter(|p| !self.image.proofs.contains(p))
                .count() as u64;
            binding.position = self
                .store
                .next_fact_position()
                .map_err(|e| ExternalError::Storage(e.to_string()))?
                .checked_add(pending)
                .ok_or_else(|| reject("fact position exhausted"))?;
            binding.state = [1; 32];
        }
        let durable = body.durable().is_some();
        let packet = protocol::sign(
            &Record {
                version: 1,
                context: self.context,
                issuer: self.participant,
                generation: image.generation.max(1),
                sequence,
                body,
            },
            &self.key,
        )
        .map_err(reject)?;
        if durable {
            image.proofs.push(packet.clone());
        }
        Ok(packet)
    }
    fn core_digest(image: &Image) -> Result<Hash> {
        let mut core = image.clone();
        core.proofs.clear();
        core.sequence = None;
        for f in &mut core.facts {
            f.responses.clear();
        }
        for r in &mut core.reversals {
            r.receipt = None;
        }
        Ok(digest(&bytes(&core)?))
    }

    fn validate(&self, image: &Image, mode: ImageValidation) -> Result<()> {
        if image.proofs.len() > MAX_ITEMS {
            return Err(reject("durable proof bound"));
        }
        if image.poisoned
            || image.processes.len() > MAX_ITEMS
            || image.known.len() > MAX_ITEMS
            || image.outgoing.len() > MAX_ITEMS
            || image.facts.len() > MAX_ITEMS
            || image.reversals.len() > MAX_ITEMS
            || image.barriers.len() > MAX_ITEMS
            || image.roots.len() > MAX_ITEMS
        {
            return Err(reject("bounded image"));
        }
        let expected = self
            .config
            .members
            .iter()
            .find(|m| m.participant == self.participant)
            .unwrap()
            .lps
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if image
            .processes
            .iter()
            .map(|p| p.lp)
            .collect::<BTreeSet<_>>()
            != expected
            || image.processes.len() != expected.len()
            || image
                .next_incarnation
                .iter()
                .map(|x| x.0)
                .collect::<BTreeSet<_>>()
                != expected
        {
            return Err(reject("image LP set"));
        }
        let mut known = BTreeMap::new();
        for m in &image.known {
            self.validate_message(m)?;
            if m.anti || known.insert(identity(m), m).is_some() {
                return Err(reject("known index"));
            }
        }
        for p in &image.processes {
            if p.pending.len() + p.history.len() + p.tombstones.len() > MAX_ITEMS
                || p.current.len() > 65536
                || p.initial.len() > 65536
                || p.fossil != image.gvt
            {
                return Err(reject("process bounds"));
            }
            self.codec
                .decode_process(&p.current)
                .map_err(ExternalError::Model)?;
            self.codec
                .decode_process(&p.initial)
                .map_err(ExternalError::Model)?;
            let mut queued = BTreeSet::new();
            for m in &p.pending {
                if m.destination != p.lp
                    || m.tick < image.gvt
                    || !queued.insert((identity(m), m.anti))
                    || known.get(&identity(m)).copied() != Some(&positive(m))
                {
                    return Err(reject("pending index"));
                }
            }
            for e in &p.history {
                if e.message.destination != p.lp
                    || e.message.anti
                    || e.before.len() > 65536
                    || e.outputs.len() > MAX_ITEMS
                    || known.get(&identity(&e.message)).copied() != Some(&e.message)
                {
                    return Err(reject("history index"));
                }
                self.codec
                    .decode_snapshot(&e.before)
                    .map_err(ExternalError::Model)?;
                for m in &e.outputs {
                    self.validate_message(m)?;
                }
            }
            if p.history
                .windows(2)
                .any(|w| order(&w[0].message) >= order(&w[1].message))
            {
                return Err(reject("history order"));
            }
            for m in &p.tombstones {
                if m.destination != p.lp || m.anti || known.get(&identity(m)).copied() != Some(m) {
                    return Err(reject("tombstone index"));
                }
            }
        }
        if image.next_incarnation.len() != expected.len()
            || image.roots.iter().collect::<BTreeSet<_>>().len() != image.roots.len()
        {
            return Err(reject("allocator/root duplicate"));
        }
        for (lp, next) in &image.next_incarnation {
            let maximum = image
                .outgoing
                .iter()
                .filter(|o| o.message.source == *lp && !o.message.anti)
                .map(|o| o.message.incarnation)
                .max();
            if maximum.is_some_and(|m| next.is_some_and(|n| n <= m))
                || (next.is_none() && maximum != Some(u64::MAX))
            {
                return Err(reject("allocator sentinel/reservation"));
            }
        }
        let roots = image
            .outgoing
            .iter()
            .filter(|o| !o.message.anti && o.message.ancestry.is_empty())
            .map(|o| (o.message.source, o.message.root_sequence))
            .collect::<BTreeSet<_>>();
        if roots != image.roots.iter().copied().collect() {
            return Err(reject("root lifetime reservation"));
        }
        for p in &image.processes {
            if p.tombstones
                .iter()
                .map(identity)
                .collect::<BTreeSet<_>>()
                .len()
                != p.tombstones.len()
                || p.replay.iter().collect::<BTreeSet<_>>().len() != p.replay.len()
                || p.replay
                    .iter()
                    .any(|id| !p.pending.iter().any(|m| !m.anti && identity(m) == *id))
            {
                return Err(reject("tombstone/replay index"));
            }
        }
        for (index, o) in image.outgoing.iter().enumerate() {
            if o.dependencies.iter().collect::<BTreeSet<_>>().len() != o.dependencies.len()
                || o.dependencies.iter().any(|d| {
                    !image.outgoing[..index]
                        .iter()
                        .any(|prior| prior.message.anti && packet_id(&prior.packet) == *d)
                })
            {
                return Err(reject("dependency DAG"));
            }
        }
        let mut incoming = BTreeSet::new();
        for f in &image.facts {
            if !incoming.insert(packet_id(&f.packet)) {
                return Err(reject("duplicate durable fact"));
            }
        }
        for r in &image.reversals {
            let record = self.auth(&r.packet)?;
            let Body::Delivery { message } = record.body else {
                return Err(reject("reversal packet"));
            };
            if !message.anti
                || r.delivery != packet_id(&r.packet)
                || self.owner(message.destination)? != self.participant
                || !image.facts.iter().any(|f| f.packet == r.packet)
                || r.dependencies.iter().any(|d| {
                    !image
                        .outgoing
                        .iter()
                        .any(|o| o.message.anti && packet_id(&o.packet) == *d)
                })
            {
                return Err(reject("reversal index"));
            }
            if let Some(receipt) = &r.receipt {
                let receipt = self.auth(receipt)?;
                if receipt.issuer != self.participant
                    || !r.applied
                    || !matches!(receipt.body,Body::Retirement{delivery,dependencies,..} if delivery==r.delivery&&dependencies==r.dependencies)
                    || r.dependencies.iter().any(|d| !finished(image, d))
                {
                    return Err(reject("retirement closure"));
                }
            }
        }
        for packet in &image.proofs {
            if !image.facts.iter().any(|f| f.responses.contains(packet))
                && !image
                    .reversals
                    .iter()
                    .any(|r| r.receipt.as_ref() == Some(packet))
            {
                return Err(reject("orphan durable proof"));
            }
        }
        for f in &image.facts {
            for response in &f.responses {
                if self.auth(response)?.body.durable().is_some() && !image.proofs.contains(response)
                {
                    return Err(reject("missing durable proof index"));
                }
            }
        }
        for reversal in &image.reversals {
            if reversal
                .receipt
                .as_ref()
                .is_some_and(|p| !image.proofs.contains(p))
            {
                return Err(reject("missing retirement proof index"));
            }
        }
        let read = self
            .store
            .validated_read()
            .map_err(|e| ExternalError::Storage(e.to_string()))?;
        let current = read.revision();
        let mut proof_positions = BTreeSet::new();
        for packet in &image.proofs {
            let record = self.auth(packet)?;
            let binding = record
                .body
                .durable()
                .ok_or_else(|| reject("proof record kind"))?;
            if record.issuer != self.participant
                || !proof_positions.insert(binding.position)
                || binding.revision > current + 1
                || (binding.revision > current
                    && (mode == ImageValidation::Committed || self.image.proofs.contains(packet)))
            {
                return Err(reject("proof issuer/revision/position"));
            }
            if binding.revision <= current {
                let (revision, raw, sections) = read
                    .fact_at(binding.position)
                    .map_err(|e| ExternalError::Storage(e.to_string()))?;
                if revision != binding.revision || raw != *packet {
                    return Err(reject("durable fact linkage"));
                }
                let historical: Image =
                    serde_json::from_slice(&sections[0].1).map_err(|e| reject(e.to_string()))?;
                if Self::core_digest(&historical)? != binding.state {
                    return Err(reject("durable accounted-state digest"));
                }
            }
        }
        if mode == ImageValidation::Candidate {
            let first = read.next_fact_position();
            let mut reserved = image
                .proofs
                .iter()
                .filter_map(|p| self.auth(p).ok())
                .filter_map(|r| r.body.durable().cloned())
                .filter(|b| b.revision > current)
                .map(|b| b.position)
                .collect::<Vec<_>>();
            reserved.sort();
            if reserved
                .iter()
                .enumerate()
                .any(|(index, p)| *p != first + index as u64)
            {
                return Err(reject("durable position reservation"));
            }
        }
        let mut packets = BTreeSet::new();
        for o in &image.outgoing {
            let r = self.auth(&o.packet)?;
            if r.issuer != self.participant
                || !matches!(r.body,Body::Delivery{ref message} if message==&o.message)
                || !packets.insert(packet_id(&o.packet))
                || o.dependencies.len() > MAX_ITEMS
            {
                return Err(reject("outbox"));
            }
        }
        for b in &image.barriers {
            if !image
                .outgoing
                .iter()
                .any(|o| o.message.anti && packet_id(&o.packet) == *b)
            {
                return Err(reject("barrier"));
            }
        }
        for f in &image.facts {
            self.auth(&f.packet)?;
            for r in &f.responses {
                if self.auth(r)?.issuer != self.participant {
                    return Err(reject("fact issuer"));
                }
            }
        }
        bytes(image)?;
        Ok(())
    }
    fn validate_message(&self, m: &Message) -> Result<()> {
        m.validate().map_err(reject)?;
        self.owner(m.root_source)?;
        for frame in &m.ancestry {
            self.owner(frame.source)?;
            if frame.ordinal as usize >= MAX_ITEMS {
                return Err(reject("output ordinal bound"));
            }
        }
        self.owner(m.source)?;
        self.owner(m.destination)?;
        if m.source != m.destination
            && !self
                .config
                .topology
                .iter()
                .any(|(lp, n)| *lp == m.source && n.contains(&m.destination))
        {
            return Err(reject("route"));
        }
        Ok(())
    }
    fn commit(&mut self, mut next: Image, records: Vec<Vec<u8>>) -> Result<()> {
        if self.fenced {
            return Err(ExternalError::Fenced);
        }
        let state = Self::core_digest(&next)?;
        let mut new_proofs = Vec::new();
        for index in 0..next.proofs.len() {
            let old = next.proofs[index].clone();
            if self.image.proofs.contains(&old) {
                continue;
            }
            let mut record = self.auth(&old)?;
            record
                .body
                .durable_mut()
                .ok_or_else(|| reject("proof kind"))?
                .state = state;
            let sealed = protocol::sign(&record, &self.key).map_err(reject)?;
            for f in &mut next.facts {
                for response in &mut f.responses {
                    if *response == old {
                        *response = sealed.clone();
                    }
                }
            }
            for r in &mut next.reversals {
                if r.receipt.as_ref() == Some(&old) {
                    r.receipt = Some(sealed.clone());
                }
            }
            next.proofs[index] = sealed.clone();
            new_proofs.push((record.body.durable().unwrap().position, sealed));
        }
        if state != Self::core_digest(&next)? {
            return Err(reject("noncircular accounted-state binding"));
        }
        new_proofs.sort_by_key(|(position, _)| *position);
        let expected_positions = new_proofs
            .iter()
            .map(|(p, _)| *p as i64)
            .collect::<Vec<_>>();
        let mut facts = new_proofs.into_iter().map(|(_, p)| p).collect::<Vec<_>>();
        for p in records {
            if self.auth(&p)?.body.durable().is_none() {
                facts.push(p);
            }
        }
        match catch_unwind(AssertUnwindSafe(|| {
            self.validate(&next, ImageValidation::Candidate)
        })) {
            Ok(r) => r?,
            Err(_) => {
                self.fenced = true;
                return Err(ExternalError::Model(
                    "codec validation panicked; owner fenced".into(),
                ));
            }
        }
        let raw = bytes(&next)?;
        let old = self
            .store
            .load()
            .map_err(|e| ExternalError::Storage(e.to_string()))?;
        let h = image_unit_digest(old.revision, &raw)?;
        let unit = Unit {
            id: h[..16].try_into().unwrap(),
            previous_revision: old.revision,
            previous_digest: old.digest,
            sections: vec![("engine".into(), raw)],
            facts,
            outbox: vec![],
        };
        let fault = self.fault;
        self.fault = CrashPoint::None;
        let receipt = self.store.commit(&unit, fault).map_err(|e| {
            self.fenced = true;
            ExternalError::Storage(e.to_string())
        })?;
        if !receipt.positions.starts_with(&expected_positions) {
            self.fenced = true;
            return Err(ExternalError::Storage(
                "journal allocation differs from durable reservation".into(),
            ));
        }
        self.image = next;
        Ok(())
    }
    /// Qualification-only one-shot pause at the real SQL commit boundary. Never
    /// enabled by default or by the ordinary external runtime feature.
    #[cfg(feature = "external-fault-injection")]
    pub fn qualify_pause_next_commit(&mut self, after: bool) {
        self.fault = if after {
            CrashPoint::AfterCommit
        } else {
            CrashPoint::BeforeCommit
        };
    }
    fn healthy(&self) -> Result<()> {
        if self.fenced || self.image.poisoned {
            Err(ExternalError::Fenced)
        } else {
            Ok(())
        }
    }
    fn runnable(&self) -> Result<()> {
        self.healthy()?;
        if !self.image.active {
            return Err(ExternalError::Standby);
        }
        if self.image.paused.is_some() || self.image.joining.is_some() {
            return Err(ExternalError::Paused);
        }
        Ok(())
    }
    pub fn generation(&self) -> u64 {
        self.image.generation
    }
    pub fn is_active(&self) -> bool {
        self.image.active && !self.fenced
    }
    pub fn model_bytes(&self, lp: u32) -> Option<&[u8]> {
        self.image
            .processes
            .iter()
            .find(|p| p.lp == lp)
            .map(|p| p.current.as_slice())
    }
    pub fn checkpoint(&self) -> Result<Vec<u8>> {
        bytes(&self.image)
    }
    pub fn revision(&self) -> Result<u64> {
        self.store
            .load()
            .map(|s| s.revision)
            .map_err(|e| ExternalError::Storage(e.to_string()))
    }
    pub fn gvt(&self) -> u128 {
        self.image.gvt
    }
    pub fn counters(&self) -> (u64, u64, u64) {
        (
            self.image.executions,
            self.image.rollbacks,
            self.image.replays,
        )
    }
    pub fn activation_proposal(&mut self) -> Result<Vec<u8>> {
        self.healthy()?;
        if self.participant != 0 || self.image.prepared.is_some() {
            return Err(reject("activation coordinator"));
        }
        if let Some(p) = &self.image.proposal {
            return Ok(p.clone());
        }
        let mut next = self.image.clone();
        let generation = next
            .generation
            .checked_add(1)
            .ok_or_else(|| reject("generation exhausted"))?;
        let nonce = digest(&bytes(&(self.context, next.generation, generation))?);
        let p = self.sign(&mut next, Body::Activation { generation, nonce })?;
        next.proposal = Some(p.clone());
        self.commit(next, vec![p.clone()])?;
        Ok(p)
    }
    pub fn join_activation(&mut self, proposal: &[u8]) -> Result<Vec<u8>> {
        self.healthy()?;
        let r = self.auth(proposal)?;
        let Body::Activation { generation, nonce } = r.body else {
            return Err(reject("activation proposal"));
        };
        let h = packet_id(proposal);
        if self.image.joining.is_some_and(|existing| existing != h) {
            return Err(reject("already joined another successor proposal"));
        }
        if let Some(f) = self.image.facts.iter().find(|f| f.packet == proposal) {
            return f
                .responses
                .first()
                .cloned()
                .ok_or_else(|| reject("join receipt"));
        }
        if r.issuer != 0
            || r.generation != self.image.generation.max(1)
            || generation
                != self
                    .image
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| reject("generation"))?
            || nonce != digest(&bytes(&(self.context, self.image.generation, generation))?)
            || self.image.prepared.is_some()
        {
            return Err(reject("successor manifest"));
        }
        let mut next = self.image.clone();
        next.active = false;
        next.joining = Some(h);
        let checkpoint = digest(&bytes(&next)?);
        let retained = next
            .outgoing
            .iter()
            .filter(|o| !o.admitted)
            .map(|o| packet_id(&o.packet))
            .collect();
        let frontiers = self.frontiers(&next)?;
        let join = self.sign(
            &mut next,
            Body::Join {
                participant: 0,
                generation,
                activation: h,
                floor: self.image.gvt,
                cut: self.image.cut,
                checkpoint,
                frontiers,
                retained,
            },
        )?;
        next.facts.push(Fact {
            packet: proposal.to_vec(),
            responses: vec![join.clone()],
        });
        self.commit(next, vec![join.clone()])?;
        Ok(join)
    }
    pub fn activate(&mut self, proposal: &[u8], joins: &[Vec<u8>]) -> Result<()> {
        self.healthy()?;
        if self.image.activation.as_deref() == Some(proposal) && self.image.joins == joins {
            return Ok(());
        }
        let r = self.auth(proposal)?;
        let Body::Activation { generation, .. } = r.body else {
            return Err(reject("activation"));
        };
        if r.issuer != 0
            || self.image.joining != Some(packet_id(proposal))
            || joins.len() != self.config.members.len()
        {
            return Err(reject("incomplete successor join"));
        }
        let mut issuers = BTreeSet::new();
        let mut recovery = Vec::new();
        for p in joins {
            let j = self.auth(p)?;
            let Body::Join {
                participant,
                generation: g,
                activation,
                floor,
                cut,
                retained,
                ..
            } = j.body
            else {
                return Err(reject("join"));
            };
            if floor != self.image.gvt
                || cut != self.image.cut
                || participant != 0
                || g != generation
                || activation != packet_id(proposal)
                || j.generation != self.image.generation.max(1)
                || !issuers.insert(j.issuer)
            {
                return Err(reject("join binding"));
            }
            if j.issuer == self.participant
                && !self
                    .image
                    .facts
                    .iter()
                    .any(|f| f.packet == proposal && f.responses.contains(p))
            {
                return Err(reject("own join"));
            }
            recovery.push((j.issuer, retained));
        }
        if issuers != self.config.members.iter().map(|m| m.participant).collect()
            || generation != self.image.generation + 1
        {
            return Err(reject("membership"));
        }
        let mut next = self.image.clone();
        next.generation = generation;
        next.active = true;
        next.paused = None;
        next.joining = None;
        next.recovery = recovery;
        next.activation = Some(proposal.to_vec());
        next.proposal = None;
        next.joins = joins.to_vec();
        self.commit(next, vec![proposal.to_vec()])
    }
    pub fn schedule_initial(&mut self, sequence: u64, event: RemoteEvent) -> Result<()> {
        self.runnable()?;
        if !self.image.initial_open || self.owner(event.source_lp.0)? != self.participant {
            return Err(reject("initial scheduling closed or source not owned"));
        }
        let mut next = self.image.clone();
        let lp = event.source_lp.0;
        if next.roots.contains(&(lp, sequence)) {
            return Err(reject("root reserved"));
        }
        next.roots.push((lp, sequence));
        let incarnation = allocate(&mut next, lp)?;
        let m = Message {
            source: lp,
            destination: event.dest_lp.0,
            tick: event.tick.ticks(),
            root_source: lp,
            root_sequence: sequence,
            ancestry: vec![],
            incarnation,
            anti: false,
            payload: event.event_payload,
        };
        self.emit(&mut next, m, vec![])?;
        self.commit(next, vec![])
    }
    fn emit(&self, image: &mut Image, message: Message, dependencies: Vec<Hash>) -> Result<()> {
        self.validate_message(&message)?;
        if message.tick < image.gvt {
            return Err(reject("pre-GVT output"));
        }
        let packet = self.sign(
            image,
            Body::Delivery {
                message: message.clone(),
            },
        )?;
        image.outgoing.push(Outgoing {
            message,
            packet,
            dependencies,
            admitted: false,
            retired: false,
            suppressed: false,
        });
        Ok(())
    }
    pub fn pending_packets(&self) -> Result<Vec<Vec<u8>>> {
        self.runnable()?;
        Ok(self
            .image
            .outgoing
            .iter()
            .filter(|o| {
                !o.suppressed
                    && (!o.admitted || (o.message.anti && !o.retired))
                    && o.dependencies.iter().all(|d| finished(&self.image, d))
            })
            .map(|o| o.packet.clone())
            .collect())
    }
    pub fn receive_from(&mut self, certificate: Hash, packet: &[u8]) -> Result<Vec<Vec<u8>>> {
        let r = self.auth(packet)?;
        if !self
            .config
            .members
            .iter()
            .any(|m| m.participant == r.issuer && m.certificate_hash == certificate)
        {
            return Err(reject("TLS/signing member mismatch"));
        }
        // The fresh complete membership explicitly authorizes these exact
        // historical packets; this is recovery ingress, not a stale live bypass.
        if matches!(r.body, Body::Activation { .. }) {
            return self.join_activation(packet).map(|p| vec![p]);
        }
        if matches!(r.body, Body::Delivery { .. })
            && r.generation < self.image.generation
            && self
                .image
                .recovery
                .iter()
                .any(|(issuer, ids)| *issuer == r.issuer && ids.contains(&packet_id(packet)))
        {
            return self.recover_delivery(packet);
        }
        self.receive_packet(packet)
    }
    pub fn receive_packet(&mut self, packet: &[u8]) -> Result<Vec<Vec<u8>>> {
        self.receive_inner(packet, false)
    }
    pub fn recover_delivery(&mut self, packet: &[u8]) -> Result<Vec<Vec<u8>>> {
        self.receive_inner(packet, true)
    }
    fn receive_inner(&mut self, packet: &[u8], recovery: bool) -> Result<Vec<Vec<u8>>> {
        self.healthy()?;
        let record = self.auth(packet)?;
        if let Some(f) = self.image.facts.iter().find(|f| f.packet == packet) {
            let mut replies = f.responses.clone();
            if let Some(r) = self
                .image
                .reversals
                .iter()
                .find(|r| r.packet == packet)
                .and_then(|r| r.receipt.clone())
            {
                if !replies.contains(&r) {
                    replies.push(r);
                }
            }
            return Ok(replies);
        }
        if !self.image.active {
            return Err(ExternalError::Standby);
        }
        if (self.image.prepared.is_some() || self.image.paused.is_some())
            && !matches!(record.body, Body::Cut { .. })
            || self.image.joining.is_some()
        {
            return Err(ExternalError::Paused);
        }
        let retained_receipt = match &record.body {
            Body::Admission { delivery, .. } | Body::Retirement { delivery, .. } => {
                self.image.outgoing.iter().any(|o| {
                    packet_id(&o.packet) == *delivery
                        && self.owner(o.message.destination).ok() == Some(record.issuer)
                        && self
                            .auth(&o.packet)
                            .is_ok_and(|r| r.generation == record.generation)
                })
            }
            _ => false,
        };
        if record.generation != self.image.generation
            && !retained_receipt
            && !(recovery
                && matches!(record.body, Body::Delivery { .. })
                && record.generation < self.image.generation
                && self
                    .image
                    .recovery
                    .iter()
                    .any(|(p, h)| *p == record.issuer && h.contains(&packet_id(packet))))
        {
            return Err(reject("stale live session"));
        }
        let mut next = self.image.clone();
        let mut replies = Vec::new();
        match record.body {
            Body::Delivery { message } => {
                self.validate_message(&message)?;
                if self.owner(message.source)? != record.issuer
                    || self.owner(message.destination)? != self.participant
                    || message.tick < next.gvt
                {
                    return Err(reject("delivery ownership/floor"));
                }
                let key = identity(&message);
                if let Some(known) = next.known.iter().find(|m| identity(m) == key) {
                    if known != &positive(&message) {
                        return Err(reject("conflicting incarnation"));
                    }
                } else {
                    next.known.push(positive(&message));
                }
                let p = next
                    .processes
                    .iter_mut()
                    .find(|p| p.lp == message.destination)
                    .unwrap();
                if !p
                    .pending
                    .iter()
                    .any(|m| identity(m) == key && m.anti == message.anti)
                    && !p.tombstones.iter().any(|m| identity(m) == key)
                    && !p.history.iter().any(|e| identity(&e.message) == key)
                    && !message.anti
                {
                    p.pending.push(message.clone());
                }
                if message.anti
                    && !p.pending.iter().any(|m| identity(m) == key && m.anti)
                    && !p.tombstones.iter().any(|m| identity(m) == key)
                {
                    p.pending.push(message.clone());
                }
                p.epoch = p
                    .epoch
                    .checked_add(1)
                    .ok_or_else(|| reject("epoch exhausted"))?;
                let effect = if next
                    .processes
                    .iter()
                    .find(|p| p.lp == message.destination)
                    .unwrap()
                    .tombstones
                    .iter()
                    .any(|m| identity(m) == key)
                {
                    FactEffect::ProtectedTombstone
                } else if message.anti {
                    FactEffect::QueuedAnti
                } else if next
                    .processes
                    .iter()
                    .find(|p| p.lp == message.destination)
                    .unwrap()
                    .history
                    .iter()
                    .any(|e| identity(&e.message) == key)
                {
                    FactEffect::AlreadyExecuted
                } else {
                    FactEffect::QueuedPositive
                };
                let ack = self.sign(
                    &mut next,
                    Body::Admission {
                        binding: DurableBinding::pending(effect),
                        delivery: packet_id(packet),
                    },
                )?;
                replies.push(ack);
                if message.anti {
                    next.reversals.push(Reversal {
                        delivery: packet_id(packet),
                        packet: packet.to_vec(),
                        dependencies: vec![],
                        applied: false,
                        receipt: None,
                        effect: FactEffect::RetiredTombstone,
                    });
                }
            }
            Body::Admission { delivery, .. } => {
                let o = next
                    .outgoing
                    .iter_mut()
                    .find(|o| packet_id(&o.packet) == delivery)
                    .ok_or_else(|| reject("unknown admission"))?;
                if self.owner(o.message.destination)? != record.issuer {
                    return Err(reject("admission receiver"));
                }
                o.admitted = true;
            }
            Body::Retirement { delivery, .. } => {
                let o = next
                    .outgoing
                    .iter_mut()
                    .find(|o| packet_id(&o.packet) == delivery && o.message.anti)
                    .ok_or_else(|| reject("unknown retirement"))?;
                if self.owner(o.message.destination)? != record.issuer {
                    return Err(reject("retirement receiver"));
                }
                o.admitted = true;
                o.retired = true;
            }
            Body::Pause { cut } => {
                if record.issuer != 0
                    || next.paused.is_some()
                    || cut
                        != next
                            .cut
                            .checked_add(1)
                            .ok_or_else(|| reject("cut exhausted"))?
                {
                    return Err(reject("pause"));
                }
                next.paused = Some(cut);
                next.pause_packet = Some(packet.to_vec());
            }
            Body::Cut {
                cut,
                floor,
                prepares,
            } => {
                self.apply_cut_to(&mut next, record.issuer, cut, floor, &prepares)?;
                next.cut_decision = Some(packet.to_vec());
            }
            _ => return Err(reject("unsupported live record")),
        }
        self.complete_retirements(&mut next)?;
        next.facts.push(Fact {
            packet: packet.to_vec(),
            responses: replies.clone(),
        });
        self.commit(next, replies.clone())?;
        Ok(self
            .image
            .facts
            .iter()
            .find(|f| f.packet == packet)
            .ok_or_else(|| reject("committed fact missing"))?
            .responses
            .clone())
    }
    fn complete_retirements(&self, image: &mut Image) -> Result<()> {
        let ready = image
            .reversals
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.applied
                    && r.receipt.is_none()
                    && r.dependencies.iter().all(|d| finished(image, d))
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        for index in ready {
            let r = &image.reversals[index];
            let p = self.sign(
                image,
                Body::Retirement {
                    binding: DurableBinding::pending(r.effect),
                    delivery: r.delivery,
                    dependencies: r.dependencies.clone(),
                },
            )?;
            image.reversals[index].receipt = Some(p);
        }
        image.barriers = image
            .barriers
            .iter()
            .filter(|d| !finished(image, d))
            .copied()
            .collect();
        Ok(())
    }
    /// Executes against independently decoded staged model instances. Only the
    /// committed complete after-image becomes observable through this driver.
    pub fn run_until_with_budget(&mut self, horizon: u128, budget: usize) -> Result<usize> {
        self.runnable()?;
        if budget > MAX_ITEMS || self.image.last_horizon.is_some_and(|h| horizon < h) {
            return Err(reject("budget/horizon"));
        }
        let staged = catch_unwind(AssertUnwindSafe(|| self.stage_steps(horizon, budget)));
        let (image, used) = match staged {
            Ok(r) => r?,
            Err(_) => {
                self.fenced = true;
                return Err(ExternalError::Model(
                    "model/codec panicked; owner fenced".into(),
                ));
            }
        };
        if used > 0 {
            self.commit(image, vec![])?;
        }
        Ok(used)
    }
    fn stage_steps(&self, horizon: u128, budget: usize) -> Result<(Image, usize)> {
        let mut image = self.image.clone();
        image.last_horizon = Some(horizon);
        let mut used = 0;
        while used < budget {
            let selected = image
                .processes
                .iter()
                .enumerate()
                .flat_map(|(p, s)| {
                    s.pending
                        .iter()
                        .enumerate()
                        .filter(move |(_, m)| m.tick <= horizon)
                        .map(move |(q, m)| (p, q, m))
                })
                .min_by_key(|(_, _, m)| (!m.anti, order(m)))
                .map(|(p, q, m)| (p, q, m.clone()));
            let Some((p, q, m)) = selected else { break };
            image.initial_open = false;
            if m.anti {
                image.processes[p].pending.remove(q);
                let key = identity(&m);
                let mut dependencies = vec![];
                let removed_history = image.processes[p]
                    .history
                    .iter()
                    .any(|e| identity(&e.message) == key);
                if let Some(index) = image.processes[p]
                    .history
                    .iter()
                    .position(|e| identity(&e.message) == key)
                {
                    dependencies = self.rollback(&mut image, p, index, Some(key))?;
                }
                image.processes[p].pending.retain(|e| identity(e) != key);
                image.processes[p].replay.retain(|id| *id != key);
                if !image.processes[p]
                    .tombstones
                    .iter()
                    .any(|e| identity(e) == key)
                {
                    image.processes[p].tombstones.push(positive(&m));
                }
                for r in &mut image.reversals {
                    if matches!(self.auth(&r.packet)?.body,Body::Delivery{ref message} if identity(message)==key)
                    {
                        r.applied = true;
                        r.effect = if !removed_history {
                            FactEffect::RetiredTombstone
                        } else {
                            FactEffect::RetiredRollback
                        };
                        r.dependencies = dependencies.clone();
                    }
                }
                used += 1;
            } else {
                if let Some(index) = image.processes[p]
                    .history
                    .iter()
                    .position(|e| order(&e.message) > order(&m))
                {
                    self.rollback(&mut image, p, index, None)?;
                    used += 1;
                    continue;
                }
                image.processes[p].pending.remove(q);
                if image.processes[p]
                    .tombstones
                    .iter()
                    .any(|e| identity(e) == identity(&m))
                {
                    used += 1;
                    continue;
                }
                let mut process = self
                    .codec
                    .decode_process(&image.processes[p].current)
                    .map_err(ExternalError::Model)?;
                let before = self
                    .codec
                    .encode_snapshot(&process.snapshot())
                    .map_err(ExternalError::Model)?;
                let event = RemoteEvent {
                    source_lp: LpId(m.source),
                    dest_lp: LpId(m.destination),
                    tick: Tick::from_ticks(m.tick),
                    event_payload: m.payload.clone(),
                };
                let outputs = process.on_event(&event);
                if outputs.len() > MAX_ITEMS {
                    return Err(reject("output batch"));
                }
                let current = self
                    .codec
                    .encode_process(&process)
                    .map_err(ExternalError::Model)?;
                if current.len() > 65536 || before.len() > 65536 {
                    return Err(reject("model bound"));
                }
                let mut messages = Vec::new();
                for (ordinal, e) in outputs.into_iter().enumerate() {
                    if e.source_lp.0 != m.destination || e.tick.ticks() <= m.tick {
                        return Err(reject("handler source/time"));
                    }
                    let mut ancestry = m.ancestry.clone();
                    ancestry.push(Frame {
                        tick: m.tick,
                        source: m.source,
                        ordinal: ordinal as u32,
                    });
                    let incarnation = allocate(&mut image, m.destination)?;
                    let output = Message {
                        source: m.destination,
                        destination: e.dest_lp.0,
                        tick: e.tick.ticks(),
                        root_source: m.root_source,
                        root_sequence: m.root_sequence,
                        ancestry,
                        incarnation,
                        anti: false,
                        payload: e.event_payload,
                    };
                    let dependencies = image.barriers.clone();
                    self.emit(&mut image, output.clone(), dependencies)?;
                    messages.push(output);
                }
                if image.processes[p].replay.contains(&identity(&m)) {
                    image.replays = image
                        .replays
                        .checked_add(1)
                        .ok_or_else(|| reject("replay counter"))?;
                    image.processes[p].replay.retain(|id| *id != identity(&m));
                }
                image.processes[p].current = current;
                image.processes[p].history.push(Execution {
                    message: m,
                    before,
                    outputs: messages,
                });
                image.executions = image
                    .executions
                    .checked_add(1)
                    .ok_or_else(|| reject("execution counter"))?;
                used += 1;
            }
            image.processes[p].epoch = image.processes[p]
                .epoch
                .checked_add(1)
                .ok_or_else(|| reject("epoch exhausted"))?;
            self.complete_retirements(&mut image)?;
        }
        Ok((image, used))
    }
    fn rollback(
        &self,
        image: &mut Image,
        p: usize,
        index: usize,
        canceled: Option<Hash>,
    ) -> Result<Vec<Hash>> {
        let snapshot = self
            .codec
            .decode_snapshot(&image.processes[p].history[index].before)
            .map_err(ExternalError::Model)?;
        let mut model = self
            .codec
            .decode_process(&image.processes[p].current)
            .map_err(ExternalError::Model)?;
        model
            .restore(&snapshot)
            .map_err(|e| ExternalError::Model(e.to_string()))?;
        image.processes[p].current = self
            .codec
            .encode_process(&model)
            .map_err(ExternalError::Model)?;
        let suffix = image.processes[p].history.split_off(index);
        let mut dependencies = Vec::new();
        for e in suffix {
            for output in e.outputs {
                let mut anti = output.clone();
                anti.anti = true;
                // Suppress the old positive without pretending that suppression was an ACK.
                if let Some(o) = image.outgoing.iter_mut().find(|o| o.message == output) {
                    o.suppressed = true;
                }
                self.emit(image, anti, vec![])?;
                let id = packet_id(&image.outgoing.last().unwrap().packet);
                dependencies.push(id);
                image.barriers.push(id);
            }
            let id = identity(&e.message);
            if canceled != Some(id)
                && !image.processes[p]
                    .pending
                    .iter()
                    .any(|m| identity(m) == id && !m.anti)
            {
                image.processes[p].pending.push(e.message);
                image.processes[p].replay.push(id);
            }
        }
        image.rollbacks = image
            .rollbacks
            .checked_add(1)
            .ok_or_else(|| reject("rollback counter"))?;
        Ok(dependencies)
    }
    fn frontiers(&self, image: &Image) -> Result<Vec<(u32, u64)>> {
        let mut result = Vec::new();
        for member in &self.config.members {
            let mut next = 0;
            for f in &image.facts {
                let r = self.auth(&f.packet)?;
                if r.issuer == member.participant {
                    next = next.max(
                        r.sequence
                            .checked_add(1)
                            .ok_or_else(|| reject("channel sequence exhausted"))?,
                    );
                }
            }
            result.push((member.participant, next));
        }
        Ok(result)
    }
    pub fn minimum(&self) -> u128 {
        self.image
            .processes
            .iter()
            .flat_map(|p| p.pending.iter().map(|m| m.tick))
            .chain(
                self.image
                    .outgoing
                    .iter()
                    .filter(|o| {
                        (!o.admitted)
                            || (o.message.anti && !o.retired)
                            || (!o.dependencies.iter().all(|d| finished(&self.image, d)))
                    })
                    .map(|o| o.message.tick),
            )
            .chain(
                self.image
                    .reversals
                    .iter()
                    .filter(|r| r.receipt.is_none())
                    .filter_map(|r| self.auth(&r.packet).ok())
                    .filter_map(|r| {
                        if let Body::Delivery { message } = r.body {
                            Some(message.tick)
                        } else {
                            None
                        }
                    }),
            )
            .min()
            .unwrap_or(u128::MAX)
    }
    pub fn pause_cut(&mut self) -> Result<Vec<u8>> {
        self.runnable()?;
        if self.participant != 0 {
            return Err(reject("cut coordinator"));
        }
        let mut next = self.image.clone();
        let cut = next
            .cut
            .checked_add(1)
            .ok_or_else(|| reject("cut exhausted"))?;
        let p = self.sign(&mut next, Body::Pause { cut })?;
        next.paused = Some(cut);
        next.pause_packet = Some(p.clone());
        next.facts.push(Fact {
            packet: p.clone(),
            responses: vec![],
        });
        self.commit(next, vec![p.clone()])?;
        Ok(p)
    }
    pub fn prepare_cut(&mut self, finite_limit: u128) -> Result<Vec<u8>> {
        self.healthy()?;
        if let Some(p) = &self.image.prepared {
            return Ok(p.clone());
        }
        let cut = self.image.paused.ok_or_else(|| reject("not paused"))?;
        if self.image.initial_open || finite_limit == u128::MAX || finite_limit < self.image.gvt {
            return Err(reject("initial inputs open or invalid finite cut limit"));
        }
        if self.image.joining.is_some()
            || self.image.outgoing.iter().any(|o| {
                (!o.admitted)
                    || (o.message.anti && !o.retired)
                    || o.dependencies.iter().any(|d| !finished(&self.image, d))
            })
            || self.image.reversals.iter().any(|r| r.receipt.is_none())
        {
            return Err(reject("captured channels or reversal closure incomplete"));
        }
        let mut sent = self
            .image
            .outgoing
            .iter()
            .map(|o| Ok((self.owner(o.message.destination)?, packet_id(&o.packet))))
            .collect::<Result<Vec<_>>>()?;
        sent.sort();
        let mut received = self
            .image
            .facts
            .iter()
            .filter_map(|f| {
                self.auth(&f.packet).ok().and_then(|r| {
                    if matches!(r.body, Body::Delivery { .. }) {
                        Some((r.issuer, packet_id(&f.packet)))
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();
        received.sort();
        let mut next = self.image.clone();
        let p = self.sign(
            &mut next,
            Body::Prepare {
                cut,
                revision: self.revision()?,
                minimum: self.minimum().min(finite_limit),
                frontiers: self.frontiers(&self.image)?,
                floor: self.image.gvt,
                checkpoint: digest(&bytes(&self.image)?),
                pause: packet_id(
                    self.image
                        .pause_packet
                        .as_deref()
                        .ok_or_else(|| reject("missing pause"))?,
                ),
                sent,
                received,
            },
        )?;
        next.prepared = Some(p.clone());
        self.commit(next, vec![p.clone()])?;
        Ok(p)
    }
    fn prepare_set(&self, cut: u64, prepares: &[Vec<u8>]) -> Result<u128> {
        if prepares.len() != self.config.members.len() {
            return Err(reject("incomplete prepares"));
        }
        let mut seen = BTreeSet::new();
        let mut floor = u128::MAX;
        let mut sent_set = BTreeSet::new();
        let mut received_set = BTreeSet::new();
        let expected_pause = packet_id(
            self.image
                .pause_packet
                .as_deref()
                .ok_or_else(|| reject("pause missing"))?,
        );
        for p in prepares {
            let r = self.auth(p)?;
            let Body::Prepare {
                cut: c,
                minimum,
                floor: previous,
                frontiers,
                pause,
                sent,
                received,
                ..
            } = r.body
            else {
                return Err(reject("prepare record"));
            };
            if c != cut
                || r.generation != self.image.generation
                || previous != self.image.gvt
                || minimum < previous
                || !seen.insert(r.issuer)
                || frontiers.iter().map(|(p, _)| *p).collect::<BTreeSet<_>>()
                    != self.config.members.iter().map(|m| m.participant).collect()
                || frontiers.len() != self.config.members.len()
            {
                return Err(reject("prepare identity/floor/channels"));
            }
            if pause != expected_pause || minimum == u128::MAX {
                return Err(reject("pause or finite bound"));
            }
            for (dest, id) in sent {
                if !self.config.members.iter().any(|m| m.participant == dest)
                    || !sent_set.insert((r.issuer, dest, id))
                {
                    return Err(reject("sent channel duplicate"));
                }
            }
            for (source, id) in received {
                if !self.config.members.iter().any(|m| m.participant == source)
                    || !received_set.insert((source, r.issuer, id))
                {
                    return Err(reject("received channel duplicate"));
                }
            }
            floor = floor.min(minimum);
        }
        if seen != self.config.members.iter().map(|m| m.participant).collect() {
            return Err(reject("prepare membership"));
        }
        if sent_set != received_set {
            return Err(reject("captured channel set mismatch"));
        }
        Ok(floor)
    }
    pub fn commit_cut(&mut self, prepares: Vec<Vec<u8>>) -> Result<Vec<u8>> {
        self.healthy()?;
        if self.participant != 0 {
            return Err(reject("cut coordinator"));
        }
        let cut = self.image.paused.ok_or_else(|| reject("not paused"))?;
        let floor = self.prepare_set(cut, &prepares)?;
        let mut next = self.image.clone();
        let p = self.sign(
            &mut next,
            Body::Cut {
                cut,
                floor,
                prepares: prepares.clone(),
            },
        )?;
        self.apply_cut_to(&mut next, self.participant, cut, floor, &prepares)?;
        next.cut_decision = Some(p.clone());
        next.facts.push(Fact {
            packet: p.clone(),
            responses: vec![],
        });
        self.commit(next, vec![p.clone()])?;
        Ok(p)
    }
    fn apply_cut_to(
        &self,
        image: &mut Image,
        issuer: u32,
        cut: u64,
        floor: u128,
        prepares: &[Vec<u8>],
    ) -> Result<()> {
        if issuer != 0
            || image.paused != Some(cut)
            || self.prepare_set(cut, prepares)? != floor
            || !image
                .prepared
                .as_ref()
                .is_some_and(|p| prepares.contains(p))
        {
            return Err(reject("common cut decision"));
        }
        for process in &mut image.processes {
            process.history.retain(|e| e.message.tick >= floor);
            process.tombstones.retain(|m| m.tick >= floor);
            process.fossil = floor;
        }
        image.gvt = floor;
        image.cut = cut;
        image.paused = None;
        image.prepared = None;
        Ok(())
    }
    pub fn close_initial_inputs(&mut self) -> Result<()> {
        self.runnable()?;
        if !self.image.initial_open {
            return Ok(());
        }
        let mut next = self.image.clone();
        next.initial_open = false;
        self.commit(next, vec![])
    }
    pub fn retained_activation(&self) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
        self.image
            .activation
            .clone()
            .map(|p| (p, self.image.joins.clone()))
    }
    pub fn recover_cut(&mut self, packet: &[u8]) -> Result<()> {
        self.healthy()?;
        let r = self.auth(packet)?;
        if r.generation != self.image.generation {
            return Err(reject("cut generation"));
        }
        let Body::Cut {
            cut,
            floor,
            prepares,
        } = r.body
        else {
            return Err(reject("cut decision"));
        };
        if self.image.cut_decision.as_deref() == Some(packet) {
            return Ok(());
        }
        let mut next = self.image.clone();
        self.apply_cut_to(&mut next, r.issuer, cut, floor, &prepares)?;
        next.cut_decision = Some(packet.to_vec());
        next.active = false;
        self.commit(next, vec![packet.to_vec()])
    }
    pub fn retained_cut_decision(&self) -> Option<&[u8]> {
        self.image.cut_decision.as_deref()
    }
}
fn allocate(image: &mut Image, source: u32) -> Result<u64> {
    let (_, next) = image
        .next_incarnation
        .iter_mut()
        .find(|(lp, _)| *lp == source)
        .ok_or_else(|| reject("unowned incarnation allocator"))?;
    let value = next.ok_or_else(|| reject("incarnation exhausted"))?;
    *next = value.checked_add(1);
    Ok(value)
}
fn logical_key(m: &Message) -> super::OptimisticEventOrderKey {
    let mut logical = super::LogicalEventId::root(LpId(m.root_source), m.root_sequence);
    for frame in &m.ancestry {
        let parent = super::OptimisticEventOrderKey::try_from_parts(
            Tick::from_ticks(frame.tick),
            LpId(frame.source),
            logical,
        )
        .expect("validated complete parent ancestry");
        logical = super::LogicalEventId::child(&parent, frame.ordinal)
            .expect("bounded complete ancestry");
    }
    super::OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(m.tick),
        LpId(m.source),
        logical,
    )
    .expect("validated complete message ancestry")
}
fn order(m: &Message) -> (super::OptimisticEventOrderKey, u64) {
    (logical_key(m), m.incarnation)
}

#[cfg(test)]
mod tests;
