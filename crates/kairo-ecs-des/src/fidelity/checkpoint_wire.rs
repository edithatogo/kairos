//! Bounded, explicit little-endian transport for the experimental fidelity image.

use super::*;
use crate::FlowCheckpointRebindV1;
use std::error::Error;
use std::fmt::{Display, Formatter};

const MAGIC: &[u8; 8] = b"KFIDW1\0\0";
const WIRE_SCHEMA: u16 = 1;

/// Limits for one portable fidelity checkpoint image.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidelityCheckpointWireLimits {
    pub checkpoint: FidelityCheckpointLimits,
    pub max_wire_bytes: usize,
    pub max_total_records: usize,
}

impl Default for FidelityCheckpointWireLimits {
    fn default() -> Self {
        Self {
            checkpoint: FidelityCheckpointLimits {
                max_admitted: 1_000_000,
                max_overrides: 1_000_000,
                max_subsystem_bytes: 64 * 1024 * 1024,
            },
            max_wire_bytes: 512 * 1024 * 1024,
            max_total_records: 2_000_000,
        }
    }
}

/// Malformed, unsupported, or over-budget fidelity wire input.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FidelityCheckpointWireError {
    UnsupportedSchema(u16),
    UnsupportedVersion(u32),
    InvalidTag,
    InvalidBoolean,
    InvalidUtf8,
    IntegerOverflow,
    Truncated,
    TrailingBytes,
    LimitExceeded(&'static str),
    AllocationFailed,
    NonCanonical,
    InvalidPolicy,
    InvalidDecision,
    InvalidBinding,
    InvalidWorkReference,
}

impl Display for FidelityCheckpointWireError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported fidelity wire schema: {version}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported fidelity checkpoint version: {version}")
            }
            Self::InvalidTag => f.write_str("invalid fidelity wire variant tag"),
            Self::InvalidBoolean => f.write_str("invalid fidelity wire boolean"),
            Self::InvalidUtf8 => f.write_str("invalid fidelity wire UTF-8"),
            Self::IntegerOverflow => f.write_str("fidelity wire integer overflow"),
            Self::Truncated => f.write_str("truncated fidelity wire image"),
            Self::TrailingBytes => f.write_str("trailing fidelity wire bytes"),
            Self::LimitExceeded(name) => write!(f, "fidelity wire {name} limit exceeded"),
            Self::AllocationFailed => f.write_str("fidelity wire allocation failed"),
            Self::NonCanonical => f.write_str("noncanonical fidelity wire records"),
            Self::InvalidPolicy => f.write_str("invalid fidelity policy value"),
            Self::InvalidDecision => f.write_str("invalid frozen fidelity decision"),
            Self::InvalidBinding => f.write_str("invalid fidelity runtime binding flag"),
            Self::InvalidWorkReference => f.write_str("unknown fidelity work reference"),
        }
    }
}

impl Error for FidelityCheckpointWireError {}

type WResult<T = ()> = Result<T, FidelityCheckpointWireError>;

