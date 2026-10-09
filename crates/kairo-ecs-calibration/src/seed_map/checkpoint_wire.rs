//! Bounded private little-endian seed-owner wire format.
#![cfg_attr(
    not(test),
    allow(dead_code, reason = "Consumed by the C2 durable owner assembler")
)]
use super::{
    validate_id, CalibrationSeedMap, CalibrationStream, CalibrationStreamKey,
    CalibrationStreamStateError, CalibrationStreamStateLimits, CalibrationStreamStateV1,
    SeedIdentity, SeedMapCheckpointEntryV1, SeedMapCheckpointStateV1, SeedPurpose,
    SeedRegistryCheckpointError, SeedRegistryCheckpointLimits, SEED_MAP_VERSION_V1,
    STREAM_VERSION_V1,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const WIRE_VERSION: u8 = 1;
const MAP_MAGIC: &[u8; 4] = b"KSMW";
const STREAM_MAGIC: &[u8; 4] = b"KSTW";
const KEY_MAGIC: &[u8; 4] = b"KSKW";
const HEADER: usize = 5;
const ID_FIXED: usize = 37;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SeedWireLimits {
    pub(crate) max_entries: usize,
    pub(crate) max_identifier_bytes: usize,
    pub(crate) max_wire_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum SeedWireError {
    #[error("unsupported seed wire schema {0}")]
    UnsupportedSchema(u8),
    #[error("invalid seed wire image")]
    InvalidFormat,
    #[error("seed wire limit exceeded")]
    LimitExceeded,
    #[error("seed wire allocation failed")]
    AllocationFailed,
    #[error(transparent)]
    Registry(#[from] SeedRegistryCheckpointError),
    #[error(transparent)]
    Stream(#[from] CalibrationStreamStateError),
}

#[derive(Clone, Copy)]
struct IdRef<'a> {
    version: u32,
    root: u64,
    replication: u64,
    study: &'a str,
    schedule: &'a str,
    case_key: &'a str,
    task: &'a str,
    purpose: SeedPurpose,
}

impl IdRef<'_> {
    fn ids(self) -> Result<usize, SeedWireError> {
        let mut n = 0usize;
        for (name, value) in [
            ("study_id", self.study),
            ("seed_schedule_id", self.schedule),
            ("case_key", self.case_key),
            ("task_key", self.task),
        ] {
            validate_id(name, value).map_err(|_| SeedWireError::InvalidFormat)?;
            n = n
                .checked_add(value.len())
                .ok_or(SeedWireError::LimitExceeded)?;
        }
        Ok(n)
    }

    fn derived(self) -> Result<u64, SeedWireError> {
        let mut h = Sha256::new();
        h.update(b"kairos.calibration.seed-map");
        h.update(self.version.to_le_bytes());
        h.update(self.root.to_le_bytes());
        h.update(self.replication.to_le_bytes());
        for value in [self.study, self.schedule, self.case_key, self.task] {
            let n = u64::try_from(value.len()).map_err(|_| SeedWireError::LimitExceeded)?;
            h.update(n.to_le_bytes());
            h.update(value.as_bytes());
        }
        h.update((self.purpose as u32).to_le_bytes());
        let digest = h.finalize();
        Ok(u64::from_le_bytes(
            digest[..8]
                .try_into()
                .map_err(|_| SeedWireError::InvalidFormat)?,
        ))
    }
}

fn idref(id: &SeedIdentity) -> IdRef<'_> {
    IdRef {
        version: id.version,
        root: id.root_seed,
        replication: id.replication_id,
        study: &id.study_id,
        schedule: &id.seed_schedule_id,
        case_key: &id.case_key,
        task: &id.task_key,
        purpose: id.purpose,
    }
}

fn add(total: usize, n: usize, cap: usize) -> Result<usize, SeedWireError> {
    let next = total.checked_add(n).ok_or(SeedWireError::LimitExceeded)?;
    if next > cap {
        return Err(SeedWireError::LimitExceeded);
    }
    Ok(next)
}
fn id_len(id: IdRef<'_>) -> Result<(usize, usize), SeedWireError> {
    let ids = id.ids()?;
    Ok((
        ID_FIXED
            .checked_add(ids)
            .ok_or(SeedWireError::LimitExceeded)?,
        ids,
    ))
}

fn put_u32(out: &mut Vec<u8>, x: u32) {
    out.extend_from_slice(&x.to_le_bytes());
}
fn put_u64(out: &mut Vec<u8>, x: u64) {
    out.extend_from_slice(&x.to_le_bytes());
}
fn put_str(out: &mut Vec<u8>, x: &str) -> Result<(), SeedWireError> {
    put_u32(
        out,
        u32::try_from(x.len()).map_err(|_| SeedWireError::LimitExceeded)?,
    );
    out.extend_from_slice(x.as_bytes());
    Ok(())
}
fn put_id(out: &mut Vec<u8>, id: &SeedIdentity) -> Result<(), SeedWireError> {
    put_u32(out, id.version);
    put_u64(out, id.root_seed);
    put_u64(out, id.replication_id);
    put_str(out, &id.study_id)?;
    put_str(out, &id.seed_schedule_id)?;
    put_str(out, &id.case_key)?;
    put_str(out, &id.task_key)?;
    out.push(id.purpose as u8);
    Ok(())
}
fn start(out: &mut Vec<u8>, magic: &[u8; 4]) {
    out.extend_from_slice(magic);
    out.push(WIRE_VERSION);
}

/// Preflight source limits before native owner cloning and output allocation.
pub(crate) fn encode_seed_map(
    map: &CalibrationSeedMap,
    limits: SeedWireLimits,
) -> Result<Vec<u8>, SeedWireError> {
    if map.registered.len() > limits.max_entries {
        return Err(SeedWireError::LimitExceeded);
    }
    validate_id("study_id", &map.study_id).map_err(|_| SeedWireError::InvalidFormat)?;
    let mut ids = map.study_id.len();
    let mut size = HEADER + 4 + 8 + 4 + map.study_id.len() + 4;
    for (&seed, id) in &map.registered {
        let r = idref(id);
        if r.version != map.version
            || r.root != map.root_seed
            || r.study != map.study_id
            || r.derived()? != seed
        {
            return Err(SeedWireError::InvalidFormat);
        }
        let (n, idbytes) = id_len(r)?;
        ids = add(ids, idbytes, limits.max_identifier_bytes)?;
        size = add(size, 8, limits.max_wire_bytes)?;
        size = add(size, n, limits.max_wire_bytes)?;
    }
    if ids > limits.max_identifier_bytes || size > limits.max_wire_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let state = map.checkpoint_state(SeedRegistryCheckpointLimits {
        max_entries: limits.max_entries,
        max_identifier_bytes: limits.max_identifier_bytes,
    })?;
    encode_map_state(&state, limits)
}

pub(crate) fn encode_map_state(
    state: &SeedMapCheckpointStateV1,
    limits: SeedWireLimits,
) -> Result<Vec<u8>, SeedWireError> {
    if state.entries.len() > limits.max_entries {
        return Err(SeedWireError::LimitExceeded);
    }
    let entry_count =
        u32::try_from(state.entries.len()).map_err(|_| SeedWireError::LimitExceeded)?;
    validate_id("study_id", &state.study_id).map_err(|_| SeedWireError::InvalidFormat)?;
    let mut ids = state.study_id.len();
    let mut size = HEADER + 4 + 8 + 4 + state.study_id.len() + 4;
    let mut prev = None;
    for e in &state.entries {
        if prev.is_some_and(|x| x >= e.seed) {
            return Err(SeedWireError::InvalidFormat);
        }
        prev = Some(e.seed);
        let r = idref(&e.identity);
        if r.version != state.version
            || r.root != state.root_seed
            || r.study != state.study_id
            || r.derived()? != e.seed
        {
            return Err(SeedWireError::InvalidFormat);
        }
        let (n, b) = id_len(r)?;
        ids = add(ids, b, limits.max_identifier_bytes)?;
        size = add(size, 8, limits.max_wire_bytes)?;
        size = add(size, n, limits.max_wire_bytes)?;
    }
    if ids > limits.max_identifier_bytes || size > limits.max_wire_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| SeedWireError::AllocationFailed)?;
    start(&mut out, MAP_MAGIC);
    put_u32(&mut out, state.version);
    put_u64(&mut out, state.root_seed);
    put_str(&mut out, &state.study_id)?;
    put_u32(&mut out, entry_count);
    for e in &state.entries {
        put_u64(&mut out, e.seed);
        put_id(&mut out, &e.identity)?;
    }
    debug_assert_eq!(out.len(), size);
    Ok(out)
}

