//! Canonical bounded little-endian encoding for the experimental Flow image.
//!
//! This is a transport for the complete native DTO. It contains stable
//! caller-declared callback IDs but no function pointers or addresses, TypeIds,
//! or process-local runtime identity.

use super::*;
use kairo_ecs_core::checkpoint::{SchedulerCheckpointEntry, SchedulerCheckpointV1};
use kairo_ecs_state::checkpoint::{WorldCheckpointV1, WorldSlotCheckpointV1};
use std::num::{NonZeroU64, NonZeroUsize};

const MAGIC: &[u8; 8] = b"KFLOWV1\0";
const WIRE_SCHEMA: u16 = 1;

/// Limits for one canonical Flow wire image. The embedded Flow limits apply
/// to their corresponding aggregate collections and payload classes.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowCheckpointWireLimits {
    pub flow: FlowCheckpointLimits,
    pub max_wire_bytes: usize,
    pub max_total_records: usize,
}

impl Default for FlowCheckpointWireLimits {
    fn default() -> Self {
        Self {
            flow: FlowCheckpointLimits::default(),
            max_wire_bytes: 512 * 1024 * 1024,
            max_total_records: 32_000_000,
        }
    }
}

/// Malformed, unsupported, or over-budget canonical Flow wire input.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowCheckpointWireError {
    UnsupportedSchema(u16),
    InvalidTag,
    InvalidBoolean,
    InvalidUtf8,
    IntegerOverflow,
    Truncated,
    TrailingBytes,
    LimitExceeded(&'static str),
    AllocationFailed,
    InvalidValue(&'static str),
}

impl Display for FlowCheckpointWireError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => write!(f, "unsupported Flow wire schema: {v}"),
            Self::InvalidTag => f.write_str("invalid Flow wire variant tag"),
            Self::InvalidBoolean => f.write_str("invalid Flow wire boolean"),
            Self::InvalidUtf8 => f.write_str("invalid Flow wire UTF-8"),
            Self::IntegerOverflow => f.write_str("Flow wire integer overflow"),
            Self::Truncated => f.write_str("truncated Flow wire image"),
            Self::TrailingBytes => f.write_str("trailing Flow wire bytes"),
            Self::LimitExceeded(name) => write!(f, "Flow wire {name} limit exceeded"),
            Self::AllocationFailed => f.write_str("Flow wire allocation failed"),
            Self::InvalidValue(name) => write!(f, "invalid Flow wire {name}"),
        }
    }
}

impl Error for FlowCheckpointWireError {}

struct Writer {
    bytes: Option<Vec<u8>>,
    len: usize,
    limits: FlowCheckpointWireLimits,
    key_bytes: usize,
    payload_bytes: usize,
    records: usize,
}

impl Writer {
    fn measure(limits: FlowCheckpointWireLimits) -> Self {
        Self {
            bytes: None,
            len: 0,
            limits,
            key_bytes: 0,
            payload_bytes: 0,
            records: 0,
        }
    }

    fn writing(
        limits: FlowCheckpointWireLimits,
        cap: usize,
    ) -> Result<Self, FlowCheckpointWireError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(cap)
            .map_err(|_| FlowCheckpointWireError::AllocationFailed)?;
        Ok(Self {
            bytes: Some(bytes),
            len: 0,
            limits,
            key_bytes: 0,
            payload_bytes: 0,
            records: 0,
        })
    }

    fn raw(&mut self, value: &[u8]) -> Result<(), FlowCheckpointWireError> {
        let next = self
            .len
            .checked_add(value.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if next > self.limits.max_wire_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded("total byte"));
        }
        if let Some(bytes) = &mut self.bytes {
            bytes.extend_from_slice(value);
        }
        self.len = next;
        Ok(())
    }

    fn u8(&mut self, v: u8) -> Result<(), FlowCheckpointWireError> {
        self.raw(&[v])
    }
    fn bool(&mut self, v: bool) -> Result<(), FlowCheckpointWireError> {
        self.u8(u8::from(v))
    }
    fn u16(&mut self, v: u16) -> Result<(), FlowCheckpointWireError> {
        self.raw(&v.to_le_bytes())
    }
    fn u32(&mut self, v: u32) -> Result<(), FlowCheckpointWireError> {
        self.raw(&v.to_le_bytes())
    }
    fn i32(&mut self, v: i32) -> Result<(), FlowCheckpointWireError> {
        self.raw(&v.to_le_bytes())
    }
    fn u64(&mut self, v: u64) -> Result<(), FlowCheckpointWireError> {
        self.raw(&v.to_le_bytes())
    }
    fn u128(&mut self, v: u128) -> Result<(), FlowCheckpointWireError> {
        self.raw(&v.to_le_bytes())
    }
    fn usize(&mut self, v: usize) -> Result<(), FlowCheckpointWireError> {
        self.u64(u64::try_from(v).map_err(|_| FlowCheckpointWireError::IntegerOverflow)?)
    }
    fn count(&mut self, n: usize) -> Result<(), FlowCheckpointWireError> {
        self.records = self
            .records
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.records > self.limits.max_total_records {
            return Err(FlowCheckpointWireError::LimitExceeded("aggregate record"));
        }
        self.usize(n)
    }
    fn key(&mut self, v: &str) -> Result<(), FlowCheckpointWireError> {
        self.key_bytes = self
            .key_bytes
            .checked_add(v.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.key_bytes > self.limits.flow.max_key_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded("aggregate key byte"));
        }
        self.usize(v.len())?;
        self.raw(v.as_bytes())
    }
    fn payload(&mut self, v: &[u8]) -> Result<(), FlowCheckpointWireError> {
        self.payload_bytes = self
            .payload_bytes
            .checked_add(v.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.payload_bytes > self.limits.flow.max_payload_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate payload byte",
            ));
        }
        self.usize(v.len())?;
        self.raw(v)
    }
    fn option<T>(
        &mut self,
        value: Option<T>,
        f: impl FnOnce(&mut Self, T) -> Result<(), FlowCheckpointWireError>,
    ) -> Result<(), FlowCheckpointWireError> {
        match value {
            Some(v) => {
                self.u8(1)?;
                f(self, v)
            }
            None => self.u8(0),
        }
    }
    fn finish(self) -> Vec<u8> {
        self.bytes.expect("writing pass owns bytes")
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    limits: FlowCheckpointWireLimits,
    key_bytes: usize,
    payload_bytes: usize,
    records: usize,
    component_rows: usize,
    sparse_slots: usize,
    dispatch_admissions: usize,
    registrations: usize,
    pending_operations: usize,
}

