// SPDX-License-Identifier: Apache-2.0
//! Bounded canonical application records. Verified signatures authenticate issuer
//! assertions; they do not construct engine admission, retirement or cut authority.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

pub type Hash = [u8; 32];
pub const MAX_FRAME: usize = 128 * 1024;
const MAX_PAYLOAD: usize = 16 * 1024;
const MAX_ANCESTRY: usize = 32;
const MAX_MEMBERS: usize = 64;
const SIGN_DOMAIN: &[u8] = b"KAIROS-EXTERNAL-ED25519-V1\0";

/// Hashes exact bytes with a domain and an explicit length.
pub fn digest(bytes: &[u8]) -> Hash {
    let mut hash = Sha256::new();
    hash.update(b"KAIROS-EXTERNAL-DIGEST-V1\0");
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
    hash.finalize().into()
}

/// Full-width ticks have one language-neutral representation, including inside
/// serde tagged enums (whose Content decoder does not support numeric u128).
pub(crate) mod tick {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{value:032x}"))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value.len() != 32
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(serde::de::Error::custom(
                "tick requires32 lowercase hex digits",
            ));
        }
        u128::from_str_radix(&value, 16).map_err(serde::de::Error::custom)
    }
}
struct BoundedWriter(Vec<u8>);
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_FRAME.saturating_sub(self.0.len()) {
            return Err(io::Error::other("external frame exceeds byte limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Canonical for this version's fixed serde struct/enum schema. Collection order
/// is part of the signed assertion; engine membership/index checks remain separate.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut output = BoundedWriter(Vec::new());
    serde_json::to_writer(&mut output, value).map_err(|error| error.to_string())?;
    Ok(output.0)
}

/// Bounds bytes before parsing, then rejects alternate encodings, duplicate or
/// ignored fields, whitespace and trailing input through exact re-encoding.
pub fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, String> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err("external frame byte limit".into());
    }
    let value: T = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if encode(&value)? != bytes {
        return Err("noncanonical external frame".into());
    }
    Ok(value)
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    #[serde(with = "tick")]
    pub tick: u128,
    pub source: u32,
    pub ordinal: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub source: u32,
    pub destination: u32,
    #[serde(with = "tick")]
    pub tick: u128,
    pub root_source: u32,
    pub root_sequence: u64,
    pub ancestry: Vec<Frame>,
    pub incarnation: u64,
    pub anti: bool,
    pub payload: Vec<u8>,
}
impl Message {
    /// Exact positive/anti pair identity. Changed tick, payload or destination is
    /// another identity; logical conflict detection must additionally use logical().
    pub fn id(&self) -> Hash {
        let mut hash = Sha256::new();
        hash.update(b"KAIROS-EXTERNAL-DELIVERY-V1\0");
        hash.update(self.logical());
        hash.update(self.source.to_be_bytes());
        hash.update(self.destination.to_be_bytes());
        hash.update(self.tick.to_be_bytes());
        hash.update(self.incarnation.to_be_bytes());
        hash.update((self.payload.len() as u64).to_be_bytes());
        hash.update(&self.payload);
        hash.finalize().into()
    }
    /// Complete structural logical lineage, without mutable delivery attributes.
    pub fn logical(&self) -> Hash {
        let mut hash = Sha256::new();
        hash.update(b"KAIROS-EXTERNAL-LOGICAL-V1\0");
        hash.update(self.root_source.to_be_bytes());
        hash.update(self.root_sequence.to_be_bytes());
        hash.update((self.ancestry.len() as u64).to_be_bytes());
        for frame in &self.ancestry {
            hash.update(frame.tick.to_be_bytes());
            hash.update(frame.source.to_be_bytes());
            hash.update(frame.ordinal.to_be_bytes());
        }
        hash.finalize().into()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.ancestry.len() > MAX_ANCESTRY || self.payload.len() > MAX_PAYLOAD {
            return Err("external message bound".into());
        }
        let source = self
            .ancestry
            .last()
            .map_or(self.root_source, |frame| frame.source);
        if source != self.source
            || self
                .ancestry
                .windows(2)
                .any(|pair| pair[0].tick >= pair[1].tick)
            || self
                .ancestry
                .last()
                .is_some_and(|frame| frame.tick >= self.tick)
        {
            return Err("invalid external ancestry".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Body {
    Delivery {
        message: Message,
    },
    Admission {
        delivery: Hash,
    },
    Retirement {
        delivery: Hash,
        dependencies: Vec<Hash>,
    },
    Activation {
        generation: u64,
        nonce: Hash,
    },
    Join {
        participant: u32,
        generation: u64,
        activation: Hash,
        #[serde(with = "tick")]
        floor: u128,
        cut: u64,
        checkpoint: Hash,
        frontiers: Vec<(u32, u64)>,
        retained: Vec<Hash>,
    },
    Pause {
        cut: u64,
    },
    Prepare {
        cut: u64,
        pause: Hash,
        revision: u64,
        checkpoint: Hash,
        #[serde(with = "tick")]
        minimum: u128,
        frontiers: Vec<(u32, u64)>,
        sent: Vec<(u32, Hash)>,
        received: Vec<(u32, Hash)>,
        #[serde(with = "tick")]
        floor: u128,
    },
    Cut {
        cut: u64,
        #[serde(with = "tick")]
        floor: u128,
        prepares: Vec<Vec<u8>>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub version: u32,
    pub context: Hash,
    pub issuer: u32,
    pub generation: u64,
    pub sequence: u64,
    pub body: Body,
}
impl Record {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.generation == 0 {
            return Err("external record version/generation".into());
        }
        match &self.body {
            Body::Delivery { message } => message.validate()?,
            Body::Retirement { dependencies, .. } => {
                if dependencies.len() > MAX_MEMBERS
                    || dependencies.iter().collect::<BTreeSet<_>>().len() != dependencies.len()
                {
                    return Err("external retirement dependency bound/duplicate".into());
                }
            }
            Body::Activation { generation, .. } => {
                if *generation == 0 {
                    return Err("external activation generation".into());
                }
            }
            Body::Join {
                generation,
                frontiers,
                retained,
                ..
            } => {
                if *generation == 0
                    || retained.len() > MAX_MEMBERS
                    || retained.iter().collect::<BTreeSet<_>>().len() != retained.len()
                    || !valid_frontiers(frontiers)
                {
                    return Err("external join bound/duplicate/generation".into());
                }
            }
            Body::Prepare {
                frontiers,
                sent,
                received,
                ..
            } => {
                if !valid_frontiers(frontiers)
                    || !valid_deliveries(sent)
                    || !valid_deliveries(received)
                {
                    return Err("external prepare frontier/delivery bound/order/duplicate".into());
                }
            }
            Body::Cut { prepares, .. } => {
                if prepares.is_empty()
                    || prepares.len() > MAX_MEMBERS
                    || prepares
                        .iter()
                        .any(|prepare| prepare.is_empty() || prepare.len() > MAX_FRAME)
                {
                    return Err("external prepare bound".into());
                }
            }
            Body::Admission { .. } | Body::Pause { .. } => {}
        }
        Ok(())
    }
}
// Exact channel reconciliation uses canonical, unique (peer, packet digest)
// sets. Scalar frontiers are retained as diagnostics, never cut authority.
fn valid_deliveries(deliveries: &[(u32, Hash)]) -> bool {
    deliveries.len() <= MAX_MEMBERS && deliveries.windows(2).all(|pair| pair[0] < pair[1])
}
fn valid_frontiers(frontiers: &[(u32, u64)]) -> bool {
    frontiers.len() <= MAX_MEMBERS
        && frontiers
            .iter()
            .map(|(member, _)| member)
            .collect::<BTreeSet<_>>()
            .len()
            == frontiers.len()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    record: Record,
    signature: Vec<u8>,
}
fn signing_bytes(record: &Record) -> Result<Vec<u8>, String> {
    let encoded = encode(record)?;
    let mut bytes = Vec::with_capacity(SIGN_DOMAIN.len() + 8 + encoded.len());
    bytes.extend_from_slice(SIGN_DOMAIN);
    bytes.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&encoded);
    Ok(bytes)
}

/// Signs a bounded assertion. This never constructs a runtime proof capability.
pub fn sign(record: &Record, key: &SigningKey) -> Result<Vec<u8>, String> {
    record.validate()?;
    let signature = key.sign(&signing_bytes(record)?).to_bytes().to_vec();
    encode(&Signed {
        record: record.clone(),
        signature,
    })
}

/// Fixed strict verifier. Session, fencing, fact/replay and ownership checks are
/// the engine driver's responsibility after issuer authentication succeeds.
pub fn verify(
    bytes: &[u8],
    context: Hash,
    keys: &BTreeMap<u32, Vec<u8>>,
) -> Result<Record, String> {
    if keys.is_empty() || keys.len() > MAX_MEMBERS {
        return Err("external participant key-set bound".into());
    }
    let signed: Signed = decode(bytes)?;
    signed.record.validate()?;
    if signed.record.context != context {
        return Err("external configuration mismatch".into());
    }
    let public = keys
        .get(&signed.record.issuer)
        .ok_or("unknown external issuer")?;
    let bytes: [u8; 32] = public
        .as_slice()
        .try_into()
        .map_err(|_| "external key length")?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| "invalid external key")?;
    // from_bytes accepts ZIP-215 encodings. This protocol requires canonical keys.
    if key.is_weak() || key.to_edwards().compress().to_bytes() != bytes {
        return Err("weak/noncanonical external key".into());
    }
    let signature =
        Signature::from_slice(&signed.signature).map_err(|_| "external signature length")?;
    key.verify_strict(&signing_bytes(&signed.record)?, &signature)
        .map_err(|_| "invalid external signature")?;
    Ok(signed.record)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> SigningKey {
        SigningKey::from_bytes(&[48; 32])
    }
    fn keys() -> BTreeMap<u32, Vec<u8>> {
        BTreeMap::from([(7, key().verifying_key().to_bytes().to_vec())])
    }
    fn message() -> Message {
        Message {
            source: 7,
            destination: 9,
            tick: (1u128 << 100) + 25,
            root_source: 7,
            root_sequence: 12,
            ancestry: vec![],
            incarnation: 4,
            anti: false,
            payload: vec![3, 1, 4],
        }
    }
    fn record() -> Record {
        Record {
            version: 1,
            context: [48; 32],
            issuer: 7,
            generation: 2,
            sequence: 0,
            body: Body::Delivery { message: message() },
        }
    }
    #[test]
    fn canonical_signed_record_and_full_width_tick_roundtrip() {
        let frame = sign(&record(), &key()).unwrap();
        assert_eq!(verify(&frame, [48; 32], &keys()).unwrap(), record());
        assert_eq!(sign(&record(), &key()).unwrap(), frame);
        if std::env::var_os("KAIRO_PROTOCOL_GOLDEN").is_some() {
            println!(
                "CAP48_GOLDEN_FRAME={}",
                String::from_utf8(frame.clone()).unwrap()
            );
        }
        let hash: String = digest(&frame).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hash,
            "b859196410fea4a5bc2146b667cedf7b43b3d3c16c331569ea92b0dc7a401e9c"
        );
    }
    #[test]
    fn logical_and_pair_identity_bind_the_intended_exact_fields() {
        let m = message();
        let mut anti = m.clone();
        anti.anti = true;
        assert_eq!(m.id(), anti.id());
        for change in 0..4 {
            let mut changed = m.clone();
            match change {
                0 => changed.tick += 1,
                1 => changed.destination += 1,
                2 => changed.payload.push(2),
                _ => changed.incarnation += 1,
            }
            assert_ne!(m.id(), changed.id());
            assert_eq!(m.logical(), changed.logical());
        }
        let mut child = m.clone();
        child.ancestry.push(Frame {
            tick: m.tick,
            source: 7,
            ordinal: 0,
        });
        child.tick += 1;
        child.validate().unwrap();
        assert_ne!(m.logical(), child.logical());
    }
    #[test]
    fn decoder_rejects_alternate_unknown_duplicate_and_oversize_frames() {
        let encoded = encode(&record()).unwrap();
        let mut spaced = encoded.clone();
        spaced.push(b' ');
        assert!(decode::<Record>(&spaced).is_err());
        let original = String::from_utf8(encoded).unwrap();
        for invalid in [
            original.replacen('{', "{\"unknown\":1,", 1),
            original.replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        ] {
            assert!(decode::<Record>(invalid.as_bytes()).is_err());
        }
        assert!(decode::<Record>(&vec![b' '; MAX_FRAME + 1]).is_err());
        assert!(encode(&vec![0u8; MAX_FRAME]).is_err());
        assert!(decode::<Record>(b"").is_err());
    }
    #[test]
    fn signatures_reject_context_unknown_issuer_and_tampered_fields() {
        let frame = sign(&record(), &key()).unwrap();
        assert!(verify(&frame, [49; 32], &keys()).is_err());
        assert!(verify(
            &frame,
            [48; 32],
            &BTreeMap::from([(8, key().verifying_key().to_bytes().to_vec())])
        )
        .is_err());
        let mut signed: Signed = decode(&frame).unwrap();
        signed.record.sequence += 1;
        assert!(verify(&encode(&signed).unwrap(), [48; 32], &keys()).is_err());
        signed.record = record();
        signed.signature[0] ^= 1;
        assert!(verify(&encode(&signed).unwrap(), [48; 32], &keys()).is_err());
        signed.signature.pop();
        assert!(verify(&encode(&signed).unwrap(), [48; 32], &keys()).is_err());
    }
    #[test]
    fn weak_noncanonical_keys_and_noncanonical_signature_scalars_are_rejected() {
        let frame = sign(&record(), &key()).unwrap();
        // Compressed identity, and the alternate y=p+1 encoding of identity.
        let mut weak = [0u8; 32];
        weak[0] = 1;
        let mut alternate = [255u8; 32];
        alternate[0] = 238;
        alternate[31] = 127;
        for public in [weak, alternate, [255u8; 32]] {
            assert!(verify(&frame, [48; 32], &BTreeMap::from([(7, public.to_vec())])).is_err());
        }
        // A valid ZIP-215 alternate encoding of a non-weak point must reject
        // before signature verification, rather than relying only on is_weak().
        let noncanonical = (0..=18)
            .find_map(|offset| {
                let mut bytes = [255u8; 32];
                bytes[0] = 237 + offset;
                bytes[31] = 127;
                VerifyingKey::from_bytes(&bytes)
                    .ok()
                    .filter(|key| !key.is_weak() && key.to_edwards().compress().to_bytes() != bytes)
                    .map(|_| bytes)
            })
            .expect("fixture includes a noncanonical non-weak point");
        assert_eq!(
            verify(
                &frame,
                [48; 32],
                &BTreeMap::from([(7, noncanonical.to_vec())])
            )
            .unwrap_err(),
            "weak/noncanonical external key"
        );
        let mut signed: Signed = decode(&frame).unwrap();
        signed.signature = vec![0; 64];
        signed.signature[0] = 1;
        assert!(verify(
            &encode(&signed).unwrap(),
            [48; 32],
            &BTreeMap::from([(7, weak.to_vec())])
        )
        .is_err());
        signed.signature[32..].fill(255);
        assert!(verify(&encode(&signed).unwrap(), [48; 32], &keys()).is_err());
    }
    #[test]
    fn bounds_version_generation_and_invalid_ancestry_fail_before_signing() {
        for mode in 0..5 {
            let mut r = record();
            match mode {
                0 => r.version = 2,
                1 => r.generation = 0,
                2 => {
                    if let Body::Delivery { message } = &mut r.body {
                        message.payload = vec![0; MAX_PAYLOAD + 1]
                    }
                }
                3 => {
                    if let Body::Delivery { message } = &mut r.body {
                        message.source = 8
                    }
                }
                _ => {
                    if let Body::Delivery { message } = &mut r.body {
                        message.ancestry = vec![
                            Frame {
                                tick: 1,
                                source: 7,
                                ordinal: 0
                            };
                            MAX_ANCESTRY + 1
                        ]
                    }
                }
            }
            assert!(sign(&r, &key()).is_err());
        }
        let mut m = message();
        m.ancestry = vec![Frame {
            tick: m.tick,
            source: 7,
            ordinal: 0,
        }];
        assert!(m.validate().is_err());
        m.ancestry = vec![
            Frame {
                tick: 2,
                source: 7,
                ordinal: 0,
            },
            Frame {
                tick: 1,
                source: 7,
                ordinal: 1,
            },
        ];
        assert!(m.validate().is_err());
    }
    #[test]
    fn collection_bounds_and_duplicate_frontiers_or_dependencies_reject() {
        let mut r = record();
        for body in [
            Body::Retirement {
                delivery: [1; 32],
                dependencies: vec![[2; 32]; 2],
            },
            Body::Prepare {
                cut: 1,
                pause: [0; 32],
                revision: 0,
                checkpoint: [0; 32],
                minimum: 1,
                frontiers: vec![(1, 1), (1, 2)],
                sent: vec![],
                received: vec![],
                floor: 0,
            },
            Body::Cut {
                cut: 1,
                floor: 1,
                prepares: vec![],
            },
            Body::Join {
                participant: 1,
                generation: 0,
                activation: [0; 32],
                floor: 0,
                cut: 0,
                checkpoint: [0; 32],
                frontiers: vec![],
                retained: vec![],
            },
        ] {
            r.body = body;
            assert!(sign(&r, &key()).is_err());
        }
    }
    fn prepare() -> Record {
        let mut r = record();
        r.body = Body::Prepare {
            cut: 3,
            pause: [1; 32],
            revision: 4,
            checkpoint: [2; 32],
            minimum: (1u128 << 100) + 25,
            frontiers: vec![(9, 4)],
            sent: vec![(9, [3; 32])],
            received: vec![(9, [4; 32])],
            floor: 1,
        };
        r
    }
    #[test]
    fn prepare_binds_pause_checkpoint_and_exact_delivery_sets() {
        let r = prepare();
        let frame = sign(&r, &key()).unwrap();
        assert_eq!(verify(&frame, [48; 32], &keys()).unwrap(), r);
        let hash: String = digest(&frame).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hash,
            "f39dd7f18d02f1890fce5ed179509134de6c71bcbe93828f7e83d927025fee1b"
        );
        for field in ["pause", "sent", "received"] {
            let mut value = serde_json::to_value(&r).unwrap();
            value["body"].as_object_mut().unwrap().remove(field);
            let error = serde_json::from_value::<Record>(value)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(&format!("missing field `{field}`")),
                "{error}"
            );
        }
        let mut altered: Signed = decode(&frame).unwrap();
        if let Body::Prepare { sent, .. } = &mut altered.record.body {
            sent[0].1 = [5; 32];
        }
        assert!(verify(&encode(&altered).unwrap(), [48; 32], &keys()).is_err());
    }
    #[test]
    fn prepare_delivery_sets_require_bounded_unique_canonical_order() {
        for invalid in [
            vec![(9, [1; 32]), (9, [1; 32])],
            vec![(9, [2; 32]), (9, [1; 32])],
            vec![(9, [1; 32]), (8, [2; 32])],
            (0..=MAX_MEMBERS).map(|n| (n as u32, [1; 32])).collect(),
        ] {
            for received in [false, true] {
                let mut r = prepare();
                if let Body::Prepare {
                    sent,
                    received: incoming,
                    ..
                } = &mut r.body
                {
                    if received {
                        *incoming = invalid.clone();
                    } else {
                        *sent = invalid.clone();
                    }
                }
                assert!(sign(&r, &key()).is_err());
            }
        }
        let mut r = record();
        r.body = Body::Join {
            participant: 7,
            generation: 3,
            activation: [1; 32],
            floor: 0,
            cut: 0,
            checkpoint: [2; 32],
            frontiers: vec![(9, 4)],
            retained: vec![[3; 32]],
        };
        assert_eq!(
            verify(&sign(&r, &key()).unwrap(), [48; 32], &keys()).unwrap(),
            r
        );
    }
}