pub(crate) fn encode_stream(
    stream: &CalibrationStream,
    limits: SeedWireLimits,
) -> Result<Vec<u8>, SeedWireError> {
    let ids = stream.checkpoint_identifier_bytes()?;
    if ids > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let total = add(HEADER + 4 + ID_FIXED + 32, ids, limits.max_wire_bytes)?;
    let state = stream.checkpoint_state(CalibrationStreamStateLimits {
        max_identifier_bytes: limits.max_identifier_bytes,
    })?;
    encode_stream_state(&state, limits, total)
}
/// Encode an already captured owner image without reconstructing or advancing
/// a temporary stream. Containing bridge checkpoints use this bounded seam.
pub(crate) fn encode_stream_image(
    state: &CalibrationStreamStateV1,
    limits: SeedWireLimits,
) -> Result<Vec<u8>, SeedWireError> {
    let id = idref(&state.identity);
    let (n, ids) = id_len(id)?;
    if ids > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let size = add(HEADER + 4 + 32, n, limits.max_wire_bytes)?;
    if state.version != 1
        || state.seed_map_version != SEED_MAP_VERSION_V1
        || state.identity.version != state.seed_map_version
        || state.stream_version != STREAM_VERSION_V1
        || id.derived()? != state.derived_seed
        || (state.draw_position == 0 && state.current_state != state.derived_seed)
    {
        return Err(SeedWireError::InvalidFormat);
    }
    encode_stream_state(state, limits, size)
}

