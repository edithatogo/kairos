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
pub(crate) struct SeedIdentity {
    pub(crate) version: u32,
    pub(crate) root_seed: u64,
    pub(crate) replication_id: u64,
    pub(crate) study_id: String,
    pub(crate) seed_schedule_id: String,
    pub(crate) case_key: String,
    pub(crate) task_key: String,
    pub(crate) purpose: SeedPurpose,
}

/// Owned engine-native registry state for the future C2 envelope layer.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private portable checkpoint adapter"
    )
)]
pub(crate) struct SeedMapCheckpointStateV1 {
    pub(crate) version: u32,
    pub(crate) root_seed: u64,
    pub(crate) study_id: String,
    pub(crate) entries: Vec<SeedMapCheckpointEntryV1>,
}

/// One canonical seed-to-logical-identity registry entry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private portable checkpoint adapter"
    )
)]
pub(crate) struct SeedMapCheckpointEntryV1 {
    pub(crate) seed: u64,
    pub(crate) identity: SeedIdentity,
}

/// Resource limits applied before registry checkpoint cloning or import allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private portable checkpoint adapter"
    )
)]
pub(crate) struct SeedRegistryCheckpointLimits {
    pub(crate) max_entries: usize,
    pub(crate) max_identifier_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private portable checkpoint adapter"
    )
)]
pub(crate) enum SeedRegistryCheckpointError {
    #[error(transparent)]
    Seed(#[from] CalibrationSeedError),
    #[error("invalid calibration seed registry checkpoint")]
    InvalidState,
    #[error("calibration seed registry checkpoint exceeds caller limits")]
    LimitExceeded,
    #[error("calibration seed registry checkpoint allocation failed")]
    AllocationFailed,
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

    /// Capture all collision-registry entries in canonical seed order.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by the future crate-private portable checkpoint adapter"
        )
    )]
    pub(crate) fn checkpoint_state(
        &self,
        limits: SeedRegistryCheckpointLimits,
    ) -> Result<SeedMapCheckpointStateV1, SeedRegistryCheckpointError> {
        if self.registered.len() > limits.max_entries {
            return Err(SeedRegistryCheckpointError::LimitExceeded);
        }
        let mut identifier_bytes = self.study_id.len();
        for identity in self.registered.values() {
            identifier_bytes = identifier_bytes
                .checked_add(identity_identifier_bytes(identity)?)
                .ok_or(SeedRegistryCheckpointError::LimitExceeded)?;
            if identifier_bytes > limits.max_identifier_bytes {
                return Err(SeedRegistryCheckpointError::LimitExceeded);
            }
        }
        if identifier_bytes > limits.max_identifier_bytes {
            return Err(SeedRegistryCheckpointError::LimitExceeded);
        }

        let mut entries = Vec::new();
        entries
            .try_reserve_exact(self.registered.len())
            .map_err(|_| SeedRegistryCheckpointError::AllocationFailed)?;
        for (&seed, identity) in &self.registered {
            entries.push(SeedMapCheckpointEntryV1 {
                seed,
                identity: identity.clone(),
            });
        }
        entries.sort_unstable_by_key(|entry| entry.seed);
        Ok(SeedMapCheckpointStateV1 {
            version: self.version,
            root_seed: self.root_seed,
            study_id: self.study_id.clone(),
            entries,
        })
    }

    /// Validate a complete state before allocating or exposing a restored map.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by the future crate-private portable checkpoint adapter"
        )
    )]
    pub(crate) fn from_checkpoint_state(
        state: SeedMapCheckpointStateV1,
        limits: SeedRegistryCheckpointLimits,
    ) -> Result<Self, SeedRegistryCheckpointError> {
        if state.version != SEED_MAP_VERSION_V1 {
            return Err(CalibrationSeedError::UnsupportedSeedMapVersion(state.version).into());
        }
        validate_id("study_id", &state.study_id)?;
        if state.entries.len() > limits.max_entries {
            return Err(SeedRegistryCheckpointError::LimitExceeded);
        }
        let mut identifier_bytes = state.study_id.len();
        let mut previous_seed = None;
        for entry in &state.entries {
            if previous_seed.is_some_and(|previous| previous >= entry.seed) {
                return Err(SeedRegistryCheckpointError::InvalidState);
            }
            previous_seed = Some(entry.seed);
            let identity = &entry.identity;
            if identity.version != state.version
                || identity.root_seed != state.root_seed
                || identity.study_id != state.study_id
            {
                return Err(SeedRegistryCheckpointError::InvalidState);
            }
            identifier_bytes = identifier_bytes
                .checked_add(identity_identifier_bytes(identity)?)
                .ok_or(SeedRegistryCheckpointError::LimitExceeded)?;
            if identifier_bytes > limits.max_identifier_bytes {
                return Err(SeedRegistryCheckpointError::LimitExceeded);
            }
        }
        if identifier_bytes > limits.max_identifier_bytes {
            return Err(SeedRegistryCheckpointError::LimitExceeded);
        }

        for entry in &state.entries {
            if derive_seed(&entry.identity)? != entry.seed {
                return Err(SeedRegistryCheckpointError::InvalidState);
            }
        }

        let mut registered = HashMap::new();
        registered
            .try_reserve(state.entries.len())
            .map_err(|_| SeedRegistryCheckpointError::AllocationFailed)?;
        for entry in state.entries {
            registered.insert(entry.seed, entry.identity);
        }
        Ok(Self {
            version: state.version,
            root_seed: state.root_seed,
            study_id: state.study_id,
            registered,
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
        let identity = self.identity_for(
            seed_schedule_id,
            replication_id,
            case_key,
            task_key,
            purpose,
        )?;
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

    /// Derive and register the expected identity without creating another
    /// advancing stream. Repeated requests for the same identity are stable.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Opaque C2.2 expected keys are currently consumed by private provider code and tests"
        )
    )]
    pub(crate) fn key_for(
        &mut self,
        seed_schedule_id: &str,
        replication_id: u64,
        case_key: &str,
        task_key: &str,
        purpose: SeedPurpose,
    ) -> Result<CalibrationStreamKey, CalibrationSeedError> {
        let identity = self.identity_for(
            seed_schedule_id,
            replication_id,
            case_key,
            task_key,
            purpose,
        )?;
        let seed = derive_seed(&identity)?;
        self.register_seed(seed, identity.clone())?;
        Ok(CalibrationStreamKey { identity })
    }

    fn identity_for(
        &self,
        seed_schedule_id: &str,
        replication_id: u64,
        case_key: &str,
        task_key: &str,
        purpose: SeedPurpose,
    ) -> Result<SeedIdentity, CalibrationSeedError> {
        validate_id("seed_schedule_id", seed_schedule_id)?;
        validate_id("case_key", case_key)?;
        validate_id("task_key", task_key)?;
        Ok(SeedIdentity {
            version: self.version,
            root_seed: self.root_seed,
            replication_id,
            study_id: self.study_id.clone(),
            seed_schedule_id: seed_schedule_id.to_owned(),
            case_key: case_key.to_owned(),
            task_key: task_key.to_owned(),
            purpose,
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

fn identity_identifier_bytes(
    identity: &SeedIdentity,
) -> Result<usize, SeedRegistryCheckpointError> {
    let mut total = 0usize;
    for (field, value) in [
        ("study_id", identity.study_id.as_str()),
        ("seed_schedule_id", identity.seed_schedule_id.as_str()),
        ("case_key", identity.case_key.as_str()),
        ("task_key", identity.task_key.as_str()),
    ] {
        validate_id(field, value)?;
        total = total
            .checked_add(value.len())
            .ok_or(SeedRegistryCheckpointError::LimitExceeded)?;
    }
    Ok(total)
}

/// Opaque expected identity for one logical calibration stream.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct CalibrationStreamKey {
    identity: SeedIdentity,
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

/// Complete native continuation state for the experimental C2 checkpoint
/// assembly. This is not a serialized format or evidence of RNG history.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private C2 checkpoint assembler"
    )
)]
pub(crate) struct CalibrationStreamStateV1 {
    pub(crate) version: u32,
    pub(crate) identity: SeedIdentity,
    pub(crate) seed_map_version: u32,
    pub(crate) stream_version: u32,
    pub(crate) derived_seed: u64,
    pub(crate) current_state: u64,
    pub(crate) draw_position: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private C2 checkpoint assembler"
    )
)]
pub(crate) struct CalibrationStreamStateLimits {
    pub(crate) max_identifier_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private C2 checkpoint assembler"
    )
)]
pub(crate) enum CalibrationStreamStateError {
    #[error(transparent)]
    Seed(#[from] CalibrationSeedError),
    #[error("calibration stream state exceeds caller limits")]
    LimitExceeded,
    #[error("invalid calibration stream state image")]
    InvalidState,
}

const CALIBRATION_STREAM_STATE_VERSION_V1: u32 = 1;

impl CalibrationStream {
    pub fn derived_seed(&self) -> u64 {
        self.derived_seed
    }

