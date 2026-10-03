//! Versioned, purpose-specific calibration seed identities.
//!
//! This module owns the v1 seed-map framing and a finite-run collision
//! registry. It does not change the Kairos engine RNG or claim that 64-bit
//! derived seeds are globally collision-free.

use kairo_ecs_rng::DeterministicStream;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

const DOMAIN: &[u8] = b"kairos.calibration.seed-map";
pub const SEED_MAP_VERSION_V1: u32 = 1;
pub const STREAM_VERSION_V1: u32 = 1;
pub const MAX_ID_BYTES: usize = 1024;

/// Stable purpose tags in `calibration-seed-map.v1`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum SeedPurpose {
    Service = 1,
    Transit = 2,
    Behavior = 3,
    Calibration = 4,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SeedIdentity {
    version: u32,
    root_seed: u64,
    replication_id: u64,
    study_id: String,
    seed_schedule_id: String,
    case_key: String,
    task_key: String,
    purpose: SeedPurpose,
}

/// Errors in seed-map construction, finite-run collision registration, or
/// stream continuation.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum CalibrationSeedError {
    #[error("unsupported calibration seed-map version {0}")]
    UnsupportedSeedMapVersion(u32),
    #[error("unsupported calibration stream version {0}")]
    UnsupportedStreamVersion(u32),
    #[error("invalid or noncanonical identifier in field {0}")]
    InvalidIdentifier(&'static str),
    #[error("distinct seed identities produced the same u64 seed {seed}")]
    SeedCollision { seed: u64 },
    #[error("calibration stream draw position overflow")]
    DrawPositionOverflow,
    #[error("owned stream snapshot identity/seed mismatch")]
    InvalidSnapshot,
}

/// Registry scoped to one finite study/run across all of its seed schedules.
/// Candidate ID is intentionally not a seed component; paired candidates use
/// the same schedule ID. Retain one instance for the whole study/run and
/// register identities through it before distributing streams; another map
/// instance has an independent collision registry.
pub struct CalibrationSeedMap {
    version: u32,
    root_seed: u64,
    study_id: String,
    registered: HashMap<u64, SeedIdentity>,
}

impl CalibrationSeedMap {
    /// Create a seed map, failing closed for versions this package does not
    /// implement. The root seed may be zero.
    pub fn new(version: u32, study_id: &str, root_seed: u64) -> Result<Self, CalibrationSeedError> {
        if version != SEED_MAP_VERSION_V1 {
            return Err(CalibrationSeedError::UnsupportedSeedMapVersion(version));
        }
        validate_id("study_id", study_id)?;
        Ok(Self {
            version,
            root_seed,
            study_id: study_id.to_owned(),
            registered: HashMap::new(),
        })
    }

    /// Derive and register the stream for one case/task/purpose identity.
    /// Re-registering the identical identity is idempotent and returns a fresh
    /// stream at draw position zero; callers own and advance each returned
    /// stream exactly once for its logical purpose. `case_key` must already be
    /// pseudonymous or synthetic; this API must never receive raw patient IDs.
    pub fn stream_for(
        &mut self,
        seed_schedule_id: &str,
        replication_id: u64,
        case_key: &str,
        task_key: &str,
        purpose: SeedPurpose,
    ) -> Result<CalibrationStream, CalibrationSeedError> {
        validate_id("seed_schedule_id", seed_schedule_id)?;
        validate_id("case_key", case_key)?;
        validate_id("task_key", task_key)?;
        let identity = SeedIdentity {
            version: self.version,
            root_seed: self.root_seed,
            replication_id,
            study_id: self.study_id.clone(),
            seed_schedule_id: seed_schedule_id.to_owned(),
            case_key: case_key.to_owned(),
            task_key: task_key.to_owned(),
            purpose,
        };
        let seed = derive_seed(&identity)?;
        self.register_seed(seed, identity.clone())?;
        Ok(CalibrationStream {
            identity,
            seed_map_version: self.version,
            stream_version: STREAM_VERSION_V1,
            derived_seed: seed,
            stream: DeterministicStream::from_seed(seed),
            draw_position: 0,
        })
    }

    fn register_seed(
        &mut self,
        seed: u64,
        identity: SeedIdentity,
    ) -> Result<(), CalibrationSeedError> {
        match self.registered.get(&seed) {
            Some(existing) if existing != &identity => {
                Err(CalibrationSeedError::SeedCollision { seed })
            }
            Some(_) => Ok(()),
            None => {
                self.registered.insert(seed, identity);
                Ok(())
            }
        }
    }
}