fn encode_stream_state(
    state: &CalibrationStreamStateV1,
    limits: SeedWireLimits,
    expected: usize,
) -> Result<Vec<u8>, SeedWireError> {
    let (n, ids) = id_len(idref(&state.identity))?;
    if ids > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let size = add(HEADER + 4 + 32, n, limits.max_wire_bytes)?;
    if size != expected {
        return Err(SeedWireError::InvalidFormat);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| SeedWireError::AllocationFailed)?;
    start(&mut out, STREAM_MAGIC);
    put_u32(&mut out, state.version);
    put_id(&mut out, &state.identity)?;
    put_u32(&mut out, state.seed_map_version);
    put_u32(&mut out, state.stream_version);
    put_u64(&mut out, state.derived_seed);
    put_u64(&mut out, state.current_state);
    put_u64(&mut out, state.draw_position);
    Ok(out)
}

pub(crate) fn encode_key(
    key: &CalibrationStreamKey,
    limits: SeedWireLimits,
) -> Result<Vec<u8>, SeedWireError> {
    let (n, ids) = id_len(idref(&key.identity))?;
    if ids > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let size = add(HEADER, n, limits.max_wire_bytes)?;
    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| SeedWireError::AllocationFailed)?;
    start(&mut out, KEY_MAGIC);
    put_id(&mut out, &key.identity)?;
    Ok(out)
}