fn validate_checkpoint(
    image: &FidelityAdapterCheckpointV1,
    limits: FidelityCheckpointWireLimits,
) -> WResult {
    if image.version != 1 {
        return Err(FidelityCheckpointWireError::UnsupportedVersion(
            image.version,
        ));
    }
    if image.admitted.len() > limits.checkpoint.max_admitted {
        return Err(FidelityCheckpointWireError::LimitExceeded("admitted"));
    }
    let mut overrides = 0_usize;
    let mut subsystem_bytes = 0_usize;
    for policy in std::iter::once(&image.current).chain(image.pending.iter()) {
        if policy.version != 1 {
            return Err(FidelityCheckpointWireError::UnsupportedVersion(
                policy.version,
            ));
        }
        overrides = overrides
            .checked_add(policy.entity_overrides.len())
            .and_then(|count| count.checked_add(policy.subsystem_overrides.len()))
            .and_then(|count| count.checked_add(policy.entity_subsystem_overrides.len()))
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        for (subsystem, _) in &policy.subsystem_overrides {
            subsystem_bytes = subsystem_bytes
                .checked_add(subsystem.len())
                .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        }
        for (_, subsystem, _) in &policy.entity_subsystem_overrides {
            subsystem_bytes = subsystem_bytes
                .checked_add(subsystem.len())
                .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        }
    }
    if overrides > limits.checkpoint.max_overrides {
        return Err(FidelityCheckpointWireError::LimitExceeded("override"));
    }
    if subsystem_bytes > limits.checkpoint.max_subsystem_bytes {
        return Err(FidelityCheckpointWireError::LimitExceeded("subsystem byte"));
    }
    validate_policy_checkpoint(&image.current).map_err(map_checkpoint_error)?;
    if let Some(policy) = image.pending.as_ref() {
        validate_policy_checkpoint(policy).map_err(map_checkpoint_error)?;
    }
    if image.admitted.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(FidelityCheckpointWireError::NonCanonical);
    }
    if image
        .admitted
        .iter()
        .any(|(_, decision)| decision.policy_version != 1)
    {
        return Err(FidelityCheckpointWireError::InvalidDecision);
    }
    if image.bound_runtime != !image.admitted.is_empty() {
        return Err(FidelityCheckpointWireError::InvalidBinding);
    }
    Ok(())
}

fn map_checkpoint_error(error: FidelityCheckpointError) -> FidelityCheckpointWireError {
    match error {
        FidelityCheckpointError::UnsupportedVersion(version) => {
            FidelityCheckpointWireError::UnsupportedVersion(version)
        }
        FidelityCheckpointError::LimitExceeded => {
            FidelityCheckpointWireError::LimitExceeded("checkpoint")
        }
        FidelityCheckpointError::NonCanonical => FidelityCheckpointWireError::NonCanonical,
        FidelityCheckpointError::InvalidDecision => FidelityCheckpointWireError::InvalidDecision,
        FidelityCheckpointError::InvalidPolicy => FidelityCheckpointWireError::InvalidPolicy,
        FidelityCheckpointError::WrongRuntime
        | FidelityCheckpointError::InvalidWork
        | FidelityCheckpointError::MissingMapping
        | FidelityCheckpointError::DuplicateMapping
        | FidelityCheckpointError::UnexpectedMapping => {
            FidelityCheckpointWireError::InvalidWorkReference
        }
    }
}

struct Writer {
    bytes: Option<Vec<u8>>,
    len: usize,
    expected_len: usize,
    limits: FidelityCheckpointWireLimits,
    records: usize,
    subsystem_bytes: usize,
}

impl Writer {
    fn measure(limits: FidelityCheckpointWireLimits) -> Self {
        Self {
            bytes: None,
            len: 0,
            expected_len: 0,
            limits,
            records: 0,
            subsystem_bytes: 0,
        }
    }