impl<'a> Reader<'a> {
    fn new(
        bytes: &'a [u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        if bytes.len() > limits.max_wire_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded("total byte"));
        }
        Ok(Self {
            bytes,
            at: 0,
            limits,
            key_bytes: 0,
            payload_bytes: 0,
            records: 0,
            component_rows: 0,
            sparse_slots: 0,
            dispatch_admissions: 0,
            registrations: 0,
            pending_operations: 0,
        })
    }
    fn raw(&mut self, len: usize) -> Result<&'a [u8], FlowCheckpointWireError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or(FlowCheckpointWireError::Truncated)?;
        self.at = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, FlowCheckpointWireError> {
        Ok(self.raw(1)?[0])
    }
    fn bool(&mut self) -> Result<bool, FlowCheckpointWireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(FlowCheckpointWireError::InvalidBoolean),
        }
    }
    fn u16(&mut self) -> Result<u16, FlowCheckpointWireError> {
        Ok(u16::from_le_bytes(self.raw(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, FlowCheckpointWireError> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, FlowCheckpointWireError> {
        Ok(i32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, FlowCheckpointWireError> {
        Ok(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
    }
    fn u128(&mut self) -> Result<u128, FlowCheckpointWireError> {
        Ok(u128::from_le_bytes(self.raw(16)?.try_into().unwrap()))
    }
    fn usize(&mut self) -> Result<usize, FlowCheckpointWireError> {
        usize::try_from(self.u64()?).map_err(|_| FlowCheckpointWireError::IntegerOverflow)
    }
    fn count(&mut self, cap: usize, what: &'static str) -> Result<usize, FlowCheckpointWireError> {
        let n = self.usize()?;
        self.records = self
            .records
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if n > cap || self.records > self.limits.max_total_records {
            return Err(FlowCheckpointWireError::LimitExceeded(what));
        }
        Ok(n)
    }
    fn component_dimensions(
        &mut self,
        rows: usize,
        sparse: usize,
    ) -> Result<(), FlowCheckpointWireError> {
        self.component_rows = self
            .component_rows
            .checked_add(rows)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        self.sparse_slots = self
            .sparse_slots
            .checked_add(sparse)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.component_rows > self.limits.flow.max_component_rows
            || self.sparse_slots > self.limits.flow.max_sparse_slots
        {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate component",
            ));
        }
        Ok(())
    }
    fn dispatch_admissions(&mut self, n: usize) -> Result<(), FlowCheckpointWireError> {
        self.dispatch_admissions = self
            .dispatch_admissions
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.dispatch_admissions > self.limits.flow.max_commands {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "dispatch admissions",
            ));
        }
        Ok(())
    }
    fn registration_count(&mut self, n: usize) -> Result<(), FlowCheckpointWireError> {
        self.registrations = self
            .registrations
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.registrations > self.limits.flow.max_registrations {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate registrations",
            ));
        }
        Ok(())
    }
    fn pending_count(&mut self, n: usize) -> Result<(), FlowCheckpointWireError> {
        self.pending_operations = self
            .pending_operations
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.pending_operations > self.limits.flow.max_pending_operations {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate pending operation",
            ));
        }
        Ok(())
    }
    fn reserve<T>(&mut self, n: usize) -> Result<Vec<T>, FlowCheckpointWireError> {
        let mut v = Vec::new();
        v.try_reserve_exact(n)
            .map_err(|_| FlowCheckpointWireError::AllocationFailed)?;
        Ok(v)
    }
    fn require_min_items(&self, n: usize, min_bytes: usize) -> Result<(), FlowCheckpointWireError> {
        if n > self.bytes.len().saturating_sub(self.at) / min_bytes {
            Err(FlowCheckpointWireError::Truncated)
        } else {
            Ok(())
        }
    }
    fn key(&mut self) -> Result<String, FlowCheckpointWireError> {
        let n = self.usize()?;
        self.key_bytes = self
            .key_bytes
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.key_bytes > self.limits.flow.max_key_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded("aggregate key byte"));
        }
        let value = self.raw(n)?;
        let value = std::str::from_utf8(value).map_err(|_| FlowCheckpointWireError::InvalidUtf8)?;
        let mut out = String::new();
        out.try_reserve_exact(n)
            .map_err(|_| FlowCheckpointWireError::AllocationFailed)?;
        out.push_str(value);
        Ok(out)
    }
    fn payload(&mut self) -> Result<Vec<u8>, FlowCheckpointWireError> {
        let n = self.usize()?;
        self.payload_bytes = self
            .payload_bytes
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.payload_bytes > self.limits.flow.max_payload_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate payload byte",
            ));
        }
        let value = self.raw(n)?;
        let mut out = Vec::new();
        out.try_reserve_exact(n)
            .map_err(|_| FlowCheckpointWireError::AllocationFailed)?;
        out.extend_from_slice(value);
        Ok(out)
    }
    fn option<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, FlowCheckpointWireError>,
    ) -> Result<Option<T>, FlowCheckpointWireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => f(self).map(Some),
            _ => Err(FlowCheckpointWireError::InvalidTag),
        }
    }
    fn finish(self) -> Result<(), FlowCheckpointWireError> {
        if self.at != self.bytes.len() {
            Err(FlowCheckpointWireError::TrailingBytes)
        } else {
            Ok(())
        }
    }
}

impl FlowCheckpointV1 {
    /// Encode the entire Flow image using explicit little-endian fields and
    /// canonical variant tags. The size/aggregate preflight completes before
    /// output allocation.
    #[doc(hidden)]
    pub fn encode_wire_v1(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<Vec<u8>, FlowCheckpointWireError> {
        if self.version != FLOW_CHECKPOINT_VERSION_V1 {
            return Err(FlowCheckpointWireError::InvalidValue(
                "Flow checkpoint version",
            ));
        }
        preflight_counts(self, limits)?;
        let mut measure = Writer::measure(limits);
        encode_flow(self, &mut measure)?;
        let size = measure.len;
        validate_canonical_order(self)?;
        let mut writer = Writer::writing(limits, size)?;
        encode_flow(self, &mut writer)?;
        Ok(writer.finish())
    }

    /// Decode detached Flow DTO data only. Call `restore_checkpoint` to apply
    /// authoritative runtime and registration validation before execution.
    #[doc(hidden)]
    pub fn decode_wire_v1(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        let mut r = Reader::new(bytes, limits)?;
        let image = decode_flow(&mut r)?;
        r.finish()?;
        preflight_counts(&image, limits)?;
        validate_canonical_order(&image)?;
        Ok(image)
    }
}

impl FlowDispatch {
    /// Encode every lifecycle record, error, and callback batch receipt.
    #[doc(hidden)]
    pub fn encode_wire_v1(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<Vec<u8>, FlowCheckpointWireError> {
        let mut m = Writer::measure(limits);
        m.raw(MAGIC)?;
        m.u16(WIRE_SCHEMA)?;
        encode_dispatch(self, &mut m)?;
        let size = m.len;
        let mut w = Writer::writing(limits, size)?;
        w.raw(MAGIC)?;
        w.u16(WIRE_SCHEMA)?;
        encode_dispatch(self, &mut w)?;
        Ok(w.finish())
    }
    /// Decode one detached dispatch value; owner reference validation remains
    /// the responsibility of the enclosing Flow image and coordinator.
    #[doc(hidden)]
    pub fn decode_wire_v1(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        let mut r = Reader::new(bytes, limits)?;
        if r.raw(MAGIC.len())? != MAGIC {
            return Err(FlowCheckpointWireError::InvalidTag);
        }
        let schema = r.u16()?;
        if schema != WIRE_SCHEMA {
            return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
        }
        let dispatch = decode_dispatch(&mut r)?;
        r.finish()?;
        Ok(dispatch)
    }
}

fn preflight_counts(
    image: &FlowCheckpointV1,
    l: FlowCheckpointWireLimits,
) -> Result<(), FlowCheckpointWireError> {
    let f = l.flow;
    let mut rows = 0usize;
    let mut sparse = 0usize;
    macro_rules! add_store {
        ($s:expr) => {
            if let Some(s) = $s {
                rows = rows
                    .checked_add(s.rows.len())
                    .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
                sparse = sparse
                    .checked_add(s.sparse_slots)
                    .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
            }
        };
    }
    add_store!(image.builtins.capacities.as_ref());
    add_store!(image.builtins.queues.as_ref());
    add_store!(image.builtins.requests.as_ref());
    add_store!(image.builtins.deadlines.as_ref());
    add_store!(image.builtins.preempting.as_ref());
    add_store!(image.builtins.allocations.as_ref());
    add_store!(image.builtins.work_specs.as_ref());
    add_store!(image.builtins.work_roles.as_ref());
    add_store!(image.builtins.work_progress.as_ref());
    for s in &image.context_stores {
        rows = rows
            .checked_add(s.rows.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        sparse = sparse
            .checked_add(s.sparse_slots)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    }
    for s in &image.restart_stores {
        rows = rows
            .checked_add(s.rows.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        sparse = sparse
            .checked_add(s.sparse_slots)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    }
    let registrations = image
        .contexts
        .len()
        .checked_add(image.handlers.len())
        .and_then(|n| n.checked_add(image.continuations.len()))
        .and_then(|n| n.checked_add(image.domains.len()))
        .and_then(|n| n.checked_add(image.context_stores.len()))
        .and_then(|n| n.checked_add(image.restart_store_manifest.len()))
        .and_then(|n| n.checked_add(image.restart_stores.len()))
        .and_then(|n| {
            image
                .handlers
                .iter()
                .chain(&image.continuations)
                .chain(&image.domains)
                .try_fold(n, |n, c| n.checked_add(c.callback_ids.len()))
        })
        .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    if image.world.slots.len() > f.max_entities
        || image.world.free_indices.len() > f.max_entities
        || image.world.live_entities.len() > f.max_entities
        || image.scheduler.entries.len() > f.max_scheduler_entries
        || image.resources.len() > f.max_resources
        || image.requests.len() > f.max_requests
        || image.actors.len() > f.max_actors
        || image.work_registrations.len() > f.max_works
        || image.actor_domains.len() > f.max_actors
        || image.commands.len() > f.max_commands
        || image.notifications.len() > f.max_notifications
        || image
            .pending_releases
            .len()
            .checked_add(image.pending_despawns.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?
            > f.max_pending_operations
        || rows > f.max_component_rows
        || sparse > f.max_sparse_slots
        || registrations > f.max_registrations
    {
        return Err(FlowCheckpointWireError::LimitExceeded("Flow collection"));
    }
    Ok(())
}

fn strict_by<T>(items: &[T], mut cmp: impl FnMut(&T, &T) -> std::cmp::Ordering) -> bool {
    items
        .windows(2)
        .all(|pair| cmp(&pair[0], &pair[1]) == std::cmp::Ordering::Less)
}
fn validate_canonical_order(i: &FlowCheckpointV1) -> WResult {
    let ordered = strict_by(&i.resources, |a, b| a.cmp(b))
        && strict_by(&i.requests, |a, b| a.cmp(b))
        && strict_by(&i.actors, |a, b| a.cmp(b))
        && strict_by(&i.actor_domains, |a, b| a.0.cmp(&b.0))
        && strict_by(&i.work_registrations, |a, b| a.id.cmp(&b.id))
        && strict_by(&i.contexts, |a, b| a.runtime_key.cmp(&b.runtime_key))
        && strict_by(&i.handlers, |a, b| a.runtime_key.cmp(&b.runtime_key))
        && strict_by(&i.continuations, |a, b| a.runtime_key.cmp(&b.runtime_key))
        && strict_by(&i.domains, |a, b| {
            (&a.runtime_key, a.kind).cmp(&(&b.runtime_key, b.kind))
        })
        && strict_by(&i.context_stores, |a, b| a.codec_key.cmp(&b.codec_key))
        && strict_by(&i.restart_store_manifest, |a, b| a.0.cmp(&b.0))
        && strict_by(&i.restart_stores, |a, b| a.codec_key.cmp(&b.codec_key))
        && strict_by(&i.commands, |a, b| a.0.cmp(&b.0))
        && strict_by(&i.pending_releases, |a, b| a.cmp(b))
        && strict_by(&i.pending_despawns, |a, b| a.cmp(b))
        && strict_by(&i.scheduler.entries, |a, b| a.sequence.cmp(&b.sequence));
    if !ordered {
        return Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry",
        ));
    }
    if i.scheduler
        .entries
        .iter()
        .any(|e| e.id.index != e.sequence || e.id.generation != e.id.index as u32)
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "scheduler event order",
        ));
    }
    ensure_unique(i.world.free_indices.iter().copied())?;
    ensure_unique(i.world.live_entities.iter().copied())?;
    ensure_unique(i.notifications.iter().map(|(event, _)| *event))?;
    ensure_unique(i.actor_domains.iter().map(|(_, work)| *work))?;
    macro_rules! unique_rows {
        ($store:expr) => {
            if let Some(store) = $store {
                ensure_unique(store.rows.iter().map(|(entity, _)| *entity))?;
            }
        };
    }
    unique_rows!(i.builtins.capacities.as_ref());
    unique_rows!(i.builtins.queues.as_ref());
    unique_rows!(i.builtins.requests.as_ref());
    unique_rows!(i.builtins.deadlines.as_ref());
    unique_rows!(i.builtins.preempting.as_ref());
    unique_rows!(i.builtins.allocations.as_ref());
    unique_rows!(i.builtins.work_specs.as_ref());
    unique_rows!(i.builtins.work_roles.as_ref());
    unique_rows!(i.builtins.work_progress.as_ref());
    for store in &i.context_stores {
        ensure_unique(store.rows.iter().map(|(entity, _)| *entity))?;
    }
    for store in &i.restart_stores {
        ensure_unique(store.rows.iter().map(|(entity, _, _)| *entity))?;
    }
    if i.notifications.iter().any(|(_, n)| n.kind > 1) {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    if let Some(store) = &i.builtins.queues {
        for (_, q) in &store.rows {
            if !q.requests.iter().is_sorted() {
                return Err(FlowCheckpointWireError::InvalidValue("queue key order"));
            }
        }
    }
    if let Some(store) = &i.builtins.deadlines {
        for (_, v) in &store.rows {
            if !v.entries.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue("deadline key order"));
            }
        }
    }
    if let Some(store) = &i.builtins.preempting {
        for (_, v) in &store.rows {
            if !v.keys.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "preemption key order",
                ));
            }
        }
    }
    Ok(())
}