// Reader routines below first validate the whole image without allocating.
// Construction repeats parsing only after schema, ordering, UTF-8, and caps pass.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], SeedWireError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(SeedWireError::InvalidFormat)?;
        let s = self
            .bytes
            .get(self.pos..end)
            .ok_or(SeedWireError::InvalidFormat)?;
        self.pos = end;
        Ok(s)
    }
    fn header(&mut self, magic: &[u8; 4]) -> Result<(), SeedWireError> {
        if self.take(4)? != magic {
            return Err(SeedWireError::InvalidFormat);
        }
        let v = self.take(1)?[0];
        if v != WIRE_VERSION {
            return Err(SeedWireError::UnsupportedSchema(v));
        }
        Ok(())
    }
    fn u32(&mut self) -> Result<u32, SeedWireError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| SeedWireError::InvalidFormat)?,
        ))
    }
    fn count(&mut self) -> Result<usize, SeedWireError> {
        usize::try_from(self.u32()?).map_err(|_| SeedWireError::LimitExceeded)
    }
    fn u64(&mut self) -> Result<u64, SeedWireError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| SeedWireError::InvalidFormat)?,
        ))
    }
    fn string(&mut self) -> Result<&'a str, SeedWireError> {
        let len = self.count()?;
        std::str::from_utf8(self.take(len)?).map_err(|_| SeedWireError::InvalidFormat)
    }
    fn id(&mut self) -> Result<IdRef<'a>, SeedWireError> {
        let version = self.u32()?;
        let root = self.u64()?;
        let replication = self.u64()?;
        let study = self.string()?;
        let schedule = self.string()?;
        let case_key = self.string()?;
        let task = self.string()?;
        let purpose = match self.take(1)?[0] {
            1 => SeedPurpose::Service,
            2 => SeedPurpose::Transit,
            3 => SeedPurpose::Behavior,
            4 => SeedPurpose::Calibration,
            _ => return Err(SeedWireError::InvalidFormat),
        };
        let id = IdRef {
            version,
            root,
            replication,
            study,
            schedule,
            case_key,
            task,
            purpose,
        };
        id.ids()?;
        Ok(id)
    }
    fn finish(&self) -> Result<(), SeedWireError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(SeedWireError::InvalidFormat)
        }
    }
}
fn wire_cap(bytes: &[u8], limits: SeedWireLimits) -> Result<(), SeedWireError> {
    if bytes.len() > limits.max_wire_bytes {
        Err(SeedWireError::LimitExceeded)
    } else {
        Ok(())
    }
}
fn owned(s: &str) -> Result<String, SeedWireError> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| SeedWireError::AllocationFailed)?;
    out.push_str(s);
    Ok(out)
}
fn owned_id(r: IdRef<'_>) -> Result<SeedIdentity, SeedWireError> {
    Ok(SeedIdentity {
        version: r.version,
        root_seed: r.root,
        replication_id: r.replication,
        study_id: owned(r.study)?,
        seed_schedule_id: owned(r.schedule)?,
        case_key: owned(r.case_key)?,
        task_key: owned(r.task)?,
        purpose: r.purpose,
    })
}

pub(crate) fn decode_seed_map(
    bytes: &[u8],
    limits: SeedWireLimits,
) -> Result<SeedMapCheckpointStateV1, SeedWireError> {
    wire_cap(bytes, limits)?;
    let mut r = Reader::new(bytes);
    r.header(MAP_MAGIC)?;
    let version = r.u32()?;
    let root_seed = r.u64()?;
    let study = r.string()?;
    validate_id("study_id", study).map_err(|_| SeedWireError::InvalidFormat)?;
    let count = r.count()?;
    if count > limits.max_entries {
        return Err(SeedWireError::LimitExceeded);
    }
    let mut ids = study.len();
    if ids > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let mut prev = None;
    for _ in 0..count {
        let seed = r.u64()?;
        if prev.is_some_and(|x| x >= seed) {
            return Err(SeedWireError::InvalidFormat);
        }
        prev = Some(seed);
        let id = r.id()?;
        ids = add(ids, id.ids()?, limits.max_identifier_bytes)?;
        if id.version != version
            || id.root != root_seed
            || id.study != study
            || id.derived()? != seed
        {
            return Err(SeedWireError::InvalidFormat);
        }
    }
    r.finish()?;
    let mut r = Reader::new(bytes);
    r.header(MAP_MAGIC)?;
    let version = r.u32()?;
    let root_seed = r.u64()?;
    let study_id = owned(r.string()?)?;
    let count = r.count()?;
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| SeedWireError::AllocationFailed)?;
    for _ in 0..count {
        let seed = r.u64()?;
        let identity = owned_id(r.id()?)?;
        entries.push(SeedMapCheckpointEntryV1 { seed, identity });
    }
    r.finish()?;
    Ok(SeedMapCheckpointStateV1 {
        version,
        root_seed,
        study_id,
        entries,
    })
}