    /// Number of completed PRNG transitions. Both `next_u64` and `next_u32`
    /// advance this by exactly one.
    pub fn draw_position(&self) -> u64 {
        self.draw_position
    }

    pub(crate) fn key(&self) -> CalibrationStreamKey {
        CalibrationStreamKey {
            identity: self.identity.clone(),
        }
    }

    pub(crate) fn purpose(&self) -> SeedPurpose {
        self.identity.purpose
    }

    /// Capture complete owned state after checking the aggregate identifier
    /// budget and state invariants, before cloning any identity strings.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by the future crate-private C2 checkpoint assembler"
        )
    )]
    pub(crate) fn checkpoint_state(
        &self,
        limits: CalibrationStreamStateLimits,
    ) -> Result<CalibrationStreamStateV1, CalibrationStreamStateError> {
        let identifier_bytes = stream_identity_identifier_bytes(&self.identity)?;
        if identifier_bytes > limits.max_identifier_bytes {
            return Err(CalibrationStreamStateError::LimitExceeded);
        }
        if self.seed_map_version != SEED_MAP_VERSION_V1
            || self.identity.version != self.seed_map_version
            || self.stream_version != STREAM_VERSION_V1
            || derive_seed(&self.identity)? != self.derived_seed
            || (self.draw_position == 0 && self.stream.clone().into_inner() != self.derived_seed)
        {
            return Err(CalibrationStreamStateError::InvalidState);
        }
        Ok(CalibrationStreamStateV1 {
            version: CALIBRATION_STREAM_STATE_VERSION_V1,
            identity: self.identity.clone(),
            seed_map_version: self.seed_map_version,
            stream_version: self.stream_version,
            derived_seed: self.derived_seed,
            current_state: self.stream.clone().into_inner(),
            draw_position: self.draw_position,
        })
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

impl CalibrationStreamStateV1 {
    /// Restore exact current state only for the owner-derived complete identity.
    /// No root-seed replay or draw-history validation is performed.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by the future crate-private C2 checkpoint assembler"
        )
    )]
    pub(crate) fn restore_for(
        self,
        expected: &CalibrationStreamKey,
        limits: CalibrationStreamStateLimits,
    ) -> Result<CalibrationStream, CalibrationStreamStateError> {
        if self.version != CALIBRATION_STREAM_STATE_VERSION_V1 {
            return Err(CalibrationStreamStateError::InvalidState);
        }
        if stream_identity_identifier_bytes(&self.identity)? > limits.max_identifier_bytes {
            return Err(CalibrationStreamStateError::LimitExceeded);
        }
        if self.seed_map_version != SEED_MAP_VERSION_V1
            || self.stream_version != STREAM_VERSION_V1
            || self.identity.version != self.seed_map_version
            || self.identity != expected.identity
            || (self.draw_position == 0 && self.current_state != self.derived_seed)
        {
            return Err(CalibrationStreamStateError::InvalidState);
        }
        let snapshot = CalibrationStreamSnapshot {
            identity: self.identity,
            seed_map_version: self.seed_map_version,
            stream_version: self.stream_version,
            derived_seed: self.derived_seed,
            current_state: self.current_state,
            draw_position: self.draw_position,
        };
        snapshot
            .restore_for(expected)
            .map_err(CalibrationStreamStateError::Seed)
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Consumed by the future crate-private C2 checkpoint assembler"
    )
)]
fn stream_identity_identifier_bytes(
    identity: &SeedIdentity,
) -> Result<usize, CalibrationStreamStateError> {
    let mut total = 0usize;
    for (field, value) in [
        ("study_id", identity.study_id.as_str()),
        ("seed_schedule_id", identity.seed_schedule_id.as_str()),
        ("case_key", identity.case_key.as_str()),
        ("task_key", identity.task_key.as_str()),
    ] {
        validate_id(field, value)?;
        total = total
            .checked_add(value.len())
            .ok_or(CalibrationStreamStateError::LimitExceeded)?;
    }
    Ok(total)
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

    /// Restore only when every logical identity component matches the expected
    /// owner, before applying the existing snapshot version and seed checks.
    pub(crate) fn restore_for(
        self,
        expected: &CalibrationStreamKey,
    ) -> Result<CalibrationStream, CalibrationSeedError> {
        if self.identity != expected.identity {
            return Err(CalibrationSeedError::InvalidSnapshot);
        }
        self.restore()
    }
}