    fn writing(limits: FidelityCheckpointWireLimits, size: usize) -> WResult<Self> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| FidelityCheckpointWireError::AllocationFailed)?;
        Ok(Self {
            bytes: Some(bytes),
            len: 0,
            expected_len: size,
            limits,
            records: 0,
            subsystem_bytes: 0,
        })
    }

    fn raw(&mut self, value: &[u8]) -> WResult {
        let next = self
            .len
            .checked_add(value.len())
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if next > self.limits.max_wire_bytes {
            return Err(FidelityCheckpointWireError::LimitExceeded("wire byte"));
        }
        if self.bytes.is_some() && next > self.expected_len {
            return Err(FidelityCheckpointWireError::InvalidPolicy);
        }
        if let Some(bytes) = &mut self.bytes {
            bytes.extend_from_slice(value);
        }
        self.len = next;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> WResult {
        self.raw(&[value])
    }

    fn u16(&mut self, value: u16) -> WResult {
        self.raw(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> WResult {
        self.raw(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> WResult {
        self.raw(&value.to_le_bytes())
    }

    fn usize(&mut self, value: usize) -> WResult {
        self.u64(u64::try_from(value).map_err(|_| FidelityCheckpointWireError::IntegerOverflow)?)
    }

    fn entity(&mut self, value: EntityId) -> WResult {
        self.u64(value.index)?;
        self.u32(value.generation)
    }

    fn count(&mut self, value: usize) -> WResult {
        self.records = self
            .records
            .checked_add(value)
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if self.records > self.limits.max_total_records {
            return Err(FidelityCheckpointWireError::LimitExceeded(
                "aggregate record",
            ));
        }
        self.usize(value)
    }

    fn subsystem(&mut self, value: &str) -> WResult {
        self.subsystem_bytes = self
            .subsystem_bytes
            .checked_add(value.len())
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if self.subsystem_bytes > self.limits.checkpoint.max_subsystem_bytes {
            return Err(FidelityCheckpointWireError::LimitExceeded("subsystem byte"));
        }
        self.usize(value.len())?;
        self.raw(value.as_bytes())
    }

    fn finish(self) -> WResult<Vec<u8>> {
        if self.len != self.expected_len {
            return Err(FidelityCheckpointWireError::InvalidPolicy);
        }
        Ok(self.bytes.expect("writer owns output buffer"))
    }
}

fn put_mode(writer: &mut Writer, mode: FidelityMode) -> WResult {
    writer.u8(match mode {
        FidelityMode::Macro => 0,
        FidelityMode::Micro => 1,
    })
}

fn put_scope(writer: &mut Writer, scope: FidelityScope) -> WResult {
    writer.u8(match scope {
        FidelityScope::EntitySubsystem => 0,
        FidelityScope::Entity => 1,
        FidelityScope::Subsystem => 2,
        FidelityScope::Global => 3,
    })
}

fn put_policy(writer: &mut Writer, policy: &FidelityPolicyCheckpointV1) -> WResult {
    writer.u32(policy.version)?;
    match policy.global {
        None => writer.u8(0)?,
        Some(mode) => {
            writer.u8(1)?;
            put_mode(writer, mode)?;
        }
    }
    writer.count(policy.entity_overrides.len())?;
    for (entity, mode) in &policy.entity_overrides {
        writer.entity(*entity)?;
        put_mode(writer, *mode)?;
    }
    writer.count(policy.subsystem_overrides.len())?;
    for (subsystem, mode) in &policy.subsystem_overrides {
        writer.subsystem(subsystem)?;
        put_mode(writer, *mode)?;
    }
    writer.count(policy.entity_subsystem_overrides.len())?;
    for (entity, subsystem, mode) in &policy.entity_subsystem_overrides {
        writer.entity(*entity)?;
        writer.subsystem(subsystem)?;
        put_mode(writer, *mode)?;
    }
    Ok(())
}

fn encode(image: &FidelityAdapterCheckpointV1, writer: &mut Writer) -> WResult {
    writer.raw(MAGIC)?;
    writer.u16(WIRE_SCHEMA)?;
    writer.u32(image.version)?;
    put_policy(writer, &image.current)?;
    match image.pending.as_ref() {
        Some(policy) => {
            writer.u8(1)?;
            put_policy(writer, policy)?;
        }
        None => writer.u8(0)?,
    }
    writer.count(image.admitted.len())?;
    for (work, decision) in &image.admitted {
        writer.entity(work.entity_id())?;
        put_mode(writer, decision.mode)?;
        put_scope(writer, decision.scope)?;
        writer.u32(decision.policy_version)?;
    }
    writer.u8(u8::from(image.bound_runtime))
}

impl FidelityAdapterCheckpointV1 {
    /// Encode this complete fidelity image as bounded little-endian data.
    #[doc(hidden)]
    pub fn encode_wire_v1(
        &self,
        limits: FidelityCheckpointWireLimits,
    ) -> Result<Vec<u8>, FidelityCheckpointWireError> {
        validate_checkpoint(self, limits)?;
        let mut measure = Writer::measure(limits);
        encode(self, &mut measure)?;
        let size = measure.len;
        let mut writer = Writer::writing(limits, size)?;
        encode(self, &mut writer)?;
        writer.finish()
    }

    /// Decode the image only after a bounded, allocation-free validation pass.
    /// Work IDs are resolved through the caller's validated Flow view; policy
    /// overrides intentionally remain opaque future configuration. Flow restore
    /// preserves entity IDs, so pair each decoded admitted work ID with itself
    /// when passing this value to `FidelityAdapter::from_checkpoint`.
    #[doc(hidden)]
    pub fn decode_wire_v1(
        bytes: &[u8],
        view: &FlowCheckpointRebindV1,
        limits: FidelityCheckpointWireLimits,
    ) -> Result<Self, FidelityCheckpointWireError> {
        if bytes.len() > limits.max_wire_bytes {
            return Err(FidelityCheckpointWireError::LimitExceeded("wire byte"));
        }
        let mut preflight = Reader::new(bytes, view, limits, false)?;
        let _ = preflight.image()?;
        preflight.finish()?;
        let mut reader = Reader::new(bytes, view, limits, true)?;
        let image = reader.image()?;
        reader.finish()?;
        Ok(image)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    view: &'a FlowCheckpointRebindV1,
    limits: FidelityCheckpointWireLimits,
    collect: bool,
    records: usize,
    overrides: usize,
    admitted: usize,
    subsystem_bytes: usize,
}

impl<'a> Reader<'a> {
    fn new(
        bytes: &'a [u8],
        view: &'a FlowCheckpointRebindV1,
        limits: FidelityCheckpointWireLimits,
        collect: bool,
    ) -> WResult<Self> {
        if bytes.len() > limits.max_wire_bytes {
            return Err(FidelityCheckpointWireError::LimitExceeded("wire byte"));
        }
        Ok(Self {
            bytes,
            at: 0,
            view,
            limits,
            collect,
            records: 0,
            overrides: 0,
            admitted: 0,
            subsystem_bytes: 0,
        })
    }

    fn raw(&mut self, size: usize) -> WResult<&'a [u8]> {
        let end = self
            .at
            .checked_add(size)
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or(FidelityCheckpointWireError::Truncated)?;
        self.at = end;
        Ok(value)
    }

    fn u8(&mut self) -> WResult<u8> {
        Ok(self.raw(1)?[0])
    }

    fn boolean(&mut self) -> WResult<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(FidelityCheckpointWireError::InvalidBoolean),
        }
    }

    fn u16(&mut self) -> WResult<u16> {
        Ok(u16::from_le_bytes(self.raw(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> WResult<u32> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> WResult<u64> {
        Ok(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
    }

    fn usize(&mut self) -> WResult<usize> {
        usize::try_from(self.u64()?).map_err(|_| FidelityCheckpointWireError::IntegerOverflow)
    }

    fn entity(&mut self) -> WResult<EntityId> {
        Ok(EntityId::new(self.u64()?, self.u32()?))
    }

    fn count(&mut self, cap: usize, label: &'static str) -> WResult<usize> {
        let count = self.usize()?;
        self.records = self
            .records
            .checked_add(count)
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if count > cap || self.records > self.limits.max_total_records {
            return Err(FidelityCheckpointWireError::LimitExceeded(label));
        }
        Ok(count)
    }

    fn override_count(&mut self) -> WResult<usize> {
        let count = self.count(self.limits.checkpoint.max_overrides, "override")?;
        self.overrides = self
            .overrides
            .checked_add(count)
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if self.overrides > self.limits.checkpoint.max_overrides {
            return Err(FidelityCheckpointWireError::LimitExceeded("override"));
        }
        Ok(count)
    }

    fn admitted_count(&mut self) -> WResult<usize> {
        let count = self.count(self.limits.checkpoint.max_admitted, "admitted")?;
        self.admitted = self
            .admitted
            .checked_add(count)
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if self.admitted > self.limits.checkpoint.max_admitted {
            return Err(FidelityCheckpointWireError::LimitExceeded("admitted"));
        }
        Ok(count)
    }

    fn mode(&mut self) -> WResult<FidelityMode> {
        match self.u8()? {
            0 => Ok(FidelityMode::Macro),
            1 => Ok(FidelityMode::Micro),
            _ => Err(FidelityCheckpointWireError::InvalidTag),
        }
    }

    fn scope(&mut self) -> WResult<FidelityScope> {
        match self.u8()? {
            0 => Ok(FidelityScope::EntitySubsystem),
            1 => Ok(FidelityScope::Entity),
            2 => Ok(FidelityScope::Subsystem),
            3 => Ok(FidelityScope::Global),
            _ => Err(FidelityCheckpointWireError::InvalidTag),
        }
    }

    fn subsystem(&mut self) -> WResult<&'a str> {
        let length = self.usize()?;
        self.subsystem_bytes = self
            .subsystem_bytes
            .checked_add(length)
            .ok_or(FidelityCheckpointWireError::IntegerOverflow)?;
        if self.subsystem_bytes > self.limits.checkpoint.max_subsystem_bytes {
            return Err(FidelityCheckpointWireError::LimitExceeded("subsystem byte"));
        }
        let bytes = self.raw(length)?;
        let value =
            std::str::from_utf8(bytes).map_err(|_| FidelityCheckpointWireError::InvalidUtf8)?;
        validate_subsystem(value).map_err(|_| FidelityCheckpointWireError::InvalidPolicy)?;
        Ok(value)
    }

    fn store_subsystem(value: &str) -> WResult<String> {
        let mut result = String::new();
        result
            .try_reserve_exact(value.len())
            .map_err(|_| FidelityCheckpointWireError::AllocationFailed)?;
        result.push_str(value);
        Ok(result)
    }

    fn policy(&mut self) -> WResult<FidelityPolicyCheckpointV1> {
        let version = self.u32()?;
        if version != 1 {
            return Err(FidelityCheckpointWireError::UnsupportedVersion(version));
        }
        let global = match self.u8()? {
            0 => None,
            1 => Some(self.mode()?),
            _ => return Err(FidelityCheckpointWireError::InvalidTag),
        };

        let entity_count = self.override_count()?;
        if entity_count > self.bytes.len().saturating_sub(self.at) / 13 {
            return Err(FidelityCheckpointWireError::Truncated);
        }
        let mut entity_overrides = Vec::new();
        if self.collect {
            entity_overrides
                .try_reserve_exact(entity_count)
                .map_err(|_| FidelityCheckpointWireError::AllocationFailed)?;
        }
        let mut previous_entity = None;
        for _ in 0..entity_count {
            let entity = self.entity()?;
            if previous_entity.is_some_and(|previous| previous >= entity) {
                return Err(FidelityCheckpointWireError::NonCanonical);
            }
            previous_entity = Some(entity);
            let mode = self.mode()?;
            if self.collect {
                entity_overrides.push((entity, mode));
            }
        }

        let subsystem_count = self.override_count()?;
        if subsystem_count > self.bytes.len().saturating_sub(self.at) / 10 {
            return Err(FidelityCheckpointWireError::Truncated);
        }
        let mut subsystem_overrides = Vec::new();
        if self.collect {
            subsystem_overrides
                .try_reserve_exact(subsystem_count)
                .map_err(|_| FidelityCheckpointWireError::AllocationFailed)?;
        }
        let mut previous_subsystem: Option<&'a str> = None;
        for _ in 0..subsystem_count {
            let subsystem = self.subsystem()?;
            if previous_subsystem.is_some_and(|previous| previous >= subsystem) {
                return Err(FidelityCheckpointWireError::NonCanonical);
            }
            let mode = self.mode()?;
            if self.collect {
                let stored = Self::store_subsystem(subsystem)?;
                previous_subsystem = Some(subsystem);
                subsystem_overrides.push((stored, mode));
            } else {
                previous_subsystem = Some(subsystem);
            }
        }

        let pair_count = self.override_count()?;
        if pair_count > self.bytes.len().saturating_sub(self.at) / 22 {
            return Err(FidelityCheckpointWireError::Truncated);
        }
        let mut entity_subsystem_overrides = Vec::new();
        if self.collect {
            entity_subsystem_overrides
                .try_reserve_exact(pair_count)
                .map_err(|_| FidelityCheckpointWireError::AllocationFailed)?;
        }
        let mut previous_pair: Option<(EntityId, &'a str)> = None;
        for _ in 0..pair_count {
            let entity = self.entity()?;
            let subsystem = self.subsystem()?;
            if previous_pair
                .as_ref()
                .is_some_and(|(previous_entity, previous_subsystem)| {
                    (*previous_entity, *previous_subsystem) >= (entity, subsystem)
                })
            {
                return Err(FidelityCheckpointWireError::NonCanonical);
            }
            let mode = self.mode()?;
            if self.collect {
                let stored = Self::store_subsystem(subsystem)?;
                previous_pair = Some((entity, subsystem));
                entity_subsystem_overrides.push((entity, stored, mode));
            } else {
                previous_pair = Some((entity, subsystem));
            }
        }
        Ok(FidelityPolicyCheckpointV1 {
            version,
            global,
            entity_overrides,
            subsystem_overrides,
            entity_subsystem_overrides,
        })
    }

    fn image(&mut self) -> WResult<FidelityAdapterCheckpointV1> {
        if self.raw(MAGIC.len())? != MAGIC {
            return Err(FidelityCheckpointWireError::InvalidTag);
        }
        let schema = self.u16()?;
        if schema != WIRE_SCHEMA {
            return Err(FidelityCheckpointWireError::UnsupportedSchema(schema));
        }
        let version = self.u32()?;
        if version != 1 {
            return Err(FidelityCheckpointWireError::UnsupportedVersion(version));
        }
        let current = self.policy()?;
        let pending = match self.u8()? {
            0 => None,
            1 => Some(self.policy()?),
            _ => return Err(FidelityCheckpointWireError::InvalidTag),
        };
        let admitted_count = self.admitted_count()?;
        if admitted_count > self.bytes.len().saturating_sub(self.at) / 18 {
            return Err(FidelityCheckpointWireError::Truncated);
        }
        let mut admitted = Vec::new();
        if self.collect {
            admitted
                .try_reserve_exact(admitted_count)
                .map_err(|_| FidelityCheckpointWireError::AllocationFailed)?;
        }
        let mut previous = None;
        for _ in 0..admitted_count {
            let entity = self.entity()?;
            let work = self
                .view
                .resolve_work(entity)
                .map_err(|_| FidelityCheckpointWireError::InvalidWorkReference)?;
            if previous.is_some_and(|old| old >= work) {
                return Err(FidelityCheckpointWireError::NonCanonical);
            }
            previous = Some(work);
            let decision = FidelityDecision {
                mode: self.mode()?,
                scope: self.scope()?,
                policy_version: self.u32()?,
            };
            if decision.policy_version != 1 {
                return Err(FidelityCheckpointWireError::InvalidDecision);
            }
            if self.collect {
                admitted.push((work, decision));
            }
        }
        let bound_runtime = self.boolean()?;
        if bound_runtime != (admitted_count != 0) {
            return Err(FidelityCheckpointWireError::InvalidBinding);
        }
        Ok(FidelityAdapterCheckpointV1 {
            version,
            current,
            pending,
            admitted,
            bound_runtime,
        })
    }

    fn finish(self) -> WResult {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(FidelityCheckpointWireError::TrailingBytes)
        }
    }
}