pub(crate) fn decode_stream(
    bytes: &[u8],
    limits: SeedWireLimits,
) -> Result<CalibrationStreamStateV1, SeedWireError> {
    wire_cap(bytes, limits)?;
    let mut r = Reader::new(bytes);
    r.header(STREAM_MAGIC)?;
    let state_version = r.u32()?;
    let id = r.id()?;
    if id.ids()? > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    let seed_map_version = r.u32()?;
    let stream_version = r.u32()?;
    let derived_seed = r.u64()?;
    let current_state = r.u64()?;
    let draw_position = r.u64()?;
    if state_version != 1
        || id.version != seed_map_version
        || seed_map_version != SEED_MAP_VERSION_V1
        || stream_version != STREAM_VERSION_V1
        || id.derived()? != derived_seed
        || (draw_position == 0 && current_state != derived_seed)
    {
        return Err(SeedWireError::InvalidFormat);
    }
    r.finish()?;
    let mut r = Reader::new(bytes);
    r.header(STREAM_MAGIC)?;
    let version = r.u32()?;
    let identity = owned_id(r.id()?)?;
    let seed_map_version = r.u32()?;
    let stream_version = r.u32()?;
    let derived_seed = r.u64()?;
    let current_state = r.u64()?;
    let draw_position = r.u64()?;
    r.finish()?;
    Ok(CalibrationStreamStateV1 {
        version,
        identity,
        seed_map_version,
        stream_version,
        derived_seed,
        current_state,
        draw_position,
    })
}

pub(crate) fn decode_key(
    bytes: &[u8],
    limits: SeedWireLimits,
) -> Result<CalibrationStreamKey, SeedWireError> {
    wire_cap(bytes, limits)?;
    let mut r = Reader::new(bytes);
    r.header(KEY_MAGIC)?;
    let id = r.id()?;
    if id.ids()? > limits.max_identifier_bytes {
        return Err(SeedWireError::LimitExceeded);
    }
    if id.version != SEED_MAP_VERSION_V1 {
        return Err(SeedWireError::InvalidFormat);
    }
    r.finish()?;
    let mut r = Reader::new(bytes);
    r.header(KEY_MAGIC)?;
    let identity = owned_id(r.id()?)?;
    r.finish()?;
    Ok(CalibrationStreamKey { identity })
}