fn ensure_unique<T: Copy + Ord>(items: impl Iterator<Item = T>) -> WResult {
    let mut values = Vec::new();
    for item in items {
        values
            .try_reserve(1)
            .map_err(|_| FlowCheckpointWireError::AllocationFailed)?;
        values.push(item);
    }
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        Err(FlowCheckpointWireError::InvalidValue("duplicate entry"))
    } else {
        Ok(())
    }
}

fn put_scheduler(w: &mut Writer, s: &SchedulerCheckpointV1) -> WResult {
    w.u16(s.schema_version)?;
    put_time(w, s.now)?;
    w.u64(s.next_event_index)?;
    w.u32(s.next_event_generation)?;
    w.u64(s.next_sequence)?;
    w.u64(s.scheduled_events)?;
    w.u64(s.dispatched_events)?;
    w.u64(s.cancelled_events)?;
    put_vec(w, &s.entries, |w, e| {
        put_schedule(w, e.request)?;
        put_event(w, e.id)?;
        w.u64(e.sequence)?;
        w.bool(e.live)
    })
}
fn get_scheduler(r: &mut Reader<'_>) -> WResult<SchedulerCheckpointV1> {
    Ok(SchedulerCheckpointV1 {
        schema_version: r.u16()?,
        now: get_time(r)?,
        next_event_index: r.u64()?,
        next_event_generation: r.u32()?,
        next_sequence: r.u64()?,
        scheduled_events: r.u64()?,
        dispatched_events: r.u64()?,
        cancelled_events: r.u64()?,
        entries: get_vec(
            r,
            r.limits.flow.max_scheduler_entries,
            "scheduler entry",
            46,
            |r| {
                Ok(SchedulerCheckpointEntry {
                    request: get_schedule(r)?,
                    id: get_event(r)?,
                    sequence: r.u64()?,
                    live: r.bool()?,
                })
            },
        )?,
    })
}
fn put_schedule(w: &mut Writer, s: ScheduleRequest) -> WResult {
    put_time(w, s.at)?;
    w.i32(s.priority)?;
    put_option_entity(w, s.entity)?;
    put_kind(w, s.kind)
}
fn get_schedule(r: &mut Reader<'_>) -> WResult<ScheduleRequest> {
    Ok(ScheduleRequest {
        at: get_time(r)?,
        priority: r.i32()?,
        entity: get_option_entity(r)?,
        kind: get_kind(r)?,
    })
}
fn put_world(w: &mut Writer, s: &WorldCheckpointV1) -> WResult {
    w.u32(s.version)?;
    put_vec(w, &s.slots, |w, x| {
        w.u32(x.generation)?;
        w.bool(x.alive)
    })?;
    put_vec(w, &s.free_indices, |w, x| w.u64(*x))?;
    put_vec(w, &s.live_entities, |w, x| put_entity(w, *x))
}
fn get_world(r: &mut Reader<'_>) -> WResult<WorldCheckpointV1> {
    Ok(WorldCheckpointV1 {
        version: r.u32()?,
        slots: get_vec(r, r.limits.flow.max_entities, "world slot", 5, |r| {
            Ok(WorldSlotCheckpointV1 {
                generation: r.u32()?,
                alive: r.bool()?,
            })
        })?,
        free_indices: get_vec(r, r.limits.flow.max_entities, "free index", 8, |r| r.u64())?,
        live_entities: get_vec(r, r.limits.flow.max_entities, "live entity", 12, get_entity)?,
    })
}