/// Owned purpose stream with checked completed-draw accounting.
pub struct CalibrationStream {
    identity: SeedIdentity,
    seed_map_version: u32,
    stream_version: u32,
    derived_seed: u64,
    stream: DeterministicStream,
    draw_position: u64,
}

impl CalibrationStream {
    pub fn derived_seed(&self) -> u64 {
        self.derived_seed
    }

    /// Number of completed PRNG transitions. Both `next_u64` and `next_u32`
    /// advance this by exactly one.
    pub fn draw_position(&self) -> u64 {
        self.draw_position
    }

    /// Advance one SplitMix64 transition. Counter overflow is detected before
    /// the underlying stream is mutated.
    pub fn next_u64(&mut self) -> Result<u64, CalibrationSeedError> {
        let next_position = self
            .draw_position
            .checked_add(1)
            .ok_or(CalibrationSeedError::DrawPositionOverflow)?;
        let value = self.stream.next_u64();
        self.draw_position = next_position;
        Ok(value)
    }

    /// Return the low 32 bits of one transition, matching the engine RNG API.
    pub fn next_u32(&mut self) -> Result<u32, CalibrationSeedError> {
        self.next_u64().map(|value| value as u32)
    }

    /// Create an opaque owned snapshot of the exact current RNG state and draw
    /// position. This is an in-memory boundary, not a serialization codec.
    pub fn snapshot(&self) -> CalibrationStreamSnapshot {
        CalibrationStreamSnapshot {
            identity: self.identity.clone(),
            seed_map_version: self.seed_map_version,
            stream_version: self.stream_version,
            derived_seed: self.derived_seed,
            current_state: self.stream.clone().into_inner(),
            draw_position: self.draw_position,
        }
    }
}

/// Opaque, owned in-memory continuation. Private fields prevent callers from
/// forging state; portable validation/serialization belongs to a separate
/// contract and implementation.
pub struct CalibrationStreamSnapshot {
    identity: SeedIdentity,
    seed_map_version: u32,
    stream_version: u32,
    derived_seed: u64,
    current_state: u64,
    draw_position: u64,
}

impl CalibrationStreamSnapshot {
    /// Restore at the next draw from the captured private state, without
    /// restarting or replaying the original seed.
    pub fn restore(self) -> Result<CalibrationStream, CalibrationSeedError> {
        if self.seed_map_version != SEED_MAP_VERSION_V1 {
            return Err(CalibrationSeedError::UnsupportedSeedMapVersion(
                self.seed_map_version,
            ));
        }
        if self.stream_version != STREAM_VERSION_V1 {
            return Err(CalibrationSeedError::UnsupportedStreamVersion(
                self.stream_version,
            ));
        }
        let expected_seed = derive_seed(&self.identity)?;
        if expected_seed != self.derived_seed {
            return Err(CalibrationSeedError::InvalidSnapshot);
        }
        Ok(CalibrationStream {
            identity: self.identity,
            seed_map_version: self.seed_map_version,
            stream_version: self.stream_version,
            derived_seed: self.derived_seed,
            stream: DeterministicStream::from_seed(self.current_state),
            draw_position: self.draw_position,
        })
    }
}

fn validate_id(field: &'static str, value: &str) -> Result<(), CalibrationSeedError> {
    if value.is_empty()
        || value.len() > MAX_ID_BYTES
        || value.chars().any(is_contract_control)
        || value.chars().next().is_some_and(is_contract_whitespace)
        || value
            .chars()
            .next_back()
            .is_some_and(is_contract_whitespace)
    {
        return Err(CalibrationSeedError::InvalidIdentifier(field));
    }
    Ok(())
}

// Unicode White_Space property list, fixed here so Rust's bundled Unicode
// table version cannot change v1 identity acceptance at a toolchain upgrade.
fn is_contract_whitespace(character: char) -> bool {
    matches!(character,
        '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{0085}' | '\u{00A0}' |
        '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' |
        '\u{202F}' | '\u{205F}' | '\u{3000}')
}

fn is_contract_control(character: char) -> bool {
    matches!(character, '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}')
}

fn derive_seed(identity: &SeedIdentity) -> Result<u64, CalibrationSeedError> {
    if identity.version != SEED_MAP_VERSION_V1 {
        return Err(CalibrationSeedError::UnsupportedSeedMapVersion(
            identity.version,
        ));
    }
    let bytes = encode_identity(identity)?;
    let digest = Sha256::digest(bytes);
    let first_eight: [u8; 8] = digest[..8]
        .try_into()
        .expect("SHA-256 digest has at least eight bytes");
    Ok(u64::from_le_bytes(first_eight))
}