/// Checks whether an existing opaque key is registered in this map without
/// deriving a new key, changing the registry, or constructing a stream.
pub(crate) fn contains_registered_key(
    map: &CalibrationSeedMap,
    key: &CalibrationStreamKey,
) -> Result<bool, SeedWireError> {
    let identity = idref(&key.identity);
    identity.ids()?;
    if map.version != SEED_MAP_VERSION_V1 {
        return Err(SeedWireError::Registry(SeedRegistryCheckpointError::Seed(
            super::CalibrationSeedError::UnsupportedSeedMapVersion(map.version),
        )));
    }
    if identity.version != map.version {
        return Err(SeedWireError::InvalidFormat);
    }
    if identity.root != map.root_seed || identity.study != map.study_id {
        return Ok(false);
    }
    let seed = identity.derived()?;
    Ok(map.registered.get(&seed) == Some(&key.identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_map::CalibrationSeedError;
    fn limits() -> SeedWireLimits {
        SeedWireLimits {
            max_entries: 128,
            max_identifier_bytes: 16 * 1024,
            max_wire_bytes: 64 * 1024,
        }
    }
    fn populated() -> CalibrationSeedMap {
        let mut map = CalibrationSeedMap::new(1, "study-α", 0x1234).unwrap();
        for purpose in [
            SeedPurpose::Service,
            SeedPurpose::Transit,
            SeedPurpose::Behavior,
            SeedPurpose::Calibration,
        ] {
            map.key_for("schedule", 7, "case", "task", purpose).unwrap();
        }
        map
    }

    fn registry_snapshot(map: &CalibrationSeedMap) -> SeedMapCheckpointStateV1 {
        map.checkpoint_state(SeedRegistryCheckpointLimits {
            max_entries: 128,
            max_identifier_bytes: 16 * 1024,
        })
        .unwrap()
    }

    #[test]
    fn registry_membership_is_read_only_and_checks_all_purposes_and_identity_fields() {
        let mut map = CalibrationSeedMap::new(1, "study-membership", 41).unwrap();
        let before = registry_snapshot(&map);
        for purpose in [
            SeedPurpose::Service,
            SeedPurpose::Transit,
            SeedPurpose::Behavior,
            SeedPurpose::Calibration,
        ] {
            let key = map.key_for("schedule", 3, "case", "task", purpose).unwrap();
            let before_query = registry_snapshot(&map);
            assert_eq!(contains_registered_key(&map, &key), Ok(true));
            assert_eq!(registry_snapshot(&map), before_query);
        }
        let mut absent_map = CalibrationSeedMap::new(1, "study-membership", 41).unwrap();
        let absent = absent_map
            .key_for(
                "schedule",
                3,
                "case",
                "not-registered",
                SeedPurpose::Service,
            )
            .unwrap();
        let before_absent_query = registry_snapshot(&map);
        assert_eq!(contains_registered_key(&map, &absent), Ok(false));
        assert_eq!(registry_snapshot(&map), before_absent_query);
        let after_registered = registry_snapshot(&map);
        assert_ne!(before, after_registered);

        let mut foreign_root = CalibrationSeedMap::new(1, "study-membership", 42).unwrap();
        let foreign_root_key = foreign_root
            .key_for("schedule", 3, "case", "task", SeedPurpose::Service)
            .unwrap();
        assert_eq!(contains_registered_key(&map, &foreign_root_key), Ok(false));
        assert_eq!(registry_snapshot(&map), after_registered);

        let mut foreign_study = CalibrationSeedMap::new(1, "another-study", 41).unwrap();
        let foreign_study_key = foreign_study
            .key_for("schedule", 3, "case", "task", SeedPurpose::Service)
            .unwrap();
        assert_eq!(contains_registered_key(&map, &foreign_study_key), Ok(false));
        assert_eq!(registry_snapshot(&map), after_registered);

        let mut wrong_version = absent.clone();
        wrong_version.identity.version = 2;
        assert_eq!(
            contains_registered_key(&map, &wrong_version),
            Err(SeedWireError::InvalidFormat)
        );
        let mut invalid_identity = absent.clone();
        invalid_identity.identity.task_key.clear();
        assert_eq!(
            contains_registered_key(&map, &invalid_identity),
            Err(SeedWireError::InvalidFormat)
        );
        assert_eq!(registry_snapshot(&map), after_registered);
    }

    #[test]
    fn registry_membership_compares_full_identity_not_just_seed_slot() {
        let mut map = CalibrationSeedMap::new(1, "study-collision", 99).unwrap();
        let key = map
            .key_for("schedule", 8, "case", "task", SeedPurpose::Service)
            .unwrap();
        let seed = idref(&key.identity).derived().unwrap();
        let mut conflicting_identity = key.identity.clone();
        conflicting_identity.task_key.push_str("-different");
        // Create an intentionally inconsistent internal registry fixture to
        // prove an occupied derived-seed slot alone is not membership.
        map.registered.insert(seed, conflicting_identity);
        let before = registry_snapshot(&map);

        assert_eq!(contains_registered_key(&map, &key), Ok(false));
        assert_eq!(registry_snapshot(&map), before);
    }

    #[test]
    fn registry_membership_rejects_unsupported_map_version() {
        let mut map = populated();
        let key = map
            .key_for("schedule", 7, "case", "task", SeedPurpose::Service)
            .unwrap();
        map.version = 2;
        assert_eq!(
            contains_registered_key(&map, &key),
            Err(SeedWireError::Registry(SeedRegistryCheckpointError::Seed(
                super::super::CalibrationSeedError::UnsupportedSeedMapVersion(2)
            )))
        );
    }

    #[test]
    fn captured_stream_image_encodes_exact_state_without_temporary_restore() {
        let mut map = populated();
        let mut stream = map
            .stream_for("schedule", 7, "case", "task", SeedPurpose::Service)
            .unwrap();
        stream.next_u64().unwrap();
        stream.next_u32().unwrap();
        let state = stream
            .checkpoint_state(CalibrationStreamStateLimits {
                max_identifier_bytes: limits().max_identifier_bytes,
            })
            .unwrap();
        let before = state.clone();
        let bytes = encode_stream_image(&state, limits()).unwrap();
        assert_eq!(bytes, encode_stream(&stream, limits()).unwrap());
        assert_eq!(decode_stream(&bytes, limits()).unwrap(), before);
        assert_eq!(state, before);
        let mut bad = state.clone();
        bad.derived_seed ^= 1;
        assert_eq!(
            encode_stream_image(&bad, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        assert_eq!(
            encode_stream_image(
                &state,
                SeedWireLimits {
                    max_wire_bytes: bytes.len() - 1,
                    ..limits()
                }
            ),
            Err(SeedWireError::LimitExceeded)
        );
    }

    #[test]
    fn map_stream_and_key_roundtrip_all_purposes_and_collision_registry() {
        let source = populated();
        let bytes = encode_seed_map(&source, limits()).unwrap();
        let decoded = decode_seed_map(&bytes, limits()).unwrap();
        assert!(decoded.entries.windows(2).all(|p| p[0].seed < p[1].seed));
        let mut restored = CalibrationSeedMap::from_checkpoint_state(
            decoded.clone(),
            SeedRegistryCheckpointLimits {
                max_entries: 128,
                max_identifier_bytes: 16 * 1024,
            },
        )
        .unwrap();
        assert_eq!(
            restored
                .checkpoint_state(SeedRegistryCheckpointLimits {
                    max_entries: 128,
                    max_identifier_bytes: 16 * 1024
                })
                .unwrap(),
            source
                .checkpoint_state(SeedRegistryCheckpointLimits {
                    max_entries: 128,
                    max_identifier_bytes: 16 * 1024
                })
                .unwrap()
        );
        let existing = &decoded.entries[0];
        let mut collision = existing.identity.clone();
        collision.task_key.push_str("-distinct");
        assert_eq!(
            restored.register_seed(existing.seed, collision),
            Err(CalibrationSeedError::SeedCollision {
                seed: existing.seed
            })
        );
        for purpose in [
            SeedPurpose::Service,
            SeedPurpose::Transit,
            SeedPurpose::Behavior,
            SeedPurpose::Calibration,
        ] {
            let mut sm = CalibrationSeedMap::new(1, "study-ß", 99).unwrap();
            let mut cm = CalibrationSeedMap::new(1, "study-ß", 99).unwrap();
            let mut s = sm
                .stream_for("sched", 31, "case-🐢", "task/α", purpose)
                .unwrap();
            let mut c = cm
                .stream_for("sched", 31, "case-🐢", "task/α", purpose)
                .unwrap();
            let key = s.key();
            for prefix in 0..(1 + purpose as u32 * 3) {
                if prefix % 2 == 0 {
                    assert_eq!(s.next_u32(), c.next_u32());
                } else {
                    assert_eq!(s.next_u64(), c.next_u64());
                }
            }
            let wire_key = encode_key(&key, limits()).unwrap();
            let decoded_key = decode_key(&wire_key, limits()).unwrap();
            assert_eq!(decoded_key, key);
            let tiny_key_wire = SeedWireLimits {
                max_wire_bytes: wire_key.len() - 1,
                ..limits()
            };
            assert_eq!(
                encode_key(&key, tiny_key_wire),
                Err(SeedWireError::LimitExceeded)
            );
            assert_eq!(
                decode_key(&wire_key, tiny_key_wire),
                Err(SeedWireError::LimitExceeded)
            );
            let wire = encode_stream(&s, limits()).unwrap();
            let decoded = decode_stream(&wire, limits()).unwrap();
            assert_eq!(
                decoded,
                s.checkpoint_state(CalibrationStreamStateLimits {
                    max_identifier_bytes: 16384
                })
                .unwrap()
            );
            let mut restored = decoded
                .restore_for(
                    &decoded_key,
                    CalibrationStreamStateLimits {
                        max_identifier_bytes: 16384,
                    },
                )
                .unwrap();
            for n in 0..128 {
                if n % 3 == 0 {
                    assert_eq!(restored.next_u32(), c.next_u32());
                } else {
                    assert_eq!(restored.next_u64(), c.next_u64());
                }
                assert_eq!(restored.draw_position(), c.draw_position());
            }
        }
    }

    #[test]
    fn malformed_schema_purpose_duplicate_utf8_trailing_truncated_and_caps_reject() {
        let mut map = populated();
        let valid = encode_seed_map(&map, limits()).unwrap();
        let mut bad = valid.clone();
        bad[4] = 2;
        assert_eq!(
            decode_seed_map(&bad, limits()),
            Err(SeedWireError::UnsupportedSchema(2))
        );
        let mut wrong_type = valid.clone();
        wrong_type[..4].copy_from_slice(KEY_MAGIC);
        assert_eq!(
            decode_seed_map(&wrong_type, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        assert_eq!(
            decode_seed_map(&valid[..valid.len() - 1], limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(
            decode_seed_map(&trailing, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let tiny = SeedWireLimits {
            max_wire_bytes: valid.len() - 1,
            ..limits()
        };
        assert_eq!(
            encode_seed_map(&map, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        assert_eq!(
            decode_seed_map(&valid, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        let empty = CalibrationSeedMap::new(1, "study", 5).unwrap();
        let empty_wire = encode_seed_map(&empty, limits()).unwrap();
        let tiny_ids = SeedWireLimits {
            max_identifier_bytes: 1,
            ..limits()
        };
        assert_eq!(
            decode_seed_map(&empty_wire, tiny_ids),
            Err(SeedWireError::LimitExceeded)
        );
        let tiny = SeedWireLimits {
            max_entries: 3,
            ..limits()
        };
        assert_eq!(
            encode_seed_map(&map, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        assert_eq!(map.registered.len(), 4);
        let tiny = SeedWireLimits {
            max_identifier_bytes: 1,
            ..limits()
        };
        assert_eq!(
            encode_seed_map(&map, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        assert_eq!(
            decode_seed_map(&valid, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        let mut duplicate = valid.clone();
        let mut reader = Reader::new(&valid);
        reader.header(MAP_MAGIC).unwrap();
        reader.u32().unwrap();
        reader.u64().unwrap();
        reader.string().unwrap();
        reader.count().unwrap();
        let first = reader.u64().unwrap();
        reader.id().unwrap();
        let second_offset = reader.pos;
        duplicate[second_offset..second_offset + 8].copy_from_slice(&first.to_le_bytes());
        assert_eq!(
            decode_seed_map(&duplicate, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let mut key = encode_key(
            &map.key_for("s", 0, "c", "t", SeedPurpose::Service).unwrap(),
            limits(),
        )
        .unwrap();
        *key.last_mut().unwrap() = 0;
        assert_eq!(
            decode_key(&key, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let mut key = encode_key(
            &map.key_for("s", 0, "c", "t", SeedPurpose::Service).unwrap(),
            limits(),
        )
        .unwrap();
        key[5] = 2;
        assert_eq!(
            decode_key(&key, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let mut key = encode_key(
            &map.key_for("s", 0, "c", "t", SeedPurpose::Service).unwrap(),
            limits(),
        )
        .unwrap();
        key[29] = 0xff;
        assert_eq!(
            decode_key(&key, limits()),
            Err(SeedWireError::InvalidFormat)
        );
    }

    #[test]
    fn stream_codec_preserves_zero_state_and_rejects_wrong_identity_and_state() {
        let mut map = CalibrationSeedMap::new(1, "study", 0).unwrap();
        let mut stream = map
            .stream_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        let key = stream.key();
        let zero = encode_stream(&stream, limits()).unwrap();
        let decoded = decode_stream(&zero, limits()).unwrap();
        assert!(decoded
            .restore_for(
                &key,
                CalibrationStreamStateLimits {
                    max_identifier_bytes: 16384
                }
            )
            .is_ok());
        let mut bad = zero.clone();
        bad[5] = 2;
        assert_eq!(
            decode_stream(&bad, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let mut version_reader = Reader::new(&zero);
        version_reader.header(STREAM_MAGIC).unwrap();
        version_reader.u32().unwrap();
        version_reader.id().unwrap();
        version_reader.u32().unwrap();
        let stream_version_offset = version_reader.pos;
        let mut bad = zero.clone();
        bad[stream_version_offset] = 2;
        assert_eq!(
            decode_stream(&bad, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        let mut bad = zero.clone();
        let offset = bad.len() - 16;
        bad[offset] ^= 1;
        assert_eq!(
            decode_stream(&bad, limits()),
            Err(SeedWireError::InvalidFormat)
        );
        stream.next_u64().unwrap();
        let bytes = encode_stream(&stream, limits()).unwrap();
        let tiny = SeedWireLimits {
            max_wire_bytes: bytes.len() - 1,
            ..limits()
        };
        assert_eq!(
            encode_stream(&stream, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        assert_eq!(
            decode_stream(&bytes, tiny),
            Err(SeedWireError::LimitExceeded)
        );
        let wrong = map
            .key_for("other", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        assert!(decode_stream(&bytes, limits())
            .unwrap()
            .restore_for(
                &wrong,
                CalibrationStreamStateLimits {
                    max_identifier_bytes: 16384
                }
            )
            .is_err());
    }
}