fn encode_flow(i: &FlowCheckpointV1, w: &mut Writer) -> WResult {
    w.raw(MAGIC)?;
    w.u16(WIRE_SCHEMA)?;
    w.u32(i.version)?;
    w.u64(i.config.max_same_tick_flow_transitions.get())?;
    w.usize(i.callback_config.max_callback_commands.get())?;
    w.u64(i.next_batch_identity)?;
    put_option_time(w, i.budget_tick)?;
    w.u64(i.budget_consumed)?;
    put_option(w, i.budget_halt, put_halt)?;
    put_scheduler(w, &i.scheduler)?;
    put_world(w, &i.world)?;
    put_vec(w, &i.resources, |w, x| put_resource(w, *x))?;
    put_vec(w, &i.requests, |w, x| put_request(w, *x))?;
    put_vec(w, &i.actors, |w, x| put_entity(w, *x))?;
    put_vec(w, &i.actor_domains, |w, (a, b)| {
        put_entity(w, *a)?;
        put_entity(w, *b)
    })?;
    put_vec(w, &i.work_registrations, put_work_registration)?;
    put_vec(w, &i.contexts, put_context_registration)?;
    put_vec(w, &i.handlers, put_callback_registration)?;
    put_vec(w, &i.continuations, put_callback_registration)?;
    put_vec(w, &i.domains, put_callback_registration)?;
    put_builtin_stores(w, &i.builtins)?;
    put_vec(w, &i.context_stores, put_context_store)?;
    put_vec(w, &i.restart_store_manifest, |w, (key, version)| {
        w.key(key)?;
        w.u32(*version)
    })?;
    put_vec(w, &i.restart_stores, put_restart_store)?;
    put_vec(w, &i.commands, |w, (id, c)| {
        put_event(w, *id)?;
        put_command(w, *c)
    })?;
    put_vec(w, &i.notifications, |w, (id, n)| {
        put_event(w, *id)?;
        put_notification(w, n)
    })?;
    put_vec(w, &i.pending_releases, |w, id| put_lease(w, *id))?;
    put_vec(w, &i.pending_despawns, |w, id| put_entity(w, *id))?;
    w.u64(i.created)?;
    w.u64(i.destroyed)?;
    w.u64(i.scheduled)?;
    w.u64(i.next_admission)?;
    w.u64(i.next_lease)
}
fn decode_flow(r: &mut Reader<'_>) -> WResult<FlowCheckpointV1> {
    let magic = r.raw(MAGIC.len())?;
    if magic != MAGIC {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    let version = r.u32()?;
    if version != FLOW_CHECKPOINT_VERSION_V1 {
        return Err(FlowCheckpointWireError::InvalidValue(
            "Flow checkpoint version",
        ));
    }
    let config = FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(r.u64()?).ok_or(
            FlowCheckpointWireError::InvalidValue("zero transition budget"),
        )?,
    };
    let callback_config = FlowCallbackConfig {
        max_callback_commands: NonZeroUsize::new(r.usize()?)
            .ok_or(FlowCheckpointWireError::InvalidValue("zero callback limit"))?,
    };
    let next_batch_identity = r.u64()?;
    let budget_tick = get_option_time(r)?;
    let budget_consumed = r.u64()?;
    let budget_halt = r.option(get_halt)?;
    let scheduler = get_scheduler(r)?;
    let world = get_world(r)?;
    let resources = get_vec(r, r.limits.flow.max_resources, "resource", 12, get_resource)?;
    let requests = get_vec(r, r.limits.flow.max_requests, "request", 12, get_request)?;
    let actors = get_vec(r, r.limits.flow.max_actors, "actor", 12, get_entity)?;
    let actor_domains = get_vec(r, r.limits.flow.max_actors, "actor domain", 24, |r| {
        Ok((get_entity(r)?, get_entity(r)?))
    })?;
    let work_registrations = get_vec(
        r,
        r.limits.flow.max_works,
        "work registration",
        30,
        get_work_registration,
    )?;
    let contexts = get_registration_vec(r, 20, get_context_registration)?;
    let handlers = get_registration_vec(r, 26, get_callback_registration)?;
    let continuations = get_registration_vec(r, 26, get_callback_registration)?;
    let domains = get_registration_vec(r, 26, get_callback_registration)?;
    let builtins = get_builtin_stores(r)?;
    let context_stores = get_registration_vec(r, 28, get_context_store)?;
    let restart_store_manifest = get_registration_vec(r, 12, |r| Ok((r.key()?, r.u32()?)))?;
    let restart_stores = get_registration_vec(r, 28, get_restart_store)?;
    let commands = get_vec(r, r.limits.flow.max_commands, "command", 13, |r| {
        Ok((get_event(r)?, get_command(r)?))
    })?;
    let notifications = get_vec(
        r,
        r.limits.flow.max_notifications,
        "notification",
        126,
        |r| Ok((get_event(r)?, get_notification(r)?)),
    )?;
    let release_count = r.count(r.limits.flow.max_pending_operations, "pending release")?;
    r.pending_count(release_count)?;
    let pending_releases = get_vec_counted(r, release_count, 20, get_lease)?;
    let despawn_count = r.count(r.limits.flow.max_pending_operations, "pending despawn")?;
    r.pending_count(despawn_count)?;
    let pending_despawns = get_vec_counted(r, despawn_count, 12, get_entity)?;
    Ok(FlowCheckpointV1 {
        version,
        config,
        callback_config,
        next_batch_identity,
        budget_tick,
        budget_consumed,
        budget_halt,
        scheduler,
        world,
        resources,
        requests,
        actors,
        actor_domains,
        work_registrations,
        contexts,
        handlers,
        continuations,
        domains,
        builtins,
        context_stores,
        restart_store_manifest,
        restart_stores,
        commands,
        notifications,
        pending_releases,
        pending_despawns,
        created: r.u64()?,
        destroyed: r.u64()?,
        scheduled: r.u64()?,
        next_admission: r.u64()?,
        next_lease: r.u64()?,
    })
}
// FlowDispatch has a standalone wire endpoint implemented below.

type WResult<T = ()> = Result<T, FlowCheckpointWireError>;