pub(crate) fn validate_id(field: &'static str, value: &str) -> Result<(), CalibrationSeedError> {
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
    fn registry_checkpoint_restores_all_purposes_and_future_collision_checks() {
        let mut source = CalibrationSeedMap::new(1, "study", 1234).unwrap();
        let identities = [
            SeedPurpose::Service,
            SeedPurpose::Transit,
            SeedPurpose::Behavior,
            SeedPurpose::Calibration,
        ];
        for purpose in identities {
            let _ = source
                .stream_for("schedule", 9, "case", "task", purpose)
                .unwrap();
        }

        let state = source
            .checkpoint_state(SeedRegistryCheckpointLimits {
                max_entries: 4,
                max_identifier_bytes: 128,
            })
            .unwrap();
        assert!(state
            .entries
            .windows(2)
            .all(|pair| pair[0].seed < pair[1].seed));
        let mut restored = CalibrationSeedMap::from_checkpoint_state(
            state,
            SeedRegistryCheckpointLimits {
                max_entries: 4,
                max_identifier_bytes: 128,
            },
        )
        .unwrap();

        for purpose in identities {
            restored
                .stream_for("schedule", 9, "case", "task", purpose)
                .unwrap();
        }
        let registered_identity = restored
            .registered
            .values()
            .find(|identity| identity.purpose == SeedPurpose::Service)
            .unwrap()
            .clone();
        let collision_identity = SeedIdentity {
            seed_schedule_id: "different-schedule".into(),
            ..registered_identity.clone()
        };
        let collided_seed = derive_seed(&registered_identity).unwrap();
        assert_eq!(
            restored.register_seed(collided_seed, collision_identity),
            Err(CalibrationSeedError::SeedCollision {
                seed: collided_seed
            })
        );
    }

    #[test]
    fn registry_checkpoint_rejects_malformed_state_without_changing_source() {
        let mut source = CalibrationSeedMap::new(1, "study", 1234).unwrap();
        source
            .stream_for("schedule", 9, "case", "task", SeedPurpose::Service)
            .unwrap();
        let original = source.registered.clone();
        let state = source
            .checkpoint_state(SeedRegistryCheckpointLimits {
                max_entries: 8,
                max_identifier_bytes: 128,
            })
            .unwrap();

        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(
                state.clone(),
                SeedRegistryCheckpointLimits {
                    max_entries: 0,
                    max_identifier_bytes: 128,
                }
            )
            .err()
            .unwrap(),
            SeedRegistryCheckpointError::LimitExceeded
        );

        let mut bad_version = state.clone();
        bad_version.version = 2;
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(
                bad_version,
                SeedRegistryCheckpointLimits {
                    max_entries: 8,
                    max_identifier_bytes: 128,
                }
            )
            .err()
            .unwrap(),
            SeedRegistryCheckpointError::Seed(CalibrationSeedError::UnsupportedSeedMapVersion(2))
        );

        let mut bad_order = state.clone();
        let first = bad_order.entries[0].clone();
        bad_order.entries.push(first);
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(
                bad_order,
                SeedRegistryCheckpointLimits {
                    max_entries: 8,
                    max_identifier_bytes: 128,
                }
            )
            .err()
            .unwrap(),
            SeedRegistryCheckpointError::InvalidState
        );

        assert!(matches!(
            source.checkpoint_state(SeedRegistryCheckpointLimits {
                max_entries: 0,
                max_identifier_bytes: 128,
            }),
            Err(SeedRegistryCheckpointError::LimitExceeded)
        ));
        assert!(matches!(
            source.checkpoint_state(SeedRegistryCheckpointLimits {
                max_entries: 1,
                max_identifier_bytes: 1,
            }),
            Err(SeedRegistryCheckpointError::LimitExceeded)
        ));
        assert_eq!(source.registered, original);
    }

    #[test]
    fn registry_checkpoint_import_rejects_identity_and_seed_mutations() {
        let mut source = CalibrationSeedMap::new(1, "study", 1234).unwrap();
        for purpose in [SeedPurpose::Service, SeedPurpose::Transit] {
            source
                .stream_for("schedule", 9, "case", "task", purpose)
                .unwrap();
        }
        let limits = SeedRegistryCheckpointLimits {
            max_entries: 2,
            max_identifier_bytes: 128,
        };
        let state = source.checkpoint_state(limits).unwrap();

        let mut unsorted = state.clone();
        unsorted.entries.reverse();
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(unsorted, limits)
                .err()
                .unwrap(),
            SeedRegistryCheckpointError::InvalidState
        );

        let mut wrong_root = state.clone();
        wrong_root.entries[0].identity.root_seed += 1;
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(wrong_root, limits)
                .err()
                .unwrap(),
            SeedRegistryCheckpointError::InvalidState
        );

        let mut wrong_study = state.clone();
        wrong_study.entries[0].identity.study_id = "other-study".into();
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(wrong_study, limits)
                .err()
                .unwrap(),
            SeedRegistryCheckpointError::InvalidState
        );

        let mut wrong_seed = state.clone();
        wrong_seed.entries[0].seed ^= 1;
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(wrong_seed, limits)
                .err()
                .unwrap(),
            SeedRegistryCheckpointError::InvalidState
        );

        let mut invalid_id = state;
        invalid_id.entries[0].identity.case_key = " bad".into();
        assert!(matches!(
            CalibrationSeedMap::from_checkpoint_state(invalid_id, limits),
            Err(SeedRegistryCheckpointError::Seed(
                CalibrationSeedError::InvalidIdentifier("case_key")
            ))
        ));
        assert_eq!(source.registered.len(), 2);
    }

    #[test]
    fn registry_checkpoint_applies_cumulative_key_limits_and_empty_registry_caps() {
        let mut source = CalibrationSeedMap::new(1, "s", 1234).unwrap();
        for schedule in ["a", "b"] {
            source
                .stream_for(schedule, 9, "c", "t", SeedPurpose::Service)
                .unwrap();
        }

        let per_identity_under_cap = SeedRegistryCheckpointLimits {
            max_entries: 2,
            max_identifier_bytes: 8,
        };
        assert_eq!(
            source
                .checkpoint_state(per_identity_under_cap)
                .err()
                .unwrap(),
            SeedRegistryCheckpointError::LimitExceeded,
            "the map study ID plus both complete identities exceeds the cumulative cap"
        );

        let within_total_cap = SeedRegistryCheckpointLimits {
            max_entries: 2,
            max_identifier_bytes: 9,
        };
        let state = source.checkpoint_state(within_total_cap).unwrap();
        assert_eq!(
            CalibrationSeedMap::from_checkpoint_state(state, per_identity_under_cap)
                .err()
                .unwrap(),
            SeedRegistryCheckpointError::LimitExceeded
        );

        let empty = CalibrationSeedMap::new(1, "s", 0).unwrap();
        let empty_caps = SeedRegistryCheckpointLimits {
            max_entries: 0,
            max_identifier_bytes: 1,
        };
        let empty_state = empty.checkpoint_state(empty_caps).unwrap();
        assert!(empty_state.entries.is_empty());
        let restored = CalibrationSeedMap::from_checkpoint_state(empty_state, empty_caps).unwrap();
        assert!(restored.registered.is_empty());
        assert_eq!(
            empty.checkpoint_state(SeedRegistryCheckpointLimits {
                max_entries: 0,
                max_identifier_bytes: 0,
            }),
            Err(SeedRegistryCheckpointError::LimitExceeded)
        );
    }

    #[test]
    fn normative_golden_bytes_digest_seed_and_draws_match() {
        let id = identity(SeedPurpose::Service);
        let encoded = encode_identity(&id).unwrap();
        assert_eq!(encoded.len(), 114);
        assert_eq!(
            encoded
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            "6b6169726f732e63616c6962726174696f6e2e736565642d6d617001000000d2040000000000000700000000000000080000000000000073747564792dceb1060000000000000063726e2d76310900000000000000636173652d3030303108000000000000007472696167653a3101000000"
        );
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
    fn rejected_sample_then_draw_position_overflow_preserves_owner() {
        let mut map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let key = map
            .key_for("reject-4", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        let mut owner = map
            .stream_for("reject-4", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        let mut control = map
            .stream_for("reject-4", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        owner.draw_position = u64::MAX - 1;
        control.draw_position = u64::MAX - 1;
        let state_before = owner.stream.clone().into_inner();
        let distribution = crate::work_duration::IntrinsicDurationDistribution::weighted_ticks(
            vec![(1, 0x8000_0000_0000_0001)],
        )
        .unwrap();

        assert_eq!(
            distribution.sample(&mut owner, &key),
            Err(crate::work_duration::WorkDurationError::Seed(
                CalibrationSeedError::DrawPositionOverflow
            ))
        );
        assert_eq!(owner.draw_position, u64::MAX - 1);
        assert_eq!(owner.stream.clone().into_inner(), state_before);
        assert_eq!(owner.next_u64(), control.next_u64());
        assert_eq!(owner.draw_position, u64::MAX);
    }

    #[test]
    fn snapshot_restore_continues_at_next_draw() {
        let mut subject_map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let mut control_map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let mut subject = subject_map
            .stream_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        let mut control = control_map
            .stream_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();

        let first_draw = subject.next_u64().unwrap();
        assert_eq!(first_draw, control.next_u64().unwrap());
        let snapshot = subject.snapshot();
        let mut restored = snapshot.restore().unwrap();

        assert!(restored.key() == control.key());
        assert_eq!(restored.draw_position(), control.draw_position());
        assert_eq!(restored.draw_position(), 1);

        let uninterrupted_next_u64 = control.next_u64().unwrap();
        assert_ne!(uninterrupted_next_u64, first_draw);
        assert_eq!(restored.next_u64().unwrap(), uninterrupted_next_u64);
        assert_eq!(restored.draw_position(), control.draw_position());

        assert_eq!(restored.next_u32().unwrap(), control.next_u32().unwrap());
        assert_eq!(restored.draw_position(), control.draw_position());
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

    #[test]
    fn complete_stream_state_image_resumes_without_replay() {
        let limits = CalibrationStreamStateLimits {
            max_identifier_bytes: 128,
        };
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let mut stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        let key = stream.key();
        stream.next_u32().unwrap();
        let image = stream.checkpoint_state(limits).unwrap();
        let mut restored = image.restore_for(&key, limits).unwrap();
        assert_eq!(restored.draw_position(), stream.draw_position());
        assert_eq!(restored.next_u64(), stream.next_u64());
    }

    #[test]
    fn state_image_transports_all_purposes_and_matches_long_future_prefixes() {
        let limits = CalibrationStreamStateLimits {
            max_identifier_bytes: 128,
        };
        let purposes = [
            SeedPurpose::Service,
            SeedPurpose::Transit,
            SeedPurpose::Behavior,
            SeedPurpose::Calibration,
        ];
        for (purpose_index, purpose) in purposes.into_iter().enumerate() {
            let mut subject_map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
            let mut control_map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
            let mut subject = subject_map
                .stream_for("crn-v1", 7, "case-0001", "triage:1", purpose)
                .unwrap();
            let mut control = control_map
                .stream_for("crn-v1", 7, "case-0001", "triage:1", purpose)
                .unwrap();
            let key = subject.key();
            for prefix in 0..purpose_index {
                if prefix % 2 == 0 {
                    assert_eq!(subject.next_u32(), control.next_u32());
                } else {
                    assert_eq!(subject.next_u64(), control.next_u64());
                }
            }
            let image = subject.checkpoint_state(limits).unwrap();
            // Rebuild the owned DTO field by field to exercise state transport.
            let transported = CalibrationStreamStateV1 {
                version: image.version,
                identity: image.identity.clone(),
                seed_map_version: image.seed_map_version,
                stream_version: image.stream_version,
                derived_seed: image.derived_seed,
                current_state: image.current_state,
                draw_position: image.draw_position,
            };
            assert_eq!(transported, image);
            let mut restored = transported.restore_for(&key, limits).unwrap();
            assert_eq!(restored.draw_position(), control.draw_position());
            for draw in 0..64 {
                if draw % 3 == 0 {
                    assert_eq!(restored.next_u32(), control.next_u32());
                } else {
                    assert_eq!(restored.next_u64(), control.next_u64());
                }
                assert_eq!(restored.draw_position(), control.draw_position());
            }
        }
    }

    #[test]
    fn state_image_rejects_identity_purpose_schema_seed_and_zero_state_changes() {
        let limits = CalibrationStreamStateLimits {
            max_identifier_bytes: 128,
        };
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        let key = stream.key();
        let image = stream.checkpoint_state(limits).unwrap();

        let mut other_map = CalibrationSeedMap::new(1, "other-study", 0).unwrap();
        let other_key = other_map
            .key_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        assert!(matches!(
            image.clone().restore_for(&other_key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        let mut other_map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let wrong_purpose = other_map
            .key_for("schedule", 0, "case", "task", SeedPurpose::Transit)
            .unwrap();
        assert!(matches!(
            image.clone().restore_for(&wrong_purpose, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        for alter in 0..5 {
            let mut identity = image.identity.clone();
            match alter {
                0 => identity.root_seed ^= 1,
                1 => identity.replication_id += 1,
                2 => identity.seed_schedule_id.push_str("-other"),
                3 => identity.case_key.push_str("-other"),
                _ => identity.task_key.push_str("-other"),
            }
            let mismatched = CalibrationStreamKey { identity };
            assert!(matches!(
                image.clone().restore_for(&mismatched, limits),
                Err(CalibrationStreamStateError::InvalidState)
            ));
        }

        let mut bad = image.clone();
        bad.version = 2;
        assert!(matches!(
            bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        let mut bad = image.clone();
        bad.seed_map_version = 2;
        assert!(matches!(
            bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        let mut bad = image.clone();
        bad.identity.version = 2;
        assert!(matches!(
            bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        let mut bad = image.clone();
        bad.stream_version = 2;
        assert!(matches!(
            bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        let mut bad = image.clone();
        bad.derived_seed ^= 1;
        bad.draw_position = 1;
        assert!(matches!(
            bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::Seed(
                CalibrationSeedError::InvalidSnapshot
            ))
        ));
        let mut bad = image;
        bad.current_state ^= 1;
        assert!(matches!(
            bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
    }

    #[test]
    fn state_image_limits_and_source_preservation_hold_on_rejection() {
        let limits = CalibrationStreamStateLimits {
            max_identifier_bytes: 128,
        };
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        let before = stream.checkpoint_state(limits).unwrap();
        let tiny = CalibrationStreamStateLimits {
            max_identifier_bytes: 1,
        };
        assert_eq!(
            stream.checkpoint_state(tiny),
            Err(CalibrationStreamStateError::LimitExceeded)
        );
        assert_eq!(stream.checkpoint_state(limits).unwrap(), before);
        let key = stream.key();
        assert!(matches!(
            before.clone().restore_for(&key, tiny),
            Err(CalibrationStreamStateError::LimitExceeded)
        ));
        assert_eq!(stream.checkpoint_state(limits).unwrap(), before);

        let mut zero_draw_bad = before.clone();
        zero_draw_bad.current_state ^= 1;
        assert!(matches!(
            zero_draw_bad.restore_for(&key, limits),
            Err(CalibrationStreamStateError::InvalidState)
        ));
        assert_eq!(stream.checkpoint_state(limits).unwrap(), before);
    }
}