fn encode_identity(identity: &SeedIdentity) -> Result<Vec<u8>, CalibrationSeedError> {
    let mut bytes = Vec::with_capacity(DOMAIN.len() + 24 + 4 * (8 + MAX_ID_BYTES));
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&identity.version.to_le_bytes());
    bytes.extend_from_slice(&identity.root_seed.to_le_bytes());
    bytes.extend_from_slice(&identity.replication_id.to_le_bytes());
    for (field, value) in [
        ("study_id", identity.study_id.as_str()),
        ("seed_schedule_id", identity.seed_schedule_id.as_str()),
        ("case_key", identity.case_key.as_str()),
        ("task_key", identity.task_key.as_str()),
    ] {
        validate_id(field, value)?;
        let length = u64::try_from(value.len())
            .map_err(|_| CalibrationSeedError::InvalidIdentifier(field))?;
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes.extend_from_slice(&(identity.purpose as u32).to_le_bytes());
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(purpose: SeedPurpose) -> SeedIdentity {
        SeedIdentity {
            version: SEED_MAP_VERSION_V1,
            root_seed: 1234,
            replication_id: 7,
            study_id: "study-α".to_owned(),
            seed_schedule_id: "crn-v1".to_owned(),
            case_key: "case-0001".to_owned(),
            task_key: "triage:1".to_owned(),
            purpose,
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn normative_golden_bytes_digest_seed_and_draws_match() {
        let id = identity(SeedPurpose::Service);
        let encoded = encode_identity(&id).unwrap();
        assert_eq!(encoded.len(), 114);
        assert_eq!(encoded.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "6b6169726f732e63616c6962726174696f6e2e736565642d6d617001000000d2040000000000000700000000000000080000000000000073747564792dceb1060000000000000063726e2d76310900000000000000636173652d3030303108000000000000007472696167653a3101000000");
        let digest = Sha256::digest(&encoded);
        assert_eq!(
            hex(&digest),
            "fba126d77ad6094874929e96c01d9b25adf9f6801235af165fc7c3d6fae930a2"
        );
        assert_eq!(derive_seed(&id).unwrap(), 5_190_915_868_605_194_747);
        let mut stream = DeterministicStream::from_seed(derive_seed(&id).unwrap());
        assert_eq!(stream.next_u64(), 0xcabf_5867_c66c_f8ef);
        assert_eq!(stream.next_u64(), 0xa20d_8e83_7c7b_ea4e);
        assert_eq!(stream.next_u64(), 0xee85_ff56_f99b_d5b4);
    }

    #[test]
    fn purpose_tags_are_exact_u32_little_endian_values() {
        for (purpose, code) in [
            (SeedPurpose::Service, 1u32),
            (SeedPurpose::Transit, 2),
            (SeedPurpose::Behavior, 3),
            (SeedPurpose::Calibration, 4),
        ] {
            let encoded = encode_identity(&identity(purpose)).unwrap();
            assert_eq!(&encoded[encoded.len() - 4..], &code.to_le_bytes());
        }
    }

    #[test]
    fn identity_validation_rejects_invalid_and_oversized_ids() {
        assert_eq!(
            validate_id("study_id", ""),
            Err(CalibrationSeedError::InvalidIdentifier("study_id"))
        );
        assert_eq!(
            validate_id("case_key", " leading"),
            Err(CalibrationSeedError::InvalidIdentifier("case_key"))
        );
        assert_eq!(
            validate_id("case_key", "\u{00A0}leading"),
            Err(CalibrationSeedError::InvalidIdentifier("case_key"))
        );
        assert_eq!(
            validate_id("task_key", "trailing "),
            Err(CalibrationSeedError::InvalidIdentifier("task_key"))
        );
        assert_eq!(
            validate_id("study_id", "bad\u{0000}id"),
            Err(CalibrationSeedError::InvalidIdentifier("study_id"))
        );
        assert_eq!(
            validate_id("task_key", &"x".repeat(MAX_ID_BYTES + 1)),
            Err(CalibrationSeedError::InvalidIdentifier("task_key"))
        );
        assert!(validate_id("case_key", &"x".repeat(MAX_ID_BYTES)).is_ok());
        // The contract's bound is UTF-8 bytes, not Unicode scalar values.
        let exactly_at_limit = format!("{}aa", "é".repeat((MAX_ID_BYTES - 2) / 2));
        assert_eq!(exactly_at_limit.len(), MAX_ID_BYTES);
        assert!(validate_id("case_key", &exactly_at_limit).is_ok());
        let one_byte_over = format!("{}a", "é".repeat(MAX_ID_BYTES / 2));
        assert_eq!(one_byte_over.len(), MAX_ID_BYTES + 1);
        assert_eq!(
            validate_id("case_key", &one_byte_over),
            Err(CalibrationSeedError::InvalidIdentifier("case_key"))
        );
    }

    #[test]
    fn finite_registry_is_idempotent_for_same_identity_and_rejects_collision() {
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let first = identity(SeedPurpose::Service);
        let other = SeedIdentity {
            seed_schedule_id: "different-schedule".to_owned(),
            ..first.clone()
        };
        map.register_seed(9, first.clone()).unwrap();
        assert_eq!(
            map.register_seed(9, other),
            Err(CalibrationSeedError::SeedCollision { seed: 9 })
        );
        assert_eq!(map.registered.get(&9), Some(&first));
        // Collision failure leaves the original identity available and
        // re-registering it remains idempotent.
        map.register_seed(9, first.clone()).unwrap();
        assert_eq!(map.registered.get(&9), Some(&first));
    }

    #[test]
    fn every_identity_field_and_root_seed_change_the_frame_and_derived_seed() {
        let baseline = identity(SeedPurpose::Service);
        let baseline_frame = encode_identity(&baseline).unwrap();
        let baseline_seed = derive_seed(&baseline).unwrap();
        let variants = [
            SeedIdentity {
                study_id: "study-other".to_owned(),
                ..baseline.clone()
            },
            SeedIdentity {
                seed_schedule_id: "schedule-other".to_owned(),
                ..baseline.clone()
            },
            SeedIdentity {
                replication_id: baseline.replication_id + 1,
                ..baseline.clone()
            },
            SeedIdentity {
                case_key: "case-other".to_owned(),
                ..baseline.clone()
            },
            SeedIdentity {
                task_key: "task-other".to_owned(),
                ..baseline.clone()
            },
            SeedIdentity {
                purpose: SeedPurpose::Transit,
                ..baseline.clone()
            },
            SeedIdentity {
                root_seed: baseline.root_seed + 1,
                ..baseline.clone()
            },
        ];

        for changed in variants {
            assert_ne!(encode_identity(&changed).unwrap(), baseline_frame);
            assert_ne!(derive_seed(&changed).unwrap(), baseline_seed);
        }
    }

    #[test]
    fn length_framing_separates_ambiguous_concatenations() {
        let left = SeedIdentity {
            study_id: "ab".to_owned(),
            seed_schedule_id: "c".to_owned(),
            ..identity(SeedPurpose::Service)
        };
        let right = SeedIdentity {
            study_id: "a".to_owned(),
            seed_schedule_id: "bc".to_owned(),
            ..identity(SeedPurpose::Service)
        };
        assert_eq!(
            format!("{}{}", left.study_id, left.seed_schedule_id),
            format!("{}{}", right.study_id, right.seed_schedule_id)
        );
        assert_ne!(
            encode_identity(&left).unwrap(),
            encode_identity(&right).unwrap()
        );
        assert_ne!(derive_seed(&left).unwrap(), derive_seed(&right).unwrap());
    }

    #[test]
    fn overflow_does_not_advance_rng_or_draw_position() {
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let mut stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        let state_before = stream.stream.clone().into_inner();
        stream.draw_position = u64::MAX;
        assert_eq!(
            stream.next_u64(),
            Err(CalibrationSeedError::DrawPositionOverflow)
        );
        assert_eq!(stream.draw_position, u64::MAX);
        assert_eq!(stream.stream.clone().into_inner(), state_before);
    }

    #[test]
    fn unknown_snapshot_versions_and_seed_mismatch_fail_closed() {
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Transit)
            .unwrap();
        let mut snapshot = stream.snapshot();
        snapshot.stream_version = 99;
        assert!(matches!(
            snapshot.restore(),
            Err(CalibrationSeedError::UnsupportedStreamVersion(99))
        ));

        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Transit)
            .unwrap();
        let mut snapshot = stream.snapshot();
        snapshot.seed_map_version = 99;
        assert!(matches!(
            snapshot.restore(),
            Err(CalibrationSeedError::UnsupportedSeedMapVersion(99))
        ));

        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Transit)
            .unwrap();
        let mut snapshot = stream.snapshot();
        snapshot.derived_seed ^= 1;
        assert!(matches!(
            snapshot.restore(),
            Err(CalibrationSeedError::InvalidSnapshot)
        ));
    }
}