fn put_entity(w: &mut Writer, id: EntityId) -> WResult {
    w.u64(id.index)?;
    w.u32(id.generation)
}
fn get_entity(r: &mut Reader<'_>) -> WResult<EntityId> {
    Ok(EntityId::new(r.u64()?, r.u32()?))
}
fn put_event(w: &mut Writer, id: EventId) -> WResult {
    w.u64(id.index)?;
    w.u32(id.generation)
}
fn get_event(r: &mut Reader<'_>) -> WResult<EventId> {
    Ok(EventId::new(r.u64()?, r.u32()?))
}
fn put_time(w: &mut Writer, t: SimTime) -> WResult {
    w.u128(t.ticks())
}
fn get_time(r: &mut Reader<'_>) -> WResult<SimTime> {
    Ok(SimTime::from_ticks(r.u128()?))
}
fn put_duration(w: &mut Writer, d: SimDuration) -> WResult {
    w.u128(d.ticks())
}
fn get_duration(r: &mut Reader<'_>) -> WResult<SimDuration> {
    Ok(SimDuration::from_ticks(r.u128()?))
}
fn put_kind(w: &mut Writer, k: EventKind) -> WResult {
    w.u32(k.code())
}
fn get_kind(r: &mut Reader<'_>) -> WResult<EventKind> {
    Ok(EventKind::custom(r.u32()?))
}
fn put_resource(w: &mut Writer, id: ResourceId) -> WResult {
    put_entity(w, id.entity_id())
}
fn get_resource(r: &mut Reader<'_>) -> WResult<ResourceId> {
    Ok(ResourceId(get_entity(r)?))
}
fn put_request(w: &mut Writer, id: RequestId) -> WResult {
    put_entity(w, id.entity_id())
}
fn get_request(r: &mut Reader<'_>) -> WResult<RequestId> {
    Ok(RequestId(get_entity(r)?))
}
fn put_work(w: &mut Writer, id: WorkId) -> WResult {
    put_entity(w, id.entity_id())
}
fn get_work(r: &mut Reader<'_>) -> WResult<WorkId> {
    Ok(WorkId(get_entity(r)?))
}
fn put_lease(w: &mut Writer, id: LeaseId) -> WResult {
    put_request(w, id.request_id())?;
    w.u64(id.revision())
}
fn get_lease(r: &mut Reader<'_>) -> WResult<LeaseId> {
    Ok(LeaseId {
        request: get_request(r)?,
        revision: r.u64()?,
    })
}
fn put_option<T>(
    w: &mut Writer,
    v: Option<T>,
    f: impl FnOnce(&mut Writer, T) -> WResult,
) -> WResult {
    w.option(v, f)
}
fn get_vec<T>(
    r: &mut Reader<'_>,
    cap: usize,
    what: &'static str,
    minimum_record_bytes: usize,
    f: impl FnMut(&mut Reader<'_>) -> WResult<T>,
) -> WResult<Vec<T>> {
    let n = r.count(cap, what)?;
    get_vec_counted(r, n, minimum_record_bytes, f)
}
fn get_vec_counted<T>(
    r: &mut Reader<'_>,
    n: usize,
    minimum_record_bytes: usize,
    mut f: impl FnMut(&mut Reader<'_>) -> WResult<T>,
) -> WResult<Vec<T>> {
    r.require_min_items(n, minimum_record_bytes)?;
    let mut v = r.reserve(n)?;
    for _ in 0..n {
        v.push(f(r)?);
    }
    Ok(v)
}
fn get_registration_vec<T>(
    r: &mut Reader<'_>,
    minimum_record_bytes: usize,
    f: impl FnMut(&mut Reader<'_>) -> WResult<T>,
) -> WResult<Vec<T>> {
    let n = r.count(r.limits.flow.max_registrations, "registration")?;
    r.registration_count(n)?;
    r.require_min_items(n, minimum_record_bytes)?;
    let mut values = r.reserve(n)?;
    let mut f = f;
    for _ in 0..n {
        values.push(f(r)?);
    }
    Ok(values)
}
fn put_vec<T>(w: &mut Writer, v: &[T], mut f: impl FnMut(&mut Writer, &T) -> WResult) -> WResult {
    w.count(v.len())?;
    for item in v {
        f(w, item)?;
    }
    Ok(())
}
fn put_option_time(w: &mut Writer, v: Option<SimTime>) -> WResult {
    put_option(w, v, put_time)
}
fn get_option_time(r: &mut Reader<'_>) -> WResult<Option<SimTime>> {
    r.option(get_time)
}
fn put_option_event(w: &mut Writer, v: Option<EventId>) -> WResult {
    put_option(w, v, put_event)
}
fn get_option_event(r: &mut Reader<'_>) -> WResult<Option<EventId>> {
    r.option(get_event)
}
fn put_option_entity(w: &mut Writer, v: Option<EntityId>) -> WResult {
    put_option(w, v, put_entity)
}
fn get_option_entity(r: &mut Reader<'_>) -> WResult<Option<EntityId>> {
    r.option(get_entity)
}
fn put_option_request(w: &mut Writer, v: Option<RequestId>) -> WResult {
    put_option(w, v, put_request)
}
fn get_option_request(r: &mut Reader<'_>) -> WResult<Option<RequestId>> {
    r.option(get_request)
}
fn put_option_work(w: &mut Writer, v: Option<WorkId>) -> WResult {
    put_option(w, v, put_work)
}
fn get_option_work(r: &mut Reader<'_>) -> WResult<Option<WorkId>> {
    r.option(get_work)
}
fn put_option_lease(w: &mut Writer, v: Option<LeaseId>) -> WResult {
    put_option(w, v, put_lease)
}
fn get_option_lease(r: &mut Reader<'_>) -> WResult<Option<LeaseId>> {
    r.option(get_lease)
}

fn put_strategy(w: &mut Writer, s: PreemptionStrategy) -> WResult {
    w.u8(match s {
        PreemptionStrategy::Suspend => 0,
        PreemptionStrategy::Abort => 1,
        PreemptionStrategy::Restart => 2,
    })
}
fn get_strategy(r: &mut Reader<'_>) -> WResult<PreemptionStrategy> {
    match r.u8()? {
        0 => Ok(PreemptionStrategy::Suspend),
        1 => Ok(PreemptionStrategy::Abort),
        2 => Ok(PreemptionStrategy::Restart),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn put_opt_strategy(w: &mut Writer, v: Option<PreemptionStrategy>) -> WResult {
    put_option(w, v, put_strategy)
}
fn get_opt_strategy(r: &mut Reader<'_>) -> WResult<Option<PreemptionStrategy>> {
    r.option(get_strategy)
}
fn put_request_state(w: &mut Writer, s: RequestState) -> WResult {
    w.u8(match s {
        RequestState::Pending => 0,
        RequestState::Queued => 1,
        RequestState::Active => 2,
        RequestState::Released => 3,
        RequestState::Cancelled => 4,
        RequestState::TimedOut => 5,
        RequestState::Suspended => 6,
        RequestState::Completed => 7,
        RequestState::Aborted => 8,
    })
}
fn get_request_state(r: &mut Reader<'_>) -> WResult<RequestState> {
    match r.u8()? {
        0 => Ok(RequestState::Pending),
        1 => Ok(RequestState::Queued),
        2 => Ok(RequestState::Active),
        3 => Ok(RequestState::Released),
        4 => Ok(RequestState::Cancelled),
        5 => Ok(RequestState::TimedOut),
        6 => Ok(RequestState::Suspended),
        7 => Ok(RequestState::Completed),
        8 => Ok(RequestState::Aborted),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn put_work_state(w: &mut Writer, s: WorkState) -> WResult {
    w.u8(match s {
        WorkState::Pending => 0,
        WorkState::Active => 1,
        WorkState::Suspended => 2,
        WorkState::Completed => 3,
        WorkState::Aborted => 4,
        WorkState::Cancelled => 5,
        WorkState::Released => 6,
    })
}
fn get_work_state(r: &mut Reader<'_>) -> WResult<WorkState> {
    match r.u8()? {
        0 => Ok(WorkState::Pending),
        1 => Ok(WorkState::Active),
        2 => Ok(WorkState::Suspended),
        3 => Ok(WorkState::Completed),
        4 => Ok(WorkState::Aborted),
        5 => Ok(WorkState::Cancelled),
        6 => Ok(WorkState::Released),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn put_transition(w: &mut Writer, s: LifecycleTransition) -> WResult {
    w.u8(match s {
        LifecycleTransition::Queued => 0,
        LifecycleTransition::Granted => 1,
        LifecycleTransition::Released => 2,
        LifecycleTransition::Cancelled => 3,
        LifecycleTransition::TimedOut => 4,
        LifecycleTransition::Preempted => 5,
        LifecycleTransition::Resumed => 6,
        LifecycleTransition::Restarted => 7,
        LifecycleTransition::Completed => 8,
        LifecycleTransition::Aborted => 9,
    })
}
fn get_transition(r: &mut Reader<'_>) -> WResult<LifecycleTransition> {
    match r.u8()? {
        0 => Ok(LifecycleTransition::Queued),
        1 => Ok(LifecycleTransition::Granted),
        2 => Ok(LifecycleTransition::Released),
        3 => Ok(LifecycleTransition::Cancelled),
        4 => Ok(LifecycleTransition::TimedOut),
        5 => Ok(LifecycleTransition::Preempted),
        6 => Ok(LifecycleTransition::Resumed),
        7 => Ok(LifecycleTransition::Restarted),
        8 => Ok(LifecycleTransition::Completed),
        9 => Ok(LifecycleTransition::Aborted),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn put_domain_control(w: &mut Writer, s: FlowDomainControl) -> WResult {
    w.u8(match s {
        FlowDomainControl::Pause => 0,
        FlowDomainControl::Resume => 1,
    })
}
fn get_domain_control(r: &mut Reader<'_>) -> WResult<FlowDomainControl> {
    match r.u8()? {
        0 => Ok(FlowDomainControl::Pause),
        1 => Ok(FlowDomainControl::Resume),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}

fn put_progress(w: &mut Writer, p: &WorkProgress) -> WResult {
    put_duration(w, p.original_duration)?;
    put_duration(w, p.useful_elapsed)?;
    put_duration(w, p.remaining)?;
    put_duration(w, p.cumulative_busy)?;
    w.u64(p.attempt_revision)?;
    w.u64(p.execution_revision)?;
    put_work_state(w, p.state)?;
    put_option_time(w, p.segment_started_at)?;
    put_option_time(w, p.completion_at)?;
    w.bool(p.restart_pending)
}
fn get_progress(r: &mut Reader<'_>) -> WResult<WorkProgress> {
    Ok(WorkProgress {
        original_duration: get_duration(r)?,
        useful_elapsed: get_duration(r)?,
        remaining: get_duration(r)?,
        cumulative_busy: get_duration(r)?,
        attempt_revision: r.u64()?,
        execution_revision: r.u64()?,
        state: get_work_state(r)?,
        segment_started_at: get_option_time(r)?,
        completion_at: get_option_time(r)?,
        restart_pending: r.bool()?,
    })
}

fn put_preview(w: &mut Writer, p: ScheduledEventPreview) -> WResult {
    put_event(w, p.id)?;
    put_time(w, p.at)?;
    w.i32(p.priority)?;
    w.u64(p.sequence)?;
    put_option_entity(w, p.entity)?;
    put_kind(w, p.kind)
}
fn get_preview(r: &mut Reader<'_>) -> WResult<ScheduledEventPreview> {
    Ok(ScheduledEventPreview {
        id: get_event(r)?,
        at: get_time(r)?,
        priority: r.i32()?,
        sequence: r.u64()?,
        entity: get_option_entity(r)?,
        kind: get_kind(r)?,
    })
}
fn put_halt(w: &mut Writer, h: FlowBudgetHalt) -> WResult {
    put_time(w, h.at)?;
    w.u64(h.consumed)?;
    w.u64(h.required_cost)?;
    put_preview(w, h.pending)
}
fn get_halt(r: &mut Reader<'_>) -> WResult<FlowBudgetHalt> {
    Ok(FlowBudgetHalt {
        at: get_time(r)?,
        consumed: r.u64()?,
        required_cost: r.u64()?,
        pending: get_preview(r)?,
    })
}

fn put_flow_error(w: &mut Writer, e: FlowError) -> WResult {
    match e {
        FlowError::InvalidEntity => w.u8(0),
        FlowError::InvalidResource => w.u8(1),
        FlowError::InvalidRequest => w.u8(2),
        FlowError::TerminalRequest => w.u8(3),
        FlowError::InvalidLease => w.u8(4),
        FlowError::CapacityInUse => w.u8(5),
        FlowError::ResourceInUse => w.u8(6),
        FlowError::PastCommand => w.u8(7),
        FlowError::CounterOverflow => w.u8(8),
        FlowError::InvalidState => w.u8(9),
        FlowError::InvalidWork => w.u8(10),
        FlowError::DuplicateActorDomainContext => w.u8(11),
        FlowError::DuplicateActorDespawn => w.u8(12),
        FlowError::ReservedEventKind => w.u8(13),
        FlowError::UnregisteredDomainEvent => w.u8(14),
        FlowError::InvalidCommandTicket => w.u8(15),
        FlowError::CallbackBatchLimitExceeded => w.u8(16),
        FlowError::SameTickBudgetExceeded { at_ticks, limit } => {
            w.u8(17)?;
            w.u128(at_ticks)?;
            w.u64(limit)
        }
        FlowError::RunHalted => w.u8(18),
    }
}
fn get_flow_error(r: &mut Reader<'_>) -> WResult<FlowError> {
    Ok(match r.u8()? {
        0 => FlowError::InvalidEntity,
        1 => FlowError::InvalidResource,
        2 => FlowError::InvalidRequest,
        3 => FlowError::TerminalRequest,
        4 => FlowError::InvalidLease,
        5 => FlowError::CapacityInUse,
        6 => FlowError::ResourceInUse,
        7 => FlowError::PastCommand,
        8 => FlowError::CounterOverflow,
        9 => FlowError::InvalidState,
        10 => FlowError::InvalidWork,
        11 => FlowError::DuplicateActorDomainContext,
        12 => FlowError::DuplicateActorDespawn,
        13 => FlowError::ReservedEventKind,
        14 => FlowError::UnregisteredDomainEvent,
        15 => FlowError::InvalidCommandTicket,
        16 => FlowError::CallbackBatchLimitExceeded,
        17 => FlowError::SameTickBudgetExceeded {
            at_ticks: r.u128()?,
            limit: r.u64()?,
        },
        18 => FlowError::RunHalted,
        _ => return Err(FlowCheckpointWireError::InvalidTag),
    })
}

fn put_ticket(w: &mut Writer, t: FlowCommandTicket) -> WResult {
    let (b, i) = t.checkpoint_parts();
    w.u64(b)?;
    w.usize(i)
}
fn get_ticket(r: &mut Reader<'_>) -> WResult<FlowCommandTicket> {
    Ok(FlowCommandTicket {
        batch: r.u64()?,
        index: r.usize()?,
    })
}

fn put_snapshot(w: &mut Writer, s: &LifecycleSnapshot) -> WResult {
    put_entity(w, s.owner)?;
    put_option_work(w, s.work)?;
    w.i32(s.priority_level)?;
    w.u32(s.capacity)?;
    w.u32(s.queue_len)?;
    w.u32(s.active_count)?;
    put_option(w, s.strategy, put_strategy)?;
    put_option_request(w, s.preemptor_request)?;
    put_option_lease(w, s.causal_lease)?;
    put_option(w, s.progress.as_ref(), put_progress)
}
fn get_snapshot(r: &mut Reader<'_>) -> WResult<LifecycleSnapshot> {
    Ok(LifecycleSnapshot {
        owner: get_entity(r)?,
        work: get_option_work(r)?,
        priority_level: r.i32()?,
        capacity: r.u32()?,
        queue_len: r.u32()?,
        active_count: r.u32()?,
        strategy: r.option(get_strategy)?,
        preemptor_request: get_option_request(r)?,
        causal_lease: get_option_lease(r)?,
        progress: r.option(get_progress)?,
    })
}
fn put_record(w: &mut Writer, v: &LifecycleRecord) -> WResult {
    put_request(w, v.request)?;
    put_resource(w, v.resource)?;
    put_time(w, v.at)?;
    put_request_state(w, v.state)?;
    put_option_lease(w, v.lease)?;
    put_event(w, v.causal_event_id)?;
    w.u32(v.transition_ordinal)?;
    put_transition(w, v.transition)?;
    put_snapshot(w, &v.snapshot)
}
fn get_record(r: &mut Reader<'_>) -> WResult<LifecycleRecord> {
    Ok(LifecycleRecord {
        request: get_request(r)?,
        resource: get_resource(r)?,
        at: get_time(r)?,
        state: get_request_state(r)?,
        lease: get_option_lease(r)?,
        causal_event_id: get_event(r)?,
        transition_ordinal: r.u32()?,
        transition: get_transition(r)?,
        snapshot: get_snapshot(r)?,
    })
}
fn put_admission(w: &mut Writer, a: &FlowCommandAdmission) -> WResult {
    put_ticket(w, a.ticket)?;
    put_event(w, a.event)?;
    put_option(w, a.request, put_request)?;
    put_option_event(w, a.deadline_event)
}
fn get_admission(r: &mut Reader<'_>) -> WResult<FlowCommandAdmission> {
    Ok(FlowCommandAdmission {
        ticket: get_ticket(r)?,
        event: get_event(r)?,
        request: r.option(get_request)?,
        deadline_event: get_option_event(r)?,
    })
}
fn put_batch(w: &mut Writer, b: &FlowBatchReceipt) -> WResult {
    match b {
        FlowBatchReceipt::Accepted(items) => {
            w.u8(0)?;
            put_vec(w, items, put_admission)
        }
        FlowBatchReceipt::Rejected(rej) => {
            w.u8(1)?;
            put_option(w, rej.failed_ticket, put_ticket)?;
            put_flow_error(w, rej.error)
        }
    }
}
fn get_batch(r: &mut Reader<'_>) -> WResult<FlowBatchReceipt> {
    match r.u8()? {
        0 => {
            let n = r.count(r.limits.flow.max_commands, "dispatch admission")?;
            r.dispatch_admissions(n)?;
            r.require_min_items(n, 30)?;
            let mut items = r.reserve(n)?;
            for _ in 0..n {
                items.push(get_admission(r)?);
            }
            Ok(FlowBatchReceipt::Accepted(items))
        }
        1 => Ok(FlowBatchReceipt::Rejected(FlowBatchRejection {
            failed_ticket: r.option(get_ticket)?,
            error: get_flow_error(r)?,
        })),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn encode_dispatch(d: &FlowDispatch, w: &mut Writer) -> WResult {
    if d.records.len() > w.limits.flow.max_commands
        || d.callback_batches.len() > w.limits.flow.max_commands
    {
        return Err(FlowCheckpointWireError::LimitExceeded(
            "dispatch collection",
        ));
    }
    let admissions = d.callback_batches.iter().try_fold(0usize, |n, b| {
        n.checked_add(match b {
            FlowBatchReceipt::Accepted(a) => a.len(),
            FlowBatchReceipt::Rejected(_) => 0,
        })
        .ok_or(FlowCheckpointWireError::IntegerOverflow)
    })?;
    if admissions > w.limits.flow.max_commands {
        return Err(FlowCheckpointWireError::LimitExceeded(
            "dispatch admissions",
        ));
    }
    put_event(w, d.event)?;
    put_time(w, d.at)?;
    put_vec(w, &d.records, put_record)?;
    put_option(w, d.error, put_flow_error)?;
    put_vec(w, &d.callback_batches, put_batch)
}
fn decode_dispatch(r: &mut Reader<'_>) -> WResult<FlowDispatch> {
    let event = get_event(r)?;
    let at = get_time(r)?;
    let records = get_vec(
        r,
        r.limits.flow.max_commands,
        "dispatch record",
        92,
        get_record,
    )?;
    let error = r.option(get_flow_error)?;
    let callback_batches = get_vec(
        r,
        r.limits.flow.max_commands,
        "dispatch batch",
        3,
        get_batch,
    )?;
    Ok(FlowDispatch {
        event,
        at,
        records,
        error,
        callback_batches,
    })
}

fn put_callback_code(w: &mut Writer, c: &FlowCallbackCodeV1) -> WResult {
    w.key(&c.stable_id)?;
    w.u32(c.version)
}
fn get_callback_code(r: &mut Reader<'_>) -> WResult<FlowCallbackCodeV1> {
    Ok(FlowCallbackCodeV1 {
        stable_id: r.key()?,
        version: r.u32()?,
    })
}
fn put_callback_registration(w: &mut Writer, c: &FlowCallbackRegistrationV1) -> WResult {
    w.key(&c.runtime_key)?;
    w.key(&c.context_codec_key)?;
    put_option(w, c.kind, put_kind)?;
    w.u8(c.variant)?;
    put_vec(w, &c.callback_ids, |w, (name, code)| {
        w.key(name)?;
        put_callback_code(w, code)
    })
}
fn get_callback_registration(r: &mut Reader<'_>) -> WResult<FlowCallbackRegistrationV1> {
    Ok(FlowCallbackRegistrationV1 {
        runtime_key: r.key()?,
        context_codec_key: r.key()?,
        kind: r.option(get_kind)?,
        variant: r.u8()?,
        callback_ids: get_registration_vec(r, 20, |r| Ok((r.key()?, get_callback_code(r)?)))?,
    })
}
fn put_context_registration(w: &mut Writer, c: &FlowContextRegistrationV1) -> WResult {
    w.key(&c.runtime_key)?;
    w.key(&c.codec_key)?;
    w.u32(c.version)
}
fn get_context_registration(r: &mut Reader<'_>) -> WResult<FlowContextRegistrationV1> {
    Ok(FlowContextRegistrationV1 {
        runtime_key: r.key()?,
        codec_key: r.key()?,
        version: r.u32()?,
    })
}
fn put_work_registration(w: &mut Writer, c: &FlowWorkRegistrationV1) -> WResult {
    put_entity(w, c.id)?;
    w.key(&c.context_runtime_key)?;
    w.key(&c.context_codec_key)?;
    put_option(w, c.restart_codec_key.as_deref(), |w, v| w.key(v))?;
    put_option(w, c.restart_codec_version, |w, v| w.u32(v))
}
fn get_work_registration(r: &mut Reader<'_>) -> WResult<FlowWorkRegistrationV1> {
    Ok(FlowWorkRegistrationV1 {
        id: get_entity(r)?,
        context_runtime_key: r.key()?,
        context_codec_key: r.key()?,
        restart_codec_key: r.option(|r| r.key())?,
        restart_codec_version: r.option(|r| r.u32())?,
    })
}

fn put_context_store(w: &mut Writer, s: &FlowContextStoreV1) -> WResult {
    w.key(&s.codec_key)?;
    w.u32(s.version)?;
    w.usize(s.sparse_slots)?;
    put_vec(w, &s.rows, |w, (id, bytes)| {
        put_entity(w, *id)?;
        w.payload(bytes)
    })
}
fn get_context_store(r: &mut Reader<'_>) -> WResult<FlowContextStoreV1> {
    let codec_key = r.key()?;
    let version = r.u32()?;
    let sparse_slots = r.usize()?;
    let n = r.count(r.limits.flow.max_component_rows, "context row")?;
    r.component_dimensions(n, sparse_slots)?;
    r.require_min_items(n, 20)?;
    let mut rows = r.reserve(n)?;
    for _ in 0..n {
        rows.push((get_entity(r)?, r.payload()?));
    }
    Ok(FlowContextStoreV1 {
        codec_key,
        version,
        sparse_slots,
        rows,
    })
}
fn put_restart_store(w: &mut Writer, s: &FlowRestartStoreV1) -> WResult {
    w.key(&s.codec_key)?;
    w.u32(s.version)?;
    w.usize(s.sparse_slots)?;
    put_vec(w, &s.rows, |w, (id, factory, bytes)| {
        put_entity(w, *id)?;
        w.key(factory)?;
        w.payload(bytes)
    })
}
fn get_restart_store(r: &mut Reader<'_>) -> WResult<FlowRestartStoreV1> {
    let codec_key = r.key()?;
    let version = r.u32()?;
    let sparse_slots = r.usize()?;
    let n = r.count(r.limits.flow.max_component_rows, "restart row")?;
    r.component_dimensions(n, sparse_slots)?;
    r.require_min_items(n, 28)?;
    let mut rows = r.reserve(n)?;
    for _ in 0..n {
        rows.push((get_entity(r)?, r.key()?, r.payload()?));
    }
    Ok(FlowRestartStoreV1 {
        codec_key,
        version,
        sparse_slots,
        rows,
    })
}

fn put_priority(w: &mut Writer, k: PriorityKey) -> WResult {
    w.i32(k.level)?;
    w.u64(k.enqueue_sequence)?;
    put_request(w, k.request)
}
fn get_priority(r: &mut Reader<'_>) -> WResult<PriorityKey> {
    Ok(PriorityKey {
        level: r.i32()?,
        enqueue_sequence: r.u64()?,
        request: get_request(r)?,
    })
}
fn put_component<T>(
    w: &mut Writer,
    s: &ComponentStoreCheckpointV1<T>,
    mut put: impl FnMut(&mut Writer, &T) -> WResult,
) -> WResult {
    w.u32(s.version)?;
    w.usize(s.sparse_slots)?;
    put_vec(w, &s.rows, |w, (id, value)| {
        put_entity(w, *id)?;
        put(w, value)
    })
}
fn get_component<T>(
    r: &mut Reader<'_>,
    minimum_record_bytes: usize,
    mut put: impl FnMut(&mut Reader<'_>) -> WResult<T>,
) -> WResult<ComponentStoreCheckpointV1<T>> {
    let version = r.u32()?;
    let sparse_slots = r.usize()?;
    let n = r.count(r.limits.flow.max_component_rows, "component row")?;
    r.component_dimensions(n, sparse_slots)?;
    r.require_min_items(n, minimum_record_bytes)?;
    let mut rows = r.reserve(n)?;
    for _ in 0..n {
        rows.push((get_entity(r)?, put(r)?));
    }
    Ok(ComponentStoreCheckpointV1 {
        version,
        sparse_slots,
        rows,
    })
}
fn put_capacity(w: &mut Writer, v: &ResourceCapacity) -> WResult {
    w.u32(v.total)
}
fn get_capacity(r: &mut Reader<'_>) -> WResult<ResourceCapacity> {
    Ok(ResourceCapacity { total: r.u32()? })
}
fn put_claim_queue(w: &mut Writer, v: &ClaimQueue) -> WResult {
    w.count(v.requests.len())?;
    for key in &v.requests {
        put_priority(w, *key)?;
    }
    Ok(())
}
fn get_claim_queue(r: &mut Reader<'_>) -> WResult<ClaimQueue> {
    let keys = get_vec(r, r.limits.flow.max_requests, "queue key", 24, get_priority)?;
    let mut set = BTreeSet::new();
    let mut previous = None;
    for key in keys {
        if previous.is_some_and(|old| old >= key) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "queue key order or duplicate",
            ));
        }
        previous = Some(key);
        set.insert(key);
    }
    Ok(ClaimQueue { requests: set })
}
fn put_resource_request(w: &mut Writer, v: &ResourceRequest) -> WResult {
    put_resource(w, v.resource)?;
    put_entity(w, v.owner)?;
    put_request_state(w, v.state)?;
    put_option(w, v.admission_sequence, |w, x| w.u64(x))?;
    put_option_lease(w, v.lease)?;
    w.i32(v.priority_level)?;
    put_option_work(w, v.work)?;
    put_time(w, v.submitted_at)?;
    put_option_time(w, v.deadline)?;
    w.bool(v.timed)?;
    w.bool(v.can_preempt)?;
    put_opt_strategy(w, v.preemptible)
}
fn get_resource_request(r: &mut Reader<'_>) -> WResult<ResourceRequest> {
    Ok(ResourceRequest {
        resource: get_resource(r)?,
        owner: get_entity(r)?,
        state: get_request_state(r)?,
        admission_sequence: r.option(|r| r.u64())?,
        lease: get_option_lease(r)?,
        priority_level: r.i32()?,
        work: get_option_work(r)?,
        submitted_at: get_time(r)?,
        deadline: get_option_time(r)?,
        timed: r.bool()?,
        can_preempt: r.bool()?,
        preemptible: get_opt_strategy(r)?,
    })
}
fn put_deadline(w: &mut Writer, v: &FlowDeadlineIndexV1) -> WResult {
    w.usize(v.expected_len)?;
    put_vec(w, &v.entries, |w, (time, key)| {
        put_time(w, *time)?;
        put_priority(w, *key)
    })
}
fn get_deadline(r: &mut Reader<'_>) -> WResult<FlowDeadlineIndexV1> {
    Ok(FlowDeadlineIndexV1 {
        expected_len: r.usize()?,
        entries: get_vec(r, r.limits.flow.max_requests, "deadline entry", 40, |r| {
            Ok((get_time(r)?, get_priority(r)?))
        })?,
    })
}
fn put_preempting(w: &mut Writer, v: &FlowPreemptingIndexV1) -> WResult {
    w.usize(v.expected_len)?;
    put_vec(w, &v.keys, |w, k| put_priority(w, *k))
}
fn get_preempting(r: &mut Reader<'_>) -> WResult<FlowPreemptingIndexV1> {
    Ok(FlowPreemptingIndexV1 {
        expected_len: r.usize()?,
        keys: get_vec(
            r,
            r.limits.flow.max_requests,
            "preemption key",
            24,
            get_priority,
        )?,
    })
}
fn put_allocation(w: &mut Writer, v: &Allocation) -> WResult {
    put_lease(w, v.lease)?;
    put_request(w, v.request)?;
    put_entity(w, v.owner)?;
    put_option_work(w, v.work)?;
    w.i32(v.priority_level)?;
    put_time(w, v.granted_at)?;
    put_time(w, v.segment_started_at)?;
    put_option_time(w, v.completion_at)
}
fn get_allocation(r: &mut Reader<'_>) -> WResult<Allocation> {
    Ok(Allocation {
        lease: get_lease(r)?,
        request: get_request(r)?,
        owner: get_entity(r)?,
        work: get_option_work(r)?,
        priority_level: r.i32()?,
        granted_at: get_time(r)?,
        segment_started_at: get_time(r)?,
        completion_at: get_option_time(r)?,
    })
}
fn put_active_allocations(w: &mut Writer, v: &ActiveAllocations) -> WResult {
    w.count(v.leases.len())?;
    for (lease, allocation) in &v.leases {
        put_lease(w, *lease)?;
        put_allocation(w, allocation)?;
    }
    Ok(())
}
fn get_active_allocations(r: &mut Reader<'_>) -> WResult<ActiveAllocations> {
    let n = r.count(r.limits.flow.max_requests, "allocation")?;
    r.require_min_items(n, 102)?;
    let mut leases = BTreeMap::new();
    let mut prev = None;
    for _ in 0..n {
        let key = get_lease(r)?;
        if prev.is_some_and(|p| p >= key) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "allocation key order",
            ));
        }
        prev = Some(key);
        let val = get_allocation(r)?;
        leases.insert(key, val);
    }
    Ok(ActiveAllocations { leases })
}
fn put_work_spec(w: &mut Writer, v: &WorkSpec) -> WResult {
    put_entity(w, v.owner)?;
    put_duration(w, v.original_duration)?;
    w.key(&v.context_type_key)?;
    put_option_request(w, v.request)
}
fn get_work_spec(r: &mut Reader<'_>) -> WResult<WorkSpec> {
    Ok(WorkSpec {
        owner: get_entity(r)?,
        original_duration: get_duration(r)?,
        context_type_key: r.key()?,
        request: get_option_request(r)?,
    })
}
fn put_role(w: &mut Writer, v: FlowWorkRoleV1) -> WResult {
    match v {
        FlowWorkRoleV1::Task => w.u8(0),
        FlowWorkRoleV1::ActorDomain { actor, kind } => {
            w.u8(1)?;
            put_entity(w, actor)?;
            put_kind(w, kind)
        }
    }
}
fn get_role(r: &mut Reader<'_>) -> WResult<FlowWorkRoleV1> {
    match r.u8()? {
        0 => Ok(FlowWorkRoleV1::Task),
        1 => Ok(FlowWorkRoleV1::ActorDomain {
            actor: get_entity(r)?,
            kind: get_kind(r)?,
        }),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn put_builtin_stores(w: &mut Writer, b: &FlowBuiltInStoresV1) -> WResult {
    put_option(w, b.capacities.as_ref(), |w, s| {
        put_component(w, s, put_capacity)
    })?;
    put_option(w, b.queues.as_ref(), |w, s| {
        put_component(w, s, put_claim_queue)
    })?;
    put_option(w, b.requests.as_ref(), |w, s| {
        put_component(w, s, put_resource_request)
    })?;
    put_option(w, b.deadlines.as_ref(), |w, s| {
        put_component(w, s, put_deadline)
    })?;
    put_option(w, b.preempting.as_ref(), |w, s| {
        put_component(w, s, put_preempting)
    })?;
    put_option(w, b.allocations.as_ref(), |w, s| {
        put_component(w, s, put_active_allocations)
    })?;
    put_option(w, b.work_specs.as_ref(), |w, s| {
        put_component(w, s, put_work_spec)
    })?;
    put_option(w, b.work_roles.as_ref(), |w, s| {
        put_component(w, s, |w, v| put_role(w, *v))
    })?;
    put_option(w, b.work_progress.as_ref(), |w, s| {
        put_component(w, s, put_progress)
    })
}
fn get_builtin_stores(r: &mut Reader<'_>) -> WResult<FlowBuiltInStoresV1> {
    Ok(FlowBuiltInStoresV1 {
        capacities: r.option(|r| get_component(r, 16, get_capacity))?,
        queues: r.option(|r| get_component(r, 20, get_claim_queue))?,
        requests: r.option(|r| get_component(r, 52, get_resource_request))?,
        deadlines: r.option(|r| get_component(r, 28, get_deadline))?,
        preempting: r.option(|r| get_component(r, 28, get_preempting))?,
        allocations: r.option(|r| get_component(r, 20, get_active_allocations))?,
        work_specs: r.option(|r| get_component(r, 37, get_work_spec))?,
        work_roles: r.option(|r| get_component(r, 13, get_role))?,
        work_progress: r.option(|r| get_component(r, 96, get_progress))?,
    })
}

fn put_command(w: &mut Writer, c: FlowCommandV1) -> WResult {
    match c {
        FlowCommandV1::Submit(x) => {
            w.u8(0)?;
            put_request(w, x)
        }
        FlowCommandV1::Release(x) => {
            w.u8(1)?;
            put_lease(w, x)
        }
        FlowCommandV1::Capacity(x, n) => {
            w.u8(2)?;
            put_resource(w, x)?;
            w.u32(n)
        }
        FlowCommandV1::Remove(x) => {
            w.u8(3)?;
            put_resource(w, x)
        }
        FlowCommandV1::Despawn(x) => {
            w.u8(4)?;
            put_entity(w, x)
        }
        FlowCommandV1::Deadline(x) => {
            w.u8(5)?;
            put_request(w, x)
        }
        FlowCommandV1::Cancel(x) => {
            w.u8(6)?;
            put_request(w, x)
        }
        FlowCommandV1::Reprioritize(x, n) => {
            w.u8(7)?;
            put_request(w, x)?;
            w.i32(n)
        }
        FlowCommandV1::Completion(req, lease, rev, at) => {
            w.u8(8)?;
            put_request(w, req)?;
            put_lease(w, lease)?;
            w.u64(rev)?;
            put_time(w, at)
        }
        FlowCommandV1::Notify => w.u8(9),
        FlowCommandV1::Domain(work, kind) => {
            w.u8(10)?;
            put_work(w, work)?;
            put_kind(w, kind)
        }
        FlowCommandV1::DomainControl(work, kind, action) => {
            w.u8(11)?;
            put_work(w, work)?;
            put_kind(w, kind)?;
            put_domain_control(w, action)
        }
    }
}
fn get_command(r: &mut Reader<'_>) -> WResult<FlowCommandV1> {
    match r.u8()? {
        0 => Ok(FlowCommandV1::Submit(get_request(r)?)),
        1 => Ok(FlowCommandV1::Release(get_lease(r)?)),
        2 => Ok(FlowCommandV1::Capacity(get_resource(r)?, r.u32()?)),
        3 => Ok(FlowCommandV1::Remove(get_resource(r)?)),
        4 => Ok(FlowCommandV1::Despawn(get_entity(r)?)),
        5 => Ok(FlowCommandV1::Deadline(get_request(r)?)),
        6 => Ok(FlowCommandV1::Cancel(get_request(r)?)),
        7 => Ok(FlowCommandV1::Reprioritize(get_request(r)?, r.i32()?)),
        8 => Ok(FlowCommandV1::Completion(
            get_request(r)?,
            get_lease(r)?,
            r.u64()?,
            get_time(r)?,
        )),
        9 => Ok(FlowCommandV1::Notify),
        10 => Ok(FlowCommandV1::Domain(get_work(r)?, get_kind(r)?)),
        11 => Ok(FlowCommandV1::DomainControl(
            get_work(r)?,
            get_kind(r)?,
            get_domain_control(r)?,
        )),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn put_notification(w: &mut Writer, n: &FlowNotificationV1) -> WResult {
    w.u8(n.kind)?;
    put_work(w, n.work)?;
    put_transition(w, n.transition)?;
    put_progress(w, &n.progress)?;
    put_event(w, n.origin)?;
    w.u32(n.ordinal)
}
fn get_notification(r: &mut Reader<'_>) -> WResult<FlowNotificationV1> {
    let kind = r.u8()?;
    if kind > 1 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    Ok(FlowNotificationV1 {
        kind,
        work: get_work(r)?,
        transition: get_transition(r)?,
        progress: get_progress(r)?,
        origin: get_event(r)?,
        ordinal: r.u32()?,
    })
}
