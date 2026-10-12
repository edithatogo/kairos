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
const MAGIC_V2: &[u8; 8] = b"KFLOWV2\0";
const MAGIC_V4: &[u8; 8] = b"KFLOWV4\0";
const WIRE_SCHEMA: u16 = 1;
const MAX_WIRE_QUEUE_ORDER_KEY_BYTES: usize = 128;

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
    key_limit_label: &'static str,
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
            key_limit_label: "aggregate key byte",
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
            key_limit_label: "aggregate key byte",
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
            return Err(FlowCheckpointWireError::LimitExceeded(self.key_limit_label));
        }
        self.usize(v.len())?;
        self.raw(v.as_bytes())
    }
    fn opaque_key(&mut self, value: &[u8]) -> Result<(), FlowCheckpointWireError> {
        if value.is_empty() {
            return Err(FlowCheckpointWireError::InvalidValue(
                "stable queue-order key",
            ));
        }
        if value.len() > MAX_WIRE_QUEUE_ORDER_KEY_BYTES {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "queue order key bytes",
            ));
        }
        self.key_bytes = self
            .key_bytes
            .checked_add(value.len())
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.key_bytes > self.limits.flow.max_key_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded(self.key_limit_label));
        }
        self.usize(value.len())?;
        self.raw(value)
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
    scanning_dispatch: bool,
    key_limit_label: &'static str,
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
            scanning_dispatch: false,
            key_limit_label: "aggregate key byte",
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
        if self.scanning_dispatch {
            return Ok(Vec::new());
        }
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
    fn scan_key(&mut self) -> WResult {
        let n = self.usize()?;
        self.key_bytes = self
            .key_bytes
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.key_bytes > self.limits.flow.max_key_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded(self.key_limit_label));
        }
        std::str::from_utf8(self.raw(n)?).map_err(|_| FlowCheckpointWireError::InvalidUtf8)?;
        Ok(())
    }
    fn scan_opaque_key(&mut self) -> WResult {
        let n = self.usize()?;
        if n == 0 {
            return Err(FlowCheckpointWireError::InvalidValue(
                "stable queue-order key",
            ));
        }
        if n > MAX_WIRE_QUEUE_ORDER_KEY_BYTES {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "queue order key bytes",
            ));
        }
        self.key_bytes = self
            .key_bytes
            .checked_add(n)
            .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        if self.key_bytes > self.limits.flow.max_key_bytes {
            return Err(FlowCheckpointWireError::LimitExceeded(self.key_limit_label));
        }
        self.raw(n)?;
        Ok(())
    }
    fn opaque_key(&mut self) -> Result<Vec<u8>, FlowCheckpointWireError> {
        let n = self.usize()?;
        if n == 0 {
            return Err(FlowCheckpointWireError::InvalidValue(
                "stable queue-order key",
            ));
        }
        if n > MAX_WIRE_QUEUE_ORDER_KEY_BYTES {
            return Err(FlowCheckpointWireError::LimitExceeded(
                "queue order key bytes",
            ));
        }
        let bytes = self.raw(n)?;
        let mut value = Vec::new();
        value
            .try_reserve_exact(n)
            .map_err(|_| FlowCheckpointWireError::AllocationFailed)?;
        value.extend_from_slice(bytes);
        Ok(value)
    }
    fn scan_payload(&mut self) -> WResult {
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
        self.raw(n)?;
        Ok(())
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

fn scan_flow_v2(bytes: &[u8], limits: FlowCheckpointWireLimits) -> WResult {
    let mut r = Reader::new(bytes, limits)?;
    if r.raw(8)? != MAGIC_V2 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    if r.u32()? != FLOW_CHECKPOINT_VERSION_V2 {
        return Err(FlowCheckpointWireError::InvalidValue(
            "Flow checkpoint version",
        ));
    }
    if r.u64()? == 0 || r.usize()? == 0 {
        return Err(FlowCheckpointWireError::InvalidValue("zero Flow limit"));
    }
    r.u64()?;
    scan_opt_time(&mut r)?;
    r.u64()?;
    scan_halt(&mut r)?;
    scan_scheduler(&mut r)?;
    scan_world(&mut r)?;
    let cap = r.limits.flow.max_resources;
    scan_vec(&mut r, cap, "resource", 12, |r| {
        get_resource(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_requests;
    scan_vec(&mut r, cap, "request", 12, |r| {
        get_request(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_actors;
    scan_vec(&mut r, cap, "actor", 12, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_actors;
    scan_vec(&mut r, cap, "actor domain", 24, |r| {
        get_entity(r)?;
        get_entity(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_works;
    scan_vec(&mut r, cap, "work registration", 30, scan_work_registration)?;
    scan_registration_vec(&mut r, 20, scan_context_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_builtin_stores(&mut r)?;
    scan_registration_vec(&mut r, 28, scan_context_store)?;
    scan_registration_vec(&mut r, 12, |r| {
        r.scan_key()?;
        r.u32()?;
        Ok(())
    })?;
    scan_registration_vec(&mut r, 28, scan_restart_store)?;
    let n = r.count(r.limits.flow.max_commands, "command")?;
    r.require_min_items(n, 13)?;
    let mut gates = 0usize;
    let mut previous = None;
    for _ in 0..n {
        let event = get_event(&mut r)?;
        if previous.is_some_and(|p| p >= event) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "noncanonical order or duplicate entry",
            ));
        }
        previous = Some(event);
        if scan_command_v2(&mut r)? {
            gates += 1;
        }
    }
    if gates > r.limits.flow.max_commands {
        return Err(FlowCheckpointWireError::LimitExceeded("pending command"));
    }
    r.pending_count(gates)?;
    let cap = r.limits.flow.max_notifications;
    scan_vec(&mut r, cap, "notification", 126, |r| {
        get_event(r)?;
        scan_notification(r)
    })?;
    let n = r.count(r.limits.flow.max_pending_operations, "pending release")?;
    r.pending_count(n)?;
    scan_vec_counted(&mut r, n, 20, |r| {
        get_lease(r)?;
        Ok(())
    })?;
    let n = r.count(r.limits.flow.max_pending_operations, "pending despawn")?;
    r.pending_count(n)?;
    scan_vec_counted(&mut r, n, 12, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    let n = r.count(r.limits.flow.max_resources, "resource grant flag")?;
    r.require_min_items(n, 13)?;
    let mut prev = None;
    for _ in 0..n {
        let id = get_resource(&mut r)?;
        if prev.is_some_and(|p| p >= id) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "noncanonical order or duplicate entry",
            ));
        }
        prev = Some(id);
        r.bool()?;
    }
    scan_opt_time(&mut r)?;
    r.finish()
}

fn scan_vec(
    r: &mut Reader<'_>,
    cap: usize,
    what: &'static str,
    min: usize,
    f: impl FnMut(&mut Reader<'_>) -> WResult,
) -> WResult {
    let n = r.count(cap, what)?;
    scan_vec_counted(r, n, min, f)
}
fn scan_vec_counted(
    r: &mut Reader<'_>,
    n: usize,
    min: usize,
    mut f: impl FnMut(&mut Reader<'_>) -> WResult,
) -> WResult {
    n.checked_mul(min)
        .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    r.require_min_items(n, min)?;
    for _ in 0..n {
        f(r)?;
    }
    Ok(())
}
fn scan_registration_vec(
    r: &mut Reader<'_>,
    min: usize,
    f: impl FnMut(&mut Reader<'_>) -> WResult,
) -> WResult {
    let n = r.count(r.limits.flow.max_registrations, "registration")?;
    r.registration_count(n)?;
    scan_vec_counted(r, n, min, f)
}
fn scan_opt_time(r: &mut Reader<'_>) -> WResult {
    scan_option(r, |r| {
        get_time(r)?;
        Ok(())
    })
}
fn scan_option(r: &mut Reader<'_>, f: impl FnOnce(&mut Reader<'_>) -> WResult) -> WResult {
    match r.u8()? {
        0 => Ok(()),
        1 => f(r),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn scan_scheduler(r: &mut Reader<'_>) -> WResult {
    r.u16()?;
    get_time(r)?;
    r.u64()?;
    r.u32()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    let cap = r.limits.flow.max_scheduler_entries;
    scan_vec(r, cap, "scheduler entry", 46, |r| {
        scan_schedule(r)?;
        get_event(r)?;
        r.u64()?;
        r.bool()?;
        Ok(())
    })
}
fn scan_schedule(r: &mut Reader<'_>) -> WResult {
    get_time(r)?;
    r.i32()?;
    scan_option(r, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    get_kind(r)?;
    Ok(())
}
fn scan_world(r: &mut Reader<'_>) -> WResult {
    r.u32()?;
    let cap = r.limits.flow.max_entities;
    scan_vec(r, cap, "world slot", 5, |r| {
        r.u32()?;
        r.bool()?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_entities;
    scan_vec(r, cap, "free index", 8, |r| {
        r.u64()?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_entities;
    scan_vec(r, cap, "live entity", 12, |r| {
        get_entity(r)?;
        Ok(())
    })
}
fn scan_halt(r: &mut Reader<'_>) -> WResult {
    scan_option(r, |r| {
        get_time(r)?;
        r.u64()?;
        r.u64()?;
        get_event(r)?;
        get_time(r)?;
        r.i32()?;
        r.u64()?;
        scan_option(r, |r| {
            get_entity(r)?;
            Ok(())
        })?;
        get_kind(r)?;
        Ok(())
    })
}
fn scan_work_registration(r: &mut Reader<'_>) -> WResult {
    get_entity(r)?;
    r.scan_key()?;
    r.scan_key()?;
    scan_option(r, |r| r.scan_key())?;
    scan_option(r, |r| {
        r.u32()?;
        Ok(())
    })
}
fn scan_context_registration(r: &mut Reader<'_>) -> WResult {
    r.scan_key()?;
    r.scan_key()?;
    r.u32()?;
    Ok(())
}
fn scan_callback_registration(r: &mut Reader<'_>) -> WResult {
    r.scan_key()?;
    r.scan_key()?;
    scan_option(r, |r| {
        get_kind(r)?;
        Ok(())
    })?;
    r.u8()?;
    scan_registration_vec(r, 20, |r| {
        r.scan_key()?;
        r.scan_key()?;
        r.u32()?;
        Ok(())
    })
}
fn scan_context_store(r: &mut Reader<'_>) -> WResult {
    r.scan_key()?;
    r.u32()?;
    let sparse = r.usize()?;
    let n = r.count(r.limits.flow.max_component_rows, "context row")?;
    r.component_dimensions(n, sparse)?;
    scan_vec_counted(r, n, 20, |r| {
        get_entity(r)?;
        r.scan_payload()
    })
}
fn scan_restart_store(r: &mut Reader<'_>) -> WResult {
    r.scan_key()?;
    r.u32()?;
    let sparse = r.usize()?;
    let n = r.count(r.limits.flow.max_component_rows, "restart row")?;
    r.component_dimensions(n, sparse)?;
    scan_vec_counted(r, n, 28, |r| {
        get_entity(r)?;
        r.scan_key()?;
        r.scan_payload()
    })
}
fn scan_notification(r: &mut Reader<'_>) -> WResult {
    let kind = r.u8()?;
    if kind > 1 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    get_work(r)?;
    scan_transition(r)?;
    scan_progress(r)?;
    get_event(r)?;
    r.u32()?;
    Ok(())
}
fn scan_transition(r: &mut Reader<'_>) -> WResult {
    match r.u8()? {
        0..=9 => Ok(()),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn scan_progress(r: &mut Reader<'_>) -> WResult {
    for _ in 0..4 {
        get_duration(r)?;
    }
    r.u64()?;
    r.u64()?;
    match r.u8()? {
        0..=6 => (),
        _ => return Err(FlowCheckpointWireError::InvalidTag),
    }
    scan_opt_time(r)?;
    scan_opt_time(r)?;
    r.bool()?;
    Ok(())
}
fn scan_strategy(r: &mut Reader<'_>) -> WResult {
    match r.u8()? {
        0..=2 => Ok(()),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn scan_request_state(r: &mut Reader<'_>) -> WResult {
    match r.u8()? {
        0..=8 => Ok(()),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn scan_domain_control(r: &mut Reader<'_>) -> WResult {
    match r.u8()? {
        0..=1 => Ok(()),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn scan_option_request(r: &mut Reader<'_>) -> WResult {
    scan_option(r, |r| {
        get_request(r)?;
        Ok(())
    })
}
fn scan_option_work(r: &mut Reader<'_>) -> WResult {
    scan_option(r, |r| {
        get_work(r)?;
        Ok(())
    })
}
fn scan_option_lease(r: &mut Reader<'_>) -> WResult {
    scan_option(r, |r| {
        get_lease(r)?;
        Ok(())
    })
}
fn scan_builtin_stores(r: &mut Reader<'_>) -> WResult {
    scan_component_option(r, 16, |r| {
        r.u32()?;
        Ok(())
    })?;
    scan_component_option(r, 20, scan_queue)?;
    scan_component_option(r, 52, scan_resource_request)?;
    scan_component_option(r, 28, scan_deadline)?;
    scan_component_option(r, 28, scan_preempting)?;
    scan_component_option(r, 20, scan_active_allocations)?;
    scan_component_option(r, 37, scan_work_spec)?;
    scan_component_option(r, 13, scan_role)?;
    scan_component_option(r, 96, scan_progress)
}
fn scan_component_option(
    r: &mut Reader<'_>,
    min: usize,
    mut item: impl FnMut(&mut Reader<'_>) -> WResult,
) -> WResult {
    scan_option(r, |r| {
        r.u32()?;
        let sparse = r.usize()?;
        let n = r.count(r.limits.flow.max_component_rows, "component row")?;
        r.component_dimensions(n, sparse)?;
        scan_vec_counted(r, n, min, |r| {
            get_entity(r)?;
            item(r)
        })
    })
}
fn scan_priority(r: &mut Reader<'_>) -> WResult<PriorityKey> {
    Ok(PriorityKey {
        level: r.i32()?,
        enqueue_sequence: r.u64()?,
        request: get_request(r)?,
    })
}
fn scan_queue(r: &mut Reader<'_>) -> WResult {
    let n = r.count(r.limits.flow.max_requests, "queue key")?;
    r.require_min_items(n, 24)?;
    let mut previous = None;
    for _ in 0..n {
        let key = scan_priority(r)?;
        if previous.is_some_and(|p| p >= key) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "queue key order or duplicate",
            ));
        }
        previous = Some(key);
    }
    Ok(())
}
fn scan_resource_request(r: &mut Reader<'_>) -> WResult {
    get_resource(r)?;
    get_entity(r)?;
    scan_request_state(r)?;
    scan_option(r, |r| {
        r.u64()?;
        Ok(())
    })?;
    scan_option_lease(r)?;
    r.i32()?;
    scan_option_work(r)?;
    get_time(r)?;
    scan_opt_time(r)?;
    r.bool()?;
    r.bool()?;
    scan_option(r, scan_strategy)
}
fn scan_deadline(r: &mut Reader<'_>) -> WResult {
    r.usize()?;
    scan_vec(r, r.limits.flow.max_requests, "deadline entry", 40, |r| {
        get_time(r)?;
        scan_priority(r)?;
        Ok(())
    })
}
fn scan_preempting(r: &mut Reader<'_>) -> WResult {
    r.usize()?;
    scan_vec(r, r.limits.flow.max_requests, "preemption key", 24, |r| {
        scan_priority(r)?;
        Ok(())
    })
}
fn scan_allocation(r: &mut Reader<'_>) -> WResult {
    get_lease(r)?;
    get_request(r)?;
    get_entity(r)?;
    scan_option_work(r)?;
    r.i32()?;
    get_time(r)?;
    get_time(r)?;
    scan_opt_time(r)
}
fn scan_active_allocations(r: &mut Reader<'_>) -> WResult {
    let n = r.count(r.limits.flow.max_requests, "allocation")?;
    r.require_min_items(n, 102)?;
    let mut previous = None;
    for _ in 0..n {
        let key = get_lease(r)?;
        if previous.is_some_and(|p| p >= key) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "allocation key order",
            ));
        }
        previous = Some(key);
        scan_allocation(r)?;
    }
    Ok(())
}
fn scan_work_spec(r: &mut Reader<'_>) -> WResult {
    get_entity(r)?;
    get_duration(r)?;
    r.scan_key()?;
    scan_option_request(r)
}
fn scan_role(r: &mut Reader<'_>) -> WResult {
    match r.u8()? {
        0 => Ok(()),
        1 => {
            get_entity(r)?;
            get_kind(r)?;
            Ok(())
        }
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}
fn scan_command_v2(r: &mut Reader<'_>) -> Result<bool, FlowCheckpointWireError> {
    Ok(match r.u8()? {
        0 => {
            get_request(r)?;
            false
        }
        1 => {
            get_lease(r)?;
            false
        }
        2 => {
            get_resource(r)?;
            r.u32()?;
            false
        }
        3 => {
            get_resource(r)?;
            false
        }
        4 => {
            get_entity(r)?;
            false
        }
        5 | 6 => {
            get_request(r)?;
            false
        }
        7 => {
            get_request(r)?;
            r.i32()?;
            false
        }
        8 => {
            get_request(r)?;
            get_lease(r)?;
            r.u64()?;
            get_time(r)?;
            false
        }
        9 => false,
        10 => {
            get_work(r)?;
            get_kind(r)?;
            false
        }
        11 => {
            get_work(r)?;
            get_kind(r)?;
            scan_domain_control(r)?;
            false
        }
        12 => {
            get_resource(r)?;
            r.bool()?;
            true
        }
        _ => return Err(FlowCheckpointWireError::InvalidTag),
    })
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

impl FlowCheckpointV2 {
    /// Encode the complete V2 image using canonical bounded little-endian fields.
    #[doc(hidden)]
    pub fn encode_wire_v2(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<Vec<u8>, FlowCheckpointWireError> {
        if self.version != FLOW_CHECKPOINT_VERSION_V2 {
            return Err(FlowCheckpointWireError::InvalidValue(
                "Flow checkpoint version",
            ));
        }
        preflight_counts_v2(self, limits)?;
        let mut measure = Writer::measure(limits);
        encode_flow_v2(self, &mut measure)?;
        validate_canonical_order_v2(self)?;
        let mut writer = Writer::writing(limits, measure.len)?;
        encode_flow_v2(self, &mut writer)?;
        Ok(writer.finish())
    }

    /// Scan the complete payload before creating any owned DTO collections.
    #[doc(hidden)]
    pub fn decode_wire_v2(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        scan_flow_v2(bytes, limits)?;
        #[cfg(test)]
        OWNED_DECODE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        let mut reader = Reader::new(bytes, limits)?;
        let image = decode_flow_v2(&mut reader)?;
        reader.finish()?;
        preflight_counts_v2(&image, limits)?;
        validate_canonical_order_v2(&image)?;
        Ok(image)
    }
}

impl FlowCheckpointImage {
    /// Encode while preserving the captured native format and all its fields.
    #[doc(hidden)]
    pub fn encode_wire(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<Vec<u8>, FlowCheckpointWireError> {
        match self {
            Self::V1(image) => image.encode_wire_v1(limits),
            Self::V2(image) => image.encode_wire_v2(limits),
            Self::V3(image) => image.encode_wire_v3(limits),
            Self::V4(image) => image.encode_wire_v4(limits),
        }
    }

    /// Dispatch only by the three known wire magics.
    #[doc(hidden)]
    pub fn decode_wire(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        match bytes.get(..8) {
            Some(magic) if magic == MAGIC => {
                FlowCheckpointV1::decode_wire_v1(bytes, limits).map(Self::V1)
            }
            Some(magic) if magic == MAGIC_V2 => {
                FlowCheckpointV2::decode_wire_v2(bytes, limits).map(Self::V2)
            }
            Some(magic) if magic == MAGIC_V3 => {
                FlowCheckpointV3::decode_wire_v3(bytes, limits).map(Self::V3)
            }
            Some(magic) if magic == MAGIC_V4 => {
                FlowCheckpointV4::decode_wire_v4(bytes, limits).map(Self::V4)
            }
            Some(_) => Err(FlowCheckpointWireError::InvalidTag),
            None => Err(FlowCheckpointWireError::Truncated),
        }
    }
}

#[cfg(test)]
thread_local! {
    static OWNED_DECODE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CANONICAL_VALIDATION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl FlowDispatch {
    /// Measure a complete dispatch without allocating an output buffer.
    #[doc(hidden)]
    pub fn encoded_wire_len_v1(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<usize, FlowCheckpointWireError> {
        let mut writer = Writer::measure(limits);
        writer.raw(MAGIC)?;
        writer.u16(WIRE_SCHEMA)?;
        encode_dispatch(self, &mut writer)?;
        Ok(writer.len)
    }

    /// Scan every dispatch field and nested collection under limits without
    /// allocating records/admissions. Canonical scalar tags, booleans, lengths,
    /// integer conversions and trailing bytes are checked before owner decode.
    #[doc(hidden)]
    pub fn preflight_wire_v1(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<(), FlowCheckpointWireError> {
        let mut reader = Reader::new(bytes, limits)?;
        reader.scanning_dispatch = true;
        if reader.raw(MAGIC.len())? != MAGIC {
            return Err(FlowCheckpointWireError::InvalidTag);
        }
        let schema = reader.u16()?;
        if schema != WIRE_SCHEMA {
            return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
        }
        let _ = decode_dispatch(&mut reader)?;
        reader.finish()
    }

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
        Self::preflight_wire_v1(bytes, limits)?;
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

fn preflight_counts_v2(image: &FlowCheckpointV2, limits: FlowCheckpointWireLimits) -> WResult {
    let f = limits.flow;
    let mut rows = 0usize;
    let mut sparse = 0usize;
    macro_rules! store {
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
    store!(image.builtins.capacities.as_ref());
    store!(image.builtins.queues.as_ref());
    store!(image.builtins.requests.as_ref());
    store!(image.builtins.deadlines.as_ref());
    store!(image.builtins.preempting.as_ref());
    store!(image.builtins.allocations.as_ref());
    store!(image.builtins.work_specs.as_ref());
    store!(image.builtins.work_roles.as_ref());
    store!(image.builtins.work_progress.as_ref());
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
    let pending = image
        .pending_releases
        .len()
        .checked_add(image.pending_despawns.len())
        .and_then(|n| {
            n.checked_add(
                image
                    .commands
                    .iter()
                    .filter(|(_, c)| matches!(c, FlowCommandV2::GrantEnabled(..)))
                    .count(),
            )
        })
        .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    if image.world.slots.len() > f.max_entities
        || image.world.free_indices.len() > f.max_entities
        || image.world.live_entities.len() > f.max_entities
        || image.scheduler.entries.len() > f.max_scheduler_entries
        || image.resources.len() > f.max_resources
        || image.resource_grant_flags.len() > f.max_resources
        || image.requests.len() > f.max_requests
        || image.actors.len() > f.max_actors
        || image.work_registrations.len() > f.max_works
        || image.actor_domains.len() > f.max_actors
        || image.commands.len() > f.max_commands
        || image.notifications.len() > f.max_notifications
        || pending > f.max_pending_operations
        || rows > f.max_component_rows
        || sparse > f.max_sparse_slots
        || registrations > f.max_registrations
    {
        return Err(FlowCheckpointWireError::LimitExceeded("Flow collection"));
    }
    Ok(())
}

fn validate_canonical_order_v2(i: &FlowCheckpointV2) -> WResult {
    #[cfg(test)]
    CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
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
        && strict_by(&i.scheduler.entries, |a, b| a.sequence.cmp(&b.sequence))
        && strict_by(&i.resource_grant_flags, |a, b| a.0.cmp(&b.0));
    if !ordered {
        return Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry",
        ));
    }
    if i.resources.len() != i.resource_grant_flags.len()
        || !i
            .resources
            .iter()
            .zip(&i.resource_grant_flags)
            .all(|(r, (f, _))| r == f)
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "resource grant flags",
        ));
    }
    if i.started_frontier != (i.scheduler.dispatched_events > 0).then_some(i.scheduler.now) {
        return Err(FlowCheckpointWireError::InvalidValue("started frontier"));
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
        ($s:expr) => {
            if let Some(s) = $s {
                ensure_unique(s.rows.iter().map(|(id, _)| *id))?;
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
    for s in &i.context_stores {
        ensure_unique(s.rows.iter().map(|(id, _)| *id))?;
    }
    for s in &i.restart_stores {
        ensure_unique(s.rows.iter().map(|(id, _, _)| *id))?;
    }
    if i.notifications.iter().any(|(_, n)| n.kind > 1) {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    if let Some(s) = &i.builtins.queues {
        for (_, q) in &s.rows {
            if !q.requests.iter().is_sorted() {
                return Err(FlowCheckpointWireError::InvalidValue("queue key order"));
            }
        }
    }
    if let Some(s) = &i.builtins.deadlines {
        for (_, v) in &s.rows {
            if !v.entries.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue("deadline key order"));
            }
        }
    }
    if let Some(s) = &i.builtins.preempting {
        for (_, v) in &s.rows {
            if !v.keys.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "preemption key order",
                ));
            }
        }
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

fn encode_flow_v2(i: &FlowCheckpointV2, w: &mut Writer) -> WResult {
    w.raw(MAGIC_V2)?;
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
        put_command_v2(w, *c)
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
    w.u64(i.next_lease)?;
    put_vec(w, &i.resource_grant_flags, |w, (resource, enabled)| {
        put_resource(w, *resource)?;
        w.bool(*enabled)
    })?;
    put_option_time(w, i.started_frontier)
}

fn decode_flow_v2(r: &mut Reader<'_>) -> WResult<FlowCheckpointV2> {
    if r.raw(MAGIC_V2.len())? != MAGIC_V2 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    let version = r.u32()?;
    if version != FLOW_CHECKPOINT_VERSION_V2 {
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
        let event = get_event(r)?;
        let command = get_command_v2(r)?;
        if matches!(command, FlowCommandV2::GrantEnabled(..)) {
            r.pending_count(1)?;
        }
        Ok((event, command))
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
    let created = r.u64()?;
    let destroyed = r.u64()?;
    let scheduled = r.u64()?;
    let next_admission = r.u64()?;
    let next_lease = r.u64()?;
    let resource_grant_flags = get_vec(
        r,
        r.limits.flow.max_resources,
        "resource grant flag",
        13,
        |r| Ok((get_resource(r)?, r.bool()?)),
    )?;
    let started_frontier = get_option_time(r)?;
    Ok(FlowCheckpointV2 {
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
        created,
        destroyed,
        scheduled,
        next_admission,
        next_lease,
        resource_grant_flags,
        started_frontier,
    })
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
        let value = f(r)?;
        if !r.scanning_dispatch {
            v.push(value);
        }
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
                let admission = get_admission(r)?;
                if !r.scanning_dispatch {
                    items.push(admission);
                }
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
fn put_command_v2(w: &mut Writer, c: FlowCommandV2) -> WResult {
    match c {
        FlowCommandV2::Submit(x) => {
            w.u8(0)?;
            put_request(w, x)
        }
        FlowCommandV2::Release(x) => {
            w.u8(1)?;
            put_lease(w, x)
        }
        FlowCommandV2::Capacity(x, n) => {
            w.u8(2)?;
            put_resource(w, x)?;
            w.u32(n)
        }
        FlowCommandV2::Remove(x) => {
            w.u8(3)?;
            put_resource(w, x)
        }
        FlowCommandV2::Despawn(x) => {
            w.u8(4)?;
            put_entity(w, x)
        }
        FlowCommandV2::Deadline(x) => {
            w.u8(5)?;
            put_request(w, x)
        }
        FlowCommandV2::Cancel(x) => {
            w.u8(6)?;
            put_request(w, x)
        }
        FlowCommandV2::Reprioritize(x, n) => {
            w.u8(7)?;
            put_request(w, x)?;
            w.i32(n)
        }
        FlowCommandV2::Completion(req, lease, rev, at) => {
            w.u8(8)?;
            put_request(w, req)?;
            put_lease(w, lease)?;
            w.u64(rev)?;
            put_time(w, at)
        }
        FlowCommandV2::Notify => w.u8(9),
        FlowCommandV2::Domain(work, kind) => {
            w.u8(10)?;
            put_work(w, work)?;
            put_kind(w, kind)
        }
        FlowCommandV2::DomainControl(work, kind, action) => {
            w.u8(11)?;
            put_work(w, work)?;
            put_kind(w, kind)?;
            put_domain_control(w, action)
        }
        FlowCommandV2::GrantEnabled(resource, enabled) => {
            w.u8(12)?;
            put_resource(w, resource)?;
            w.bool(enabled)
        }
    }
}
fn get_command_v2(r: &mut Reader<'_>) -> WResult<FlowCommandV2> {
    match r.u8()? {
        0 => Ok(FlowCommandV2::Submit(get_request(r)?)),
        1 => Ok(FlowCommandV2::Release(get_lease(r)?)),
        2 => Ok(FlowCommandV2::Capacity(get_resource(r)?, r.u32()?)),
        3 => Ok(FlowCommandV2::Remove(get_resource(r)?)),
        4 => Ok(FlowCommandV2::Despawn(get_entity(r)?)),
        5 => Ok(FlowCommandV2::Deadline(get_request(r)?)),
        6 => Ok(FlowCommandV2::Cancel(get_request(r)?)),
        7 => Ok(FlowCommandV2::Reprioritize(get_request(r)?, r.i32()?)),
        8 => Ok(FlowCommandV2::Completion(
            get_request(r)?,
            get_lease(r)?,
            r.u64()?,
            get_time(r)?,
        )),
        9 => Ok(FlowCommandV2::Notify),
        10 => Ok(FlowCommandV2::Domain(get_work(r)?, get_kind(r)?)),
        11 => Ok(FlowCommandV2::DomainControl(
            get_work(r)?,
            get_kind(r)?,
            get_domain_control(r)?,
        )),
        12 => Ok(FlowCommandV2::GrantEnabled(get_resource(r)?, r.bool()?)),
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

#[cfg(test)]
mod wire_v2_tests {
    use super::*;
    thread_local! { static DECODER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

    fn codecs() -> FlowCheckpointCodecs {
        FlowCheckpointCodecs::new()
    }
    fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        if remaining < 8 {
            return Err(FlowCheckpointCodecError("payload budget".into()));
        }
        Ok(value.to_le_bytes().to_vec())
    }
    fn decode_u64(
        bytes: &[u8],
        _: &FlowCheckpointRebindV1,
    ) -> Result<u64, FlowCheckpointCodecError> {
        DECODER_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        let value: [u8; 8] = bytes
            .try_into()
            .map_err(|_| FlowCheckpointCodecError("u64 payload".into()))?;
        Ok(u64::from_le_bytes(value))
    }
    fn restart_factory(template: &u64) -> u64 {
        *template + 1
    }

    fn rich_image() -> (FlowCheckpointV2, FlowCheckpointCodecs) {
        let mut codecs = codecs();
        codecs
            .register_context::<u64>("scan.context", 1, encode_u64, decode_u64)
            .unwrap();
        codecs
            .register_restart_template::<u64, u64>("scan.restart", 1, encode_u64, decode_u64)
            .unwrap();
        codecs
            .register_restart_factory::<u64, u64>("scan.restart", "scan.factory", restart_factory)
            .unwrap();
        let mut runtime = FlowRuntime::new();
        let resource = runtime.create_resource(2).unwrap();
        let owners: Vec<_> = (0..5).map(|_| runtime.spawn_actor().unwrap()).collect();
        let mut requests = Vec::new();
        for (index, &owner) in owners.iter().take(4).enumerate() {
            let work = if index == 0 {
                runtime
                    .create_restartable_work(
                        owner,
                        SimDuration::from_ticks(12),
                        "scan.restart",
                        100_u64,
                        restart_factory,
                    )
                    .unwrap()
            } else {
                runtime
                    .create_work(
                        owner,
                        SimDuration::from_ticks(12),
                        "scan.context",
                        index as u64,
                    )
                    .unwrap()
            };
            requests.push(
                runtime
                    .acquire(resource)
                    .owner(owner)
                    .timed_work(work)
                    .priority(index as i32)
                    .submit()
                    .unwrap(),
            );
        }
        for _ in 0..16 {
            if requests
                .iter()
                .all(|request| runtime.request(*request).unwrap().state != RequestState::Pending)
            {
                break;
            }
            runtime.step().unwrap().unwrap();
        }
        let lease = requests
            .iter()
            .find_map(|request| runtime.request(*request).ok().and_then(|r| r.lease))
            .unwrap();
        runtime.release(lease, SimTime::from_ticks(12)).unwrap();
        runtime
            .despawn_actor_at_with_scheduler_priority(owners[4], SimTime::from_ticks(13), 0)
            .unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(14), 0)
            .unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(15), 0)
            .unwrap();
        let image = runtime
            .capture_checkpoint_v2(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert!(image
            .builtins
            .queues
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, q)| q.requests.len() >= 2)));
        assert!(image
            .builtins
            .allocations
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, a)| a.leases.len() >= 2)));
        assert!(image.restart_stores.iter().any(|s| !s.rows.is_empty()));
        (image, codecs)
    }

    fn component_totals(i: &FlowCheckpointV2) -> (usize, usize) {
        let mut rows = 0usize;
        let mut sparse = 0usize;
        macro_rules! add {
            ($s:expr) => {
                if let Some(s) = $s {
                    rows += s.rows.len();
                    sparse += s.sparse_slots;
                }
            };
        }
        add!(i.builtins.capacities.as_ref());
        add!(i.builtins.queues.as_ref());
        add!(i.builtins.requests.as_ref());
        add!(i.builtins.deadlines.as_ref());
        add!(i.builtins.preempting.as_ref());
        add!(i.builtins.allocations.as_ref());
        add!(i.builtins.work_specs.as_ref());
        add!(i.builtins.work_roles.as_ref());
        add!(i.builtins.work_progress.as_ref());
        for s in &i.context_stores {
            rows += s.rows.len();
            sparse += s.sparse_slots;
        }
        for s in &i.restart_stores {
            rows += s.rows.len();
            sparse += s.sparse_slots;
        }
        (rows, sparse)
    }

    #[test]
    fn v2_wire_roundtrips_all_command_tags_and_both_boolean_values() {
        let mut runtime = FlowRuntime::new();
        let resource = runtime.create_resource(1).unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
            .unwrap();
        let mut image = runtime
            .capture_checkpoint_v2(&codecs(), FlowCheckpointLimits::default())
            .unwrap();
        let entity = EntityId::new(55, 3);
        let request = RequestId(entity);
        let work = WorkId(entity);
        let lease = LeaseId {
            request,
            revision: 7,
        };
        let commands = [
            FlowCommandV2::Submit(request),
            FlowCommandV2::Release(lease),
            FlowCommandV2::Capacity(resource, 3),
            FlowCommandV2::Remove(resource),
            FlowCommandV2::Despawn(entity),
            FlowCommandV2::Deadline(request),
            FlowCommandV2::Cancel(request),
            FlowCommandV2::Reprioritize(request, -4),
            FlowCommandV2::Completion(request, lease, 9, SimTime::from_ticks(4)),
            FlowCommandV2::Notify,
            FlowCommandV2::Domain(work, EventKind::custom(771)),
            FlowCommandV2::DomainControl(work, EventKind::custom(771), FlowDomainControl::Pause),
            FlowCommandV2::GrantEnabled(resource, false),
            FlowCommandV2::GrantEnabled(resource, true),
            FlowCommandV2::DomainControl(work, EventKind::custom(771), FlowDomainControl::Resume),
        ];
        image.commands = commands
            .into_iter()
            .enumerate()
            .map(|(n, c)| (EventId::new(n as u64, 0), c))
            .collect();
        image.started_frontier = None;
        let bytes = image
            .encode_wire_v2(FlowCheckpointWireLimits::default())
            .unwrap();
        let decoded =
            FlowCheckpointV2::decode_wire_v2(&bytes, FlowCheckpointWireLimits::default()).unwrap();
        assert_eq!(decoded, image);
        assert_eq!(
            decoded
                .encode_wire_v2(FlowCheckpointWireLimits::default())
                .unwrap(),
            bytes
        );
    }

    #[test]
    fn complete_scan_rejects_last_field_and_trailing_bytes_before_ownership() {
        let (image, _) = rich_image();
        assert!(image
            .context_stores
            .iter()
            .flat_map(|s| &s.rows)
            .any(|(_, p)| !p.is_empty()));
        assert!(image.restart_stores.iter().any(|s| !s.rows.is_empty()
            && s.rows
                .iter()
                .any(|(_, factory, payload)| factory == "scan.factory" && !payload.is_empty())));
        assert!(image
            .restart_store_manifest
            .iter()
            .any(|(key, _)| key == "scan.restart"));
        assert!(image
            .builtins
            .queues
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, q)| q.requests.len() >= 2)));
        assert!(image
            .builtins
            .allocations
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, a)| a.leases.len() >= 2)));
        assert_eq!(
            image
                .commands
                .iter()
                .filter(|(_, c)| matches!(c, FlowCommandV2::GrantEnabled(..)))
                .count(),
            2
        );
        assert_eq!(image.pending_releases.len(), 1);
        assert_eq!(image.pending_despawns.len(), 1);
        let bytes = image
            .encode_wire_v2(FlowCheckpointWireLimits::default())
            .unwrap();
        let mut bad_frontier = bytes.clone();
        let frontier_tag = bad_frontier.len() - 17;
        bad_frontier[frontier_tag] = 2;
        OWNED_DECODE_CALLS.with(|n| n.set(0));
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bad_frontier, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::InvalidTag
        );
        assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&trailing, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::TrailingBytes
        );
        assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bytes, FlowCheckpointWireLimits::default()).unwrap(),
            image
        );
        assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 1);
        let mut low_payload = FlowCheckpointWireLimits::default();
        low_payload.flow.max_payload_bytes = 0;
        OWNED_DECODE_CALLS.with(|n| n.set(0));
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bytes, low_payload).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("aggregate payload byte")
        );
        assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        let exact_registrations = image.contexts.len()
            + image.handlers.len()
            + image.continuations.len()
            + image.domains.len()
            + image.context_stores.len()
            + image.restart_store_manifest.len()
            + image.restart_stores.len()
            + image
                .handlers
                .iter()
                .chain(&image.continuations)
                .chain(&image.domains)
                .map(|c| c.callback_ids.len())
                .sum::<usize>();
        let exact_pending = image.pending_releases.len()
            + image.pending_despawns.len()
            + image
                .commands
                .iter()
                .filter(|(_, c)| matches!(c, FlowCommandV2::GrantEnabled(..)))
                .count();
        let exact_limits = FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_registrations: exact_registrations,
                max_pending_operations: exact_pending,
                ..FlowCheckpointLimits::default()
            },
            ..FlowCheckpointWireLimits::default()
        };
        assert_eq!(image.encode_wire_v2(exact_limits).unwrap(), bytes);
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bytes, exact_limits).unwrap(),
            image
        );
        let mut short_reg = exact_limits;
        short_reg.flow.max_registrations -= 1;
        assert!(matches!(
            image.encode_wire_v2(short_reg),
            Err(FlowCheckpointWireError::LimitExceeded(_))
        ));
        assert!(matches!(
            FlowCheckpointV2::decode_wire_v2(&bytes, short_reg),
            Err(FlowCheckpointWireError::LimitExceeded(_))
        ));
        let mut short_pending = exact_limits;
        short_pending.flow.max_pending_operations -= 1;
        assert!(matches!(
            image.encode_wire_v2(short_pending),
            Err(FlowCheckpointWireError::LimitExceeded(_))
        ));
        assert!(matches!(
            FlowCheckpointV2::decode_wire_v2(&bytes, short_pending),
            Err(FlowCheckpointWireError::LimitExceeded(_))
        ));
    }

    #[test]
    fn v2_wire_exact_limits_and_limit_plus_one_rejections() {
        let (image, _) = rich_image();
        let defaults = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v2(defaults).unwrap();
        let mut measured = Writer::measure(defaults);
        encode_flow_v2(&image, &mut measured).unwrap();
        let (rows, sparse) = component_totals(&image);
        let pending = image.pending_releases.len()
            + image.pending_despawns.len()
            + image
                .commands
                .iter()
                .filter(|(_, c)| matches!(c, FlowCommandV2::GrantEnabled(..)))
                .count();
        let registrations = image.contexts.len()
            + image.handlers.len()
            + image.continuations.len()
            + image.domains.len()
            + image.context_stores.len()
            + image.restart_store_manifest.len()
            + image.restart_stores.len()
            + image
                .handlers
                .iter()
                .chain(&image.continuations)
                .chain(&image.domains)
                .map(|c| c.callback_ids.len())
                .sum::<usize>();
        let values = [
            bytes.len(),
            measured.records,
            image.resources.len(),
            image.commands.len(),
            pending,
            registrations,
            measured.key_bytes,
            rows,
            sparse,
            measured.payload_bytes,
        ];
        assert!(values.iter().all(|v| *v > 0));
        let exact = FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_resources: values[2],
                max_commands: values[3],
                max_pending_operations: values[4],
                max_registrations: values[5],
                max_key_bytes: values[6],
                max_component_rows: values[7],
                max_sparse_slots: values[8],
                max_payload_bytes: values[9],
                ..FlowCheckpointLimits::default()
            },
            max_wire_bytes: values[0],
            max_total_records: values[1],
        };
        assert_eq!(image.encode_wire_v2(exact).unwrap(), bytes);
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bytes, exact).unwrap(),
            image
        );
        for (index, limit) in [
            "wire",
            "records",
            "resources",
            "commands",
            "pending",
            "registrations",
            "keys",
            "components",
            "sparse",
            "payload",
        ]
        .into_iter()
        .enumerate()
        {
            let mut too_small = exact;
            match index {
                0 => too_small.max_wire_bytes -= 1,
                1 => too_small.max_total_records -= 1,
                2 => too_small.flow.max_resources -= 1,
                3 => too_small.flow.max_commands -= 1,
                4 => too_small.flow.max_pending_operations -= 1,
                5 => too_small.flow.max_registrations -= 1,
                6 => too_small.flow.max_key_bytes -= 1,
                7 => too_small.flow.max_component_rows -= 1,
                8 => too_small.flow.max_sparse_slots -= 1,
                _ => too_small.flow.max_payload_bytes -= 1,
            }
            assert!(
                matches!(
                    image.encode_wire_v2(too_small),
                    Err(FlowCheckpointWireError::LimitExceeded(_))
                ),
                "encode {limit}"
            );
            assert!(
                matches!(
                    FlowCheckpointV2::decode_wire_v2(&bytes, too_small),
                    Err(FlowCheckpointWireError::LimitExceeded(_))
                ),
                "{limit}"
            );
        }
    }

    #[test]
    fn v2_wire_rejects_discriminators_tags_utf8_and_truncation() {
        let mut runtime = FlowRuntime::new();
        let resource = runtime.create_resource(1).unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
            .unwrap();
        let image = runtime
            .capture_checkpoint_v2(&codecs(), FlowCheckpointLimits::default())
            .unwrap();
        let bytes = image
            .encode_wire_v2(FlowCheckpointWireLimits::default())
            .unwrap();
        let mut unknown = bytes.clone();
        unknown[0] ^= 1;
        assert_eq!(
            FlowCheckpointImage::decode_wire(&unknown, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::InvalidTag
        );
        let mut schema = bytes.clone();
        schema[8] = 2;
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&schema, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::UnsupportedSchema(2)
        );
        let mut version = bytes.clone();
        version[10] = 3;
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&version, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::InvalidValue("Flow checkpoint version")
        );
        let mut truncated = bytes.clone();
        truncated.pop();
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&truncated, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::Truncated
        );
        let event = image.commands[0].0;
        let mut needle = Writer::writing(FlowCheckpointWireLimits::default(), 32).unwrap();
        put_event(&mut needle, event).unwrap();
        put_command_v2(&mut needle, image.commands[0].1).unwrap();
        let needle = needle.finish();
        let offset = bytes
            .windows(needle.len())
            .position(|w| w == needle)
            .unwrap();
        let mut bad_command = bytes.clone();
        bad_command[offset + 12] = 99;
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bad_command, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::InvalidTag
        );
        let mut bad_bool = bytes.clone();
        bad_bool[offset + 25] = 2;
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bad_bool, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::InvalidBoolean
        );
        let key = b"KFLOWV2";
        assert!(bytes.windows(key.len()).any(|w| w == key));
    }

    #[test]
    fn scan_rejects_invalid_utf8_without_ownership() {
        let mut runtime = FlowRuntime::new();
        let owner = runtime.spawn_actor().unwrap();
        runtime
            .create_work(owner, SimDuration::from_ticks(1), "utf8.key", 1_u64)
            .unwrap();
        let mut codecs = codecs();
        codecs
            .register_context::<u64>("utf8.key", 1, encode_u64, decode_u64)
            .unwrap();
        let resource = runtime.create_resource(1).unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
            .unwrap();
        let image = runtime
            .capture_checkpoint_v2(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        let mut bytes = image
            .encode_wire_v2(FlowCheckpointWireLimits::default())
            .unwrap();
        let key = b"utf8.key";
        let pos = bytes.windows(key.len()).position(|w| w == key).unwrap();
        bytes[pos] = 0xff;
        OWNED_DECODE_CALLS.with(|n| n.set(0));
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bytes, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::InvalidUtf8
        );
        assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 0);
    }

    #[test]
    fn measurement_limit_rejects_before_canonical_validation() {
        let (image, _) = rich_image();
        let defaults = FlowCheckpointWireLimits::default();
        let mut measured = Writer::measure(defaults);
        encode_flow_v2(&image, &mut measured).unwrap();
        assert!(measured.payload_bytes > 0);
        let mut too_small = defaults;
        too_small.flow.max_payload_bytes = measured.payload_bytes - 1;
        CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(0));
        assert!(matches!(
            image.encode_wire_v2(too_small),
            Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate payload byte"
            ))
        ));
        assert_eq!(CANONICAL_VALIDATION_CALLS.with(|calls| calls.get()), 0);
        CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(0));
        image.encode_wire_v2(defaults).unwrap();
        assert_eq!(CANONICAL_VALIDATION_CALLS.with(|calls| calls.get()), 1);
    }

    #[test]
    fn v2_encoder_rejects_incomplete_flags_and_wrong_frontier() {
        let (image, _) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let mut missing = image.clone();
        missing.resource_grant_flags.clear();
        assert_eq!(
            missing.encode_wire_v2(limits).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("resource grant flags")
        );
        let mut duplicate = image.clone();
        duplicate
            .resource_grant_flags
            .push(duplicate.resource_grant_flags[0]);
        assert_eq!(
            duplicate.encode_wire_v2(limits).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("noncanonical order or duplicate entry")
        );
        let mut wrong_resource = image.clone();
        wrong_resource.resource_grant_flags[0].0 = ResourceId(EntityId::new(u64::MAX, u32::MAX));
        assert_eq!(
            wrong_resource.encode_wire_v2(limits).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("resource grant flags")
        );
        let mut wrong_frontier = image.clone();
        wrong_frontier.started_frontier = None;
        assert_eq!(
            wrong_frontier.encode_wire_v2(limits).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("started frontier")
        );
        let (mut precommit, _) = rich_image();
        precommit.scheduler.dispatched_events = 0;
        precommit.started_frontier = None;
        let mut wrong_some = precommit.clone();
        wrong_some.started_frontier = Some(SimTime::ZERO);
        assert_eq!(
            wrong_some.encode_wire_v2(limits).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("started frontier")
        );
    }

    #[test]
    fn version_specific_readers_reject_opposite_magic_and_router_accepts_both() {
        let (v2, _) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let v2_bytes = v2.encode_wire_v2(limits).unwrap();
        assert_eq!(
            FlowCheckpointV1::decode_wire_v1(&v2_bytes, limits).unwrap_err(),
            FlowCheckpointWireError::InvalidTag
        );
        assert!(matches!(
            FlowCheckpointImage::decode_wire(&v2_bytes, limits).unwrap(),
            FlowCheckpointImage::V2(_)
        ));
        let mut runtime = FlowRuntime::new();
        runtime.create_resource(1).unwrap();
        let v1 = runtime
            .capture_checkpoint(&codecs(), FlowCheckpointLimits::default())
            .unwrap();
        let v1_bytes = v1.encode_wire_v1(limits).unwrap();
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&v1_bytes, limits).unwrap_err(),
            FlowCheckpointWireError::InvalidTag
        );
        assert!(matches!(
            FlowCheckpointImage::decode_wire(&v1_bytes, limits).unwrap(),
            FlowCheckpointImage::V1(_)
        ));
    }

    #[test]
    fn malformed_gate_scheduler_join_precedes_context_codecs_after_wire_decode() {
        let (image, codecs) = rich_image();
        let limits = FlowCheckpointLimits::default();
        let wire_limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v2(wire_limits).unwrap();
        let mut malformed = image.clone();
        let event = malformed
            .commands
            .iter()
            .find_map(|(event, command)| {
                matches!(command, FlowCommandV2::GrantEnabled(..)).then_some(*event)
            })
            .unwrap();
        malformed
            .scheduler
            .entries
            .iter_mut()
            .find(|entry| entry.id == event)
            .unwrap()
            .request
            .kind = EventKind::custom(u32::MAX);
        let malformed_bytes = malformed.encode_wire_v2(wire_limits).unwrap();
        let decoded = FlowCheckpointV2::decode_wire_v2(&malformed_bytes, wire_limits).unwrap();
        DECODER_CALLS.with(|calls| calls.set(0));
        assert!(matches!(
            FlowRuntime::restore_checkpoint_v2(decoded, &codecs, limits),
            Err(FlowCheckpointError::InvalidState(_))
        ));
        assert_eq!(DECODER_CALLS.with(|calls| calls.get()), 0);
        let valid = FlowCheckpointV2::decode_wire_v2(&bytes, wire_limits).unwrap();
        FlowRuntime::restore_checkpoint_v2(valid, &codecs, limits).unwrap();
        assert!(DECODER_CALLS.with(|calls| calls.get()) > 0);
    }

    #[test]
    fn nested_queue_and_allocation_order_corruption_is_rejected_by_nonowning_scan() {
        let (image, _) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v2(limits).unwrap();
        OWNED_DECODE_CALLS.with(|n| n.set(0));
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&bytes, limits).unwrap(),
            image
        );
        assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 1);
        // Locate an entire canonical nested payload uniquely in the complete image.
        // Replace it without changing lengths, then exercise the public scan boundary.
        let reject = |canonical: &[u8], replacement: &[u8], message| {
            assert_eq!(canonical.len(), replacement.len());
            let positions: Vec<_> = bytes
                .windows(canonical.len())
                .enumerate()
                .filter_map(|(offset, value)| (value == canonical).then_some(offset))
                .collect();
            assert_eq!(positions.len(), 1);
            let mut malformed = bytes.clone();
            let offset = positions[0];
            malformed[offset..offset + canonical.len()].copy_from_slice(replacement);
            OWNED_DECODE_CALLS.with(|n| n.set(0));
            assert_eq!(
                FlowCheckpointV2::decode_wire_v2(&malformed, limits).unwrap_err(),
                FlowCheckpointWireError::InvalidValue(message)
            );
            assert_eq!(OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        };
        let queue = image
            .builtins
            .queues
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|(_, q)| q)
            .find(|q| q.requests.len() >= 2)
            .unwrap();
        let keys: Vec<_> = queue.requests.iter().copied().collect();
        assert_eq!(keys.len(), 2);
        let encode_queue = |keys: &[PriorityKey]| {
            let mut writer = Writer::writing(limits, 128).unwrap();
            writer.count(keys.len()).unwrap();
            for key in keys {
                put_priority(&mut writer, *key).unwrap();
            }
            writer.finish()
        };
        let canonical = encode_queue(&keys);
        reject(
            &canonical,
            &encode_queue(&[keys[1], keys[0]]),
            "queue key order or duplicate",
        );
        reject(
            &canonical,
            &encode_queue(&[keys[0], keys[0]]),
            "queue key order or duplicate",
        );
        let active = image
            .builtins
            .allocations
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|(_, a)| a)
            .find(|a| a.leases.len() >= 2)
            .unwrap();
        let allocations: Vec<_> = active.leases.iter().collect();
        assert_eq!(allocations.len(), 2);
        let encode_allocations = |rows: &[(&LeaseId, &Allocation)]| {
            let mut writer = Writer::writing(limits, 512).unwrap();
            writer.count(rows.len()).unwrap();
            for (lease, allocation) in rows {
                put_lease(&mut writer, **lease).unwrap();
                put_allocation(&mut writer, allocation).unwrap();
            }
            writer.finish()
        };
        let canonical = encode_allocations(&allocations);
        reject(
            &canonical,
            &encode_allocations(&[allocations[1], allocations[0]]),
            "allocation key order",
        );
        // Keep payload widths/content unchanged: duplicate only the lease identity.
        reject(
            &canonical,
            &encode_allocations(&[allocations[0], (allocations[0].0, allocations[1].1)]),
            "allocation key order",
        );
    }

    #[test]
    fn scan_rejects_huge_collection_lengths_and_short_minimum_records() {
        let mut runtime = FlowRuntime::new();
        let resource = runtime.create_resource(1).unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
            .unwrap();
        let image = runtime
            .capture_checkpoint_v2(&codecs(), FlowCheckpointLimits::default())
            .unwrap();
        let bytes = image
            .encode_wire_v2(FlowCheckpointWireLimits::default())
            .unwrap();
        let mut r = Reader::new(&bytes, FlowCheckpointWireLimits::default()).unwrap();
        r.raw(8).unwrap();
        r.u16().unwrap();
        r.u32().unwrap();
        r.u64().unwrap();
        r.usize().unwrap();
        r.u64().unwrap();
        get_option_time(&mut r).unwrap();
        r.u64().unwrap();
        scan_halt(&mut r).unwrap();
        scan_scheduler(&mut r).unwrap();
        scan_world(&mut r).unwrap();
        let count_at = r.at;
        let mut huge = bytes.clone();
        huge[count_at..count_at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut overflow_caps = FlowCheckpointWireLimits::default();
        overflow_caps.flow.max_resources = usize::MAX;
        overflow_caps.max_total_records = usize::MAX;
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&huge, overflow_caps).unwrap_err(),
            FlowCheckpointWireError::IntegerOverflow
        );
        let mut cap_bytes = bytes.clone();
        cap_bytes[count_at..count_at + 8].copy_from_slice(&2u64.to_le_bytes());
        let mut tiny_caps = FlowCheckpointWireLimits::default();
        tiny_caps.flow.max_resources = 1;
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&cap_bytes, tiny_caps).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("resource")
        );
        let mut short = bytes[..count_at + 8].to_vec();
        short[count_at..count_at + 8].copy_from_slice(&1u64.to_le_bytes());
        short.extend_from_slice(&[0; 11]);
        assert_eq!(
            FlowCheckpointV2::decode_wire_v2(&short, FlowCheckpointWireLimits::default())
                .unwrap_err(),
            FlowCheckpointWireError::Truncated
        );
    }
}

#[cfg(test)]
mod dispatch_scanning_tests {
    use super::*;
    #[test]
    fn scan_reader_does_not_reserve_or_retain_collection_values() {
        let dispatch = FlowDispatch {
            event: EventId::new(2, 2),
            at: SimTime::ZERO,
            records: Vec::new(),
            error: None,
            callback_batches: vec![FlowBatchReceipt::Accepted(vec![FlowCommandAdmission {
                ticket: FlowCommandTicket { batch: 0, index: 0 },
                event: EventId::new(1, 1),
                request: None,
                deadline_event: None,
            }])],
        };
        let limits = FlowCheckpointWireLimits::default();
        let bytes = dispatch.encode_wire_v1(limits).unwrap();
        let mut reader = Reader::new(&bytes, limits).unwrap();
        reader.scanning_dispatch = true;
        assert_eq!(reader.reserve::<u8>(usize::MAX).unwrap().capacity(), 0);
        assert_eq!(reader.raw(MAGIC.len()).unwrap(), MAGIC);
        assert_eq!(reader.u16().unwrap(), WIRE_SCHEMA);
        let scanned = decode_dispatch(&mut reader).unwrap();
        assert!(scanned.records.is_empty());
        assert!(scanned.callback_batches.is_empty());
        reader.finish().unwrap();
        assert_eq!(
            FlowDispatch::decode_wire_v1(&bytes, limits).unwrap(),
            dispatch
        );
    }
}

// V3 retains the complete DTO and command vector throughout all phases. These
// outer traversals deliberately leave the existing V1/V2 transport unchanged.
const MAGIC_V3: &[u8; 8] = b"KFLOWV3\0";
#[cfg(test)]
thread_local! {
    static V3_OWNED_DECODE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static V3_CANONICAL_VALIDATION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static V3_OUTPUT_ALLOCATION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
thread_local! {
    static V4_OWNED_DECODE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static V4_CANONICAL_VALIDATION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static V4_OUTPUT_ALLOCATION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
fn scan_flow_v3(bytes: &[u8], limits: FlowCheckpointWireLimits) -> WResult {
    let mut r = Reader::new(bytes, limits)?;
    if r.raw(8)? != MAGIC_V3 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    if r.u32()? != FLOW_CHECKPOINT_VERSION_V3 {
        return Err(FlowCheckpointWireError::InvalidValue(
            "Flow checkpoint version",
        ));
    }
    if r.u64()? == 0 || r.usize()? == 0 {
        return Err(FlowCheckpointWireError::InvalidValue("zero Flow limit"));
    }
    r.u64()?;
    scan_opt_time(&mut r)?;
    r.u64()?;
    scan_halt(&mut r)?;
    scan_scheduler(&mut r)?;
    scan_world(&mut r)?;
    let cap = r.limits.flow.max_resources;
    scan_vec(&mut r, cap, "resource", 12, |r| {
        get_resource(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_requests;
    scan_vec(&mut r, cap, "request", 12, |r| {
        get_request(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_actors;
    scan_vec(&mut r, cap, "actor", 12, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_actors;
    scan_vec(&mut r, cap, "actor domain", 24, |r| {
        get_entity(r)?;
        get_entity(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_works;
    scan_vec(&mut r, cap, "work registration", 30, scan_work_registration)?;
    scan_registration_vec(&mut r, 20, scan_context_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_builtin_stores_v3(&mut r)?;
    scan_registration_vec(&mut r, 28, scan_context_store)?;
    scan_registration_vec(&mut r, 12, |r| {
        r.scan_key()?;
        r.u32()?;
        Ok(())
    })?;
    scan_registration_vec(&mut r, 28, scan_restart_store)?;
    let n = r.count(r.limits.flow.max_commands, "command")?;
    r.require_min_items(n, 13)?;
    let mut gates = 0usize;
    let mut previous = None;
    for _ in 0..n {
        let event = get_event(&mut r)?;
        if previous.is_some_and(|p| p >= event) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "noncanonical order or duplicate entry",
            ));
        }
        previous = Some(event);
        if scan_command_v3(&mut r)? {
            gates = gates
                .checked_add(1)
                .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        }
    }
    if gates > r.limits.flow.max_commands {
        return Err(FlowCheckpointWireError::LimitExceeded("pending command"));
    }
    r.pending_count(gates)?;
    let cap = r.limits.flow.max_notifications;
    scan_vec(&mut r, cap, "notification", 126, |r| {
        get_event(r)?;
        scan_notification(r)
    })?;
    let n = r.count(r.limits.flow.max_pending_operations, "pending release")?;
    r.pending_count(n)?;
    scan_vec_counted(&mut r, n, 20, |r| {
        get_lease(r)?;
        Ok(())
    })?;
    let n = r.count(r.limits.flow.max_pending_operations, "pending despawn")?;
    r.pending_count(n)?;
    scan_vec_counted(&mut r, n, 12, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    let n = r.count(r.limits.flow.max_resources, "resource grant flag")?;
    r.require_min_items(n, 13)?;
    let mut prev = None;
    for _ in 0..n {
        let id = get_resource(&mut r)?;
        if prev.is_some_and(|p| p >= id) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "noncanonical order or duplicate entry",
            ));
        }
        prev = Some(id);
        r.bool()?;
    }
    scan_opt_time(&mut r)?;
    let n = r.count(r.limits.flow.max_resources, "allocation mode")?;
    scan_vec_counted(&mut r, n, 13, |r| {
        get_resource(r)?;
        match r.u8()? {
            0 | 1 => Ok(()),
            _ => Err(FlowCheckpointWireError::InvalidTag),
        }
    })?;
    r.finish()
}

fn scan_flow_v4(bytes: &[u8], limits: FlowCheckpointWireLimits) -> WResult {
    let mut r = Reader::new(bytes, limits)?;
    r.key_limit_label = "key bytes";
    if r.raw(8)? != MAGIC_V4 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    if r.u32()? != FLOW_CHECKPOINT_VERSION_V4 {
        return Err(FlowCheckpointWireError::InvalidValue(
            "Flow checkpoint version",
        ));
    }
    if r.u64()? == 0 || r.usize()? == 0 {
        return Err(FlowCheckpointWireError::InvalidValue("zero Flow limit"));
    }
    r.u64()?;
    scan_opt_time(&mut r)?;
    r.u64()?;
    scan_halt(&mut r)?;
    scan_scheduler(&mut r)?;
    scan_world(&mut r)?;
    let cap = r.limits.flow.max_resources;
    scan_vec(&mut r, cap, "resource", 12, |r| {
        get_resource(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_requests;
    scan_vec(&mut r, cap, "request", 12, |r| {
        get_request(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_actors;
    scan_vec(&mut r, cap, "actor", 12, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_actors;
    scan_vec(&mut r, cap, "actor domain", 24, |r| {
        get_entity(r)?;
        get_entity(r)?;
        Ok(())
    })?;
    let cap = r.limits.flow.max_works;
    scan_vec(&mut r, cap, "work registration", 30, scan_work_registration)?;
    scan_registration_vec(&mut r, 20, scan_context_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_registration_vec(&mut r, 26, scan_callback_registration)?;
    scan_builtin_stores_v3(&mut r)?;
    scan_registration_vec(&mut r, 28, scan_context_store)?;
    scan_registration_vec(&mut r, 12, |r| {
        r.scan_key()?;
        r.u32()?;
        Ok(())
    })?;
    scan_registration_vec(&mut r, 28, scan_restart_store)?;
    let n = r.count(r.limits.flow.max_commands, "command")?;
    r.require_min_items(n, 13)?;
    let mut gates = 0usize;
    let mut previous = None;
    for _ in 0..n {
        let event = get_event(&mut r)?;
        if previous.is_some_and(|p| p >= event) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "noncanonical order or duplicate entry",
            ));
        }
        previous = Some(event);
        if scan_command_v3(&mut r)? {
            gates = gates
                .checked_add(1)
                .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
        }
    }
    if gates > r.limits.flow.max_commands {
        return Err(FlowCheckpointWireError::LimitExceeded("pending command"));
    }
    r.pending_count(gates)?;
    let cap = r.limits.flow.max_notifications;
    scan_vec(&mut r, cap, "notification", 126, |r| {
        get_event(r)?;
        scan_notification(r)
    })?;
    let n = r.count(r.limits.flow.max_pending_operations, "pending release")?;
    r.pending_count(n)?;
    scan_vec_counted(&mut r, n, 20, |r| {
        get_lease(r)?;
        Ok(())
    })?;
    let n = r.count(r.limits.flow.max_pending_operations, "pending despawn")?;
    r.pending_count(n)?;
    scan_vec_counted(&mut r, n, 12, |r| {
        get_entity(r)?;
        Ok(())
    })?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    r.u64()?;
    let n = r.count(r.limits.flow.max_resources, "resource grant flag")?;
    r.require_min_items(n, 13)?;
    let mut prev = None;
    for _ in 0..n {
        let id = get_resource(&mut r)?;
        if prev.is_some_and(|p| p >= id) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "noncanonical order or duplicate entry",
            ));
        }
        prev = Some(id);
        r.bool()?;
    }
    scan_opt_time(&mut r)?;
    let n = r.count(r.limits.flow.max_resources, "allocation mode")?;
    scan_vec_counted(&mut r, n, 13, |r| {
        get_resource(r)?;
        match r.u8()? {
            0 | 1 => Ok(()),
            _ => Err(FlowCheckpointWireError::InvalidTag),
        }
    })?;
    scan_queue_order_metadata_v4(&mut r)?;
    r.finish()
}

fn scan_queue_order_metadata_v4(r: &mut Reader<'_>) -> WResult {
    let mode_count = r.count(r.limits.flow.max_resources, "queue order mode rows")?;
    r.require_min_items(mode_count, 13)?;
    let mut previous_resource = None;
    for _ in 0..mode_count {
        let resource = get_resource(r)?;
        match r.u8()? {
            0 | 1 => {}
            _ => return Err(FlowCheckpointWireError::InvalidTag),
        }
        if previous_resource.is_some_and(|previous| previous >= resource) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "queue-order metadata canonical order",
            ));
        }
        previous_resource = Some(resource);
    }

    let request_count = r.count(r.limits.flow.max_requests, "queue order request rows")?;
    r.require_min_items(request_count, 13)?;
    let mut previous_request = None;
    for _ in 0..request_count {
        let request = get_request(r)?;
        match r.u8()? {
            0 => {}
            1 => r.scan_opaque_key()?,
            _ => return Err(FlowCheckpointWireError::InvalidTag),
        }
        if previous_request.is_some_and(|previous| previous >= request) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "queue-order metadata canonical order",
            ));
        }
        previous_request = Some(request);
    }
    Ok(())
}

fn scan_command_v3(r: &mut Reader<'_>) -> Result<bool, FlowCheckpointWireError> {
    Ok(match r.u8()? {
        0 => {
            get_request(r)?;
            false
        }
        1 => {
            get_lease(r)?;
            false
        }
        2 => {
            get_resource(r)?;
            r.u32()?;
            false
        }
        3 => {
            get_resource(r)?;
            false
        }
        4 => {
            get_entity(r)?;
            false
        }
        5 | 6 => {
            get_request(r)?;
            false
        }
        7 => {
            get_request(r)?;
            r.i32()?;
            false
        }
        8 => {
            get_request(r)?;
            get_lease(r)?;
            r.u64()?;
            get_time(r)?;
            false
        }
        9 => false,
        10 => {
            get_work(r)?;
            get_kind(r)?;
            false
        }
        11 => {
            get_work(r)?;
            get_kind(r)?;
            scan_domain_control(r)?;
            false
        }
        12 => {
            get_resource(r)?;
            r.bool()?;
            true
        }
        13 => {
            get_resource(r)?;
            true
        }
        _ => return Err(FlowCheckpointWireError::InvalidTag),
    })
}

fn scan_builtin_stores_v3(r: &mut Reader<'_>) -> WResult {
    scan_component_option(r, 16, |r| {
        r.u32()?;
        Ok(())
    })?;
    scan_component_option(r, 20, scan_queue)?;
    scan_component_option(r, 52, scan_resource_request)?;
    scan_component_option(r, 28, scan_deadline_v3)?;
    scan_component_option(r, 28, scan_preempting_v3)?;
    scan_component_option(r, 20, scan_active_allocations)?;
    scan_component_option(r, 37, scan_work_spec)?;
    scan_component_option(r, 13, scan_role)?;
    scan_component_option(r, 96, scan_progress)
}
fn scan_deadline_v3(r: &mut Reader<'_>) -> WResult {
    r.usize()?;
    let mut previous = None;
    scan_vec(r, r.limits.flow.max_requests, "deadline entry", 40, |r| {
        let key = (get_time(r)?, scan_priority(r)?);
        if previous.is_some_and(|p| p >= key) {
            return Err(FlowCheckpointWireError::InvalidValue("deadline key order"));
        }
        previous = Some(key);
        Ok(())
    })
}
fn scan_preempting_v3(r: &mut Reader<'_>) -> WResult {
    r.usize()?;
    let mut previous = None;
    scan_vec(r, r.limits.flow.max_requests, "preemption key", 24, |r| {
        let key = scan_priority(r)?;
        if previous.is_some_and(|p| p >= key) {
            return Err(FlowCheckpointWireError::InvalidValue(
                "preemption key order",
            ));
        }
        previous = Some(key);
        Ok(())
    })
}
impl FlowCheckpointV3 {
    /// Encode the complete V3 image using canonical bounded little-endian fields.
    #[doc(hidden)]
    pub fn encode_wire_v3(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<Vec<u8>, FlowCheckpointWireError> {
        if self.version != FLOW_CHECKPOINT_VERSION_V3 {
            return Err(FlowCheckpointWireError::InvalidValue(
                "Flow checkpoint version",
            ));
        }
        preflight_counts_v3(self, limits)?;
        let mut measure = Writer::measure(limits);
        encode_flow_v3(self, &mut measure)?;
        validate_canonical_order_v3(self)?;
        #[cfg(test)]
        V3_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.set(calls.get() + 1));
        let mut writer = Writer::writing(limits, measure.len)?;
        encode_flow_v3(self, &mut writer)?;
        Ok(writer.finish())
    }

    /// Scan the complete payload before creating any owned DTO collections.
    #[doc(hidden)]
    pub fn decode_wire_v3(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        scan_flow_v3(bytes, limits)?;
        #[cfg(test)]
        V3_OWNED_DECODE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        let mut reader = Reader::new(bytes, limits)?;
        let image = decode_flow_v3(&mut reader)?;
        reader.finish()?;
        preflight_counts_v3(&image, limits)?;
        validate_canonical_order_v3(&image)?;
        Ok(image)
    }
}

fn preflight_counts_v3(image: &FlowCheckpointV3, limits: FlowCheckpointWireLimits) -> WResult {
    let f = limits.flow;
    let mut rows = 0usize;
    let mut sparse = 0usize;
    macro_rules! store {
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
    store!(image.builtins.capacities.as_ref());
    store!(image.builtins.queues.as_ref());
    store!(image.builtins.requests.as_ref());
    store!(image.builtins.deadlines.as_ref());
    store!(image.builtins.preempting.as_ref());
    store!(image.builtins.allocations.as_ref());
    store!(image.builtins.work_specs.as_ref());
    store!(image.builtins.work_roles.as_ref());
    store!(image.builtins.work_progress.as_ref());
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
    let pending = image
        .pending_releases
        .len()
        .checked_add(image.pending_despawns.len())
        .and_then(|n| {
            n.checked_add(
                image
                    .commands
                    .iter()
                    .filter(|(_, c)| {
                        matches!(
                            c,
                            FlowCommandV3::GrantEnabled(..) | FlowCommandV3::AllocationFlush(..)
                        )
                    })
                    .count(),
            )
        })
        .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    if image.world.slots.len() > f.max_entities
        || image.world.free_indices.len() > f.max_entities
        || image.world.live_entities.len() > f.max_entities
        || image.scheduler.entries.len() > f.max_scheduler_entries
        || image.resources.len() > f.max_resources
        || image.resource_grant_flags.len() > f.max_resources
        || image.allocation_modes.len() > f.max_resources
        || image.requests.len() > f.max_requests
        || image.actors.len() > f.max_actors
        || image.work_registrations.len() > f.max_works
        || image.actor_domains.len() > f.max_actors
        || image.commands.len() > f.max_commands
        || image.notifications.len() > f.max_notifications
        || pending > f.max_pending_operations
        || rows > f.max_component_rows
        || sparse > f.max_sparse_slots
        || registrations > f.max_registrations
    {
        return Err(FlowCheckpointWireError::LimitExceeded("Flow collection"));
    }
    if image.builtins.queues.as_ref().is_some_and(|s| {
        s.rows
            .iter()
            .any(|(_, q)| q.requests.len() > f.max_requests)
    }) || image
        .builtins
        .allocations
        .as_ref()
        .is_some_and(|s| s.rows.iter().any(|(_, a)| a.leases.len() > f.max_requests))
        || image
            .builtins
            .deadlines
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, d)| d.entries.len() > f.max_requests))
        || image
            .builtins
            .preempting
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, p)| p.keys.len() > f.max_requests))
    {
        return Err(FlowCheckpointWireError::LimitExceeded("Flow collection"));
    }
    Ok(())
}

fn preflight_counts_v4(image: &FlowCheckpointV4, limits: FlowCheckpointWireLimits) -> WResult {
    let f = limits.flow;
    let mut rows = 0usize;
    let mut sparse = 0usize;
    macro_rules! store {
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
    store!(image.builtins.capacities.as_ref());
    store!(image.builtins.queues.as_ref());
    store!(image.builtins.requests.as_ref());
    store!(image.builtins.deadlines.as_ref());
    store!(image.builtins.preempting.as_ref());
    store!(image.builtins.allocations.as_ref());
    store!(image.builtins.work_specs.as_ref());
    store!(image.builtins.work_roles.as_ref());
    store!(image.builtins.work_progress.as_ref());
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
    let pending = image
        .pending_releases
        .len()
        .checked_add(image.pending_despawns.len())
        .and_then(|n| {
            n.checked_add(
                image
                    .commands
                    .iter()
                    .filter(|(_, c)| {
                        matches!(
                            c,
                            FlowCommandV3::GrantEnabled(..) | FlowCommandV3::AllocationFlush(..)
                        )
                    })
                    .count(),
            )
        })
        .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
    if image.queue_order_modes.len() > f.max_resources {
        return Err(FlowCheckpointWireError::LimitExceeded(
            "queue order mode rows",
        ));
    }
    if image.request_queue_orders.len() > f.max_requests {
        return Err(FlowCheckpointWireError::LimitExceeded(
            "queue order request rows",
        ));
    }
    let mut queue_key_bytes = 0usize;
    for (_, ordering) in &image.request_queue_orders {
        if let FlowRequestQueueOrderV4::StableKey(bytes) = ordering {
            if bytes.is_empty() {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "stable queue-order key",
                ));
            }
            if bytes.len() > MAX_WIRE_QUEUE_ORDER_KEY_BYTES {
                return Err(FlowCheckpointWireError::LimitExceeded(
                    "queue order key bytes",
                ));
            }
            queue_key_bytes = queue_key_bytes
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointWireError::IntegerOverflow)?;
            if queue_key_bytes > f.max_key_bytes {
                return Err(FlowCheckpointWireError::LimitExceeded("key bytes"));
            }
        }
    }
    if image.world.slots.len() > f.max_entities
        || image.world.free_indices.len() > f.max_entities
        || image.world.live_entities.len() > f.max_entities
        || image.scheduler.entries.len() > f.max_scheduler_entries
        || image.resources.len() > f.max_resources
        || image.resource_grant_flags.len() > f.max_resources
        || image.allocation_modes.len() > f.max_resources
        || image.requests.len() > f.max_requests
        || image.actors.len() > f.max_actors
        || image.work_registrations.len() > f.max_works
        || image.actor_domains.len() > f.max_actors
        || image.commands.len() > f.max_commands
        || image.notifications.len() > f.max_notifications
        || pending > f.max_pending_operations
        || rows > f.max_component_rows
        || sparse > f.max_sparse_slots
        || registrations > f.max_registrations
    {
        return Err(FlowCheckpointWireError::LimitExceeded("Flow collection"));
    }
    if image.builtins.queues.as_ref().is_some_and(|s| {
        s.rows
            .iter()
            .any(|(_, q)| q.requests.len() > f.max_requests)
    }) || image
        .builtins
        .allocations
        .as_ref()
        .is_some_and(|s| s.rows.iter().any(|(_, a)| a.leases.len() > f.max_requests))
        || image
            .builtins
            .deadlines
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, d)| d.entries.len() > f.max_requests))
        || image
            .builtins
            .preempting
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, p)| p.keys.len() > f.max_requests))
    {
        return Err(FlowCheckpointWireError::LimitExceeded("Flow collection"));
    }
    Ok(())
}

fn validate_canonical_order_v3(i: &FlowCheckpointV3) -> WResult {
    #[cfg(test)]
    V3_CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
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
        && strict_by(&i.scheduler.entries, |a, b| a.sequence.cmp(&b.sequence))
        && strict_by(&i.resource_grant_flags, |a, b| a.0.cmp(&b.0));
    if !ordered {
        return Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry",
        ));
    }
    if i.resources.len() != i.resource_grant_flags.len()
        || !i
            .resources
            .iter()
            .zip(&i.resource_grant_flags)
            .all(|(r, (f, _))| r == f)
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "resource grant flags",
        ));
    }
    if i.started_frontier != (i.scheduler.dispatched_events > 0).then_some(i.scheduler.now) {
        return Err(FlowCheckpointWireError::InvalidValue("started frontier"));
    }
    if !strict_by(&i.allocation_modes, |a, b| a.0.cmp(&b.0)) {
        return Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry",
        ));
    }
    if i.resources.len() != i.allocation_modes.len()
        || !i
            .resources
            .iter()
            .zip(&i.allocation_modes)
            .all(|(r, (m, _))| r == m)
    {
        return Err(FlowCheckpointWireError::InvalidValue("allocation modes"));
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
        ($s:expr) => {
            if let Some(s) = $s {
                ensure_unique(s.rows.iter().map(|(id, _)| *id))?;
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
    for s in &i.context_stores {
        ensure_unique(s.rows.iter().map(|(id, _)| *id))?;
    }
    for s in &i.restart_stores {
        ensure_unique(s.rows.iter().map(|(id, _, _)| *id))?;
    }
    if i.notifications.iter().any(|(_, n)| n.kind > 1) {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    if let Some(s) = &i.builtins.queues {
        for (_, q) in &s.rows {
            if !q.requests.iter().is_sorted() {
                return Err(FlowCheckpointWireError::InvalidValue("queue key order"));
            }
        }
    }
    if let Some(s) = &i.builtins.deadlines {
        for (_, v) in &s.rows {
            if !v.entries.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue("deadline key order"));
            }
        }
    }
    if let Some(s) = &i.builtins.preempting {
        for (_, v) in &s.rows {
            if !v.keys.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "preemption key order",
                ));
            }
        }
    }
    Ok(())
}

fn validate_canonical_order_v4(i: &FlowCheckpointV4) -> WResult {
    #[cfg(test)]
    V4_CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
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
        && strict_by(&i.scheduler.entries, |a, b| a.sequence.cmp(&b.sequence))
        && strict_by(&i.resource_grant_flags, |a, b| a.0.cmp(&b.0));
    if !ordered {
        return Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry",
        ));
    }
    if i.resources.len() != i.resource_grant_flags.len()
        || !i
            .resources
            .iter()
            .zip(&i.resource_grant_flags)
            .all(|(r, (f, _))| r == f)
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "resource grant flags",
        ));
    }
    if i.started_frontier != (i.scheduler.dispatched_events > 0).then_some(i.scheduler.now) {
        return Err(FlowCheckpointWireError::InvalidValue("started frontier"));
    }
    if !strict_by(&i.allocation_modes, |a, b| a.0.cmp(&b.0)) {
        return Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry",
        ));
    }
    if i.resources.len() != i.allocation_modes.len()
        || !i
            .resources
            .iter()
            .zip(&i.allocation_modes)
            .all(|(r, (m, _))| r == m)
    {
        return Err(FlowCheckpointWireError::InvalidValue("allocation modes"));
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
        ($s:expr) => {
            if let Some(s) = $s {
                ensure_unique(s.rows.iter().map(|(id, _)| *id))?;
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
    for s in &i.context_stores {
        ensure_unique(s.rows.iter().map(|(id, _)| *id))?;
    }
    for s in &i.restart_stores {
        ensure_unique(s.rows.iter().map(|(id, _, _)| *id))?;
    }
    if i.notifications.iter().any(|(_, n)| n.kind > 1) {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    if let Some(s) = &i.builtins.queues {
        for (_, q) in &s.rows {
            if !q.requests.iter().is_sorted() {
                return Err(FlowCheckpointWireError::InvalidValue("queue key order"));
            }
        }
    }
    if let Some(s) = &i.builtins.deadlines {
        for (_, v) in &s.rows {
            if !v.entries.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue("deadline key order"));
            }
        }
    }
    if let Some(s) = &i.builtins.preempting {
        for (_, v) in &s.rows {
            if !v.keys.windows(2).all(|w| w[0] < w[1]) {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "preemption key order",
                ));
            }
        }
    }
    validate_queue_order_metadata_wire_v4(i)?;
    Ok(())
}

fn validate_queue_order_metadata_wire_v4(i: &FlowCheckpointV4) -> WResult {
    if !strict_by(&i.queue_order_modes, |a, b| a.0.cmp(&b.0))
        || !strict_by(&i.request_queue_orders, |a, b| a.0.cmp(&b.0))
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata canonical order",
        ));
    }
    let resources: std::collections::BTreeSet<_> = i.resources.iter().copied().collect();
    if resources.len() != i.resources.len()
        || i.queue_order_modes.len() != resources.len()
        || i.queue_order_modes
            .iter()
            .any(|(id, _)| !resources.contains(id))
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage",
        ));
    }
    let modes: std::collections::BTreeMap<_, _> = i
        .queue_order_modes
        .iter()
        .map(|(id, mode)| (*id, *mode))
        .collect();
    let request_ids: std::collections::BTreeSet<_> = i.requests.iter().copied().collect();
    if request_ids.len() != i.requests.len()
        || i.request_queue_orders.len() != i.requests.len()
        || i.request_queue_orders
            .iter()
            .any(|(id, _)| !request_ids.contains(id))
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage",
        ));
    }
    let mut request_values = std::collections::BTreeMap::new();
    if let Some(store) = &i.builtins.requests {
        if store.rows.len() != i.requests.len() {
            return Err(FlowCheckpointWireError::InvalidValue(
                "queue-order metadata coverage",
            ));
        }
        for (entity, request) in &store.rows {
            if request_values.insert(RequestId(*entity), request).is_some() {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "queue-order metadata coverage",
                ));
            }
        }
    }
    if request_values
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        != request_ids
    {
        return Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage",
        ));
    }
    let terminal = |state| {
        matches!(
            state,
            RequestState::Released
                | RequestState::Cancelled
                | RequestState::TimedOut
                | RequestState::Completed
                | RequestState::Aborted
        )
    };
    let mut removed_modes = std::collections::BTreeMap::new();
    for (request_id, ordering) in &i.request_queue_orders {
        let request =
            request_values
                .get(request_id)
                .ok_or(FlowCheckpointWireError::InvalidValue(
                    "queue-order metadata coverage",
                ))?;
        let stable = matches!(ordering, FlowRequestQueueOrderV4::StableKey(_));
        match modes.get(&request.resource) {
            Some(FlowQueueOrdering::AdmissionOrder) if stable => {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "queue-order metadata coverage",
                ));
            }
            Some(FlowQueueOrdering::StableKey) if !stable => {
                return Err(FlowCheckpointWireError::InvalidValue(
                    "queue-order metadata coverage",
                ));
            }
            Some(_) => {}
            None => {
                if !terminal(request.state) {
                    return Err(FlowCheckpointWireError::InvalidValue(
                        "queue-order metadata coverage",
                    ));
                }
                if removed_modes
                    .insert(request.resource, stable)
                    .is_some_and(|previous| previous != stable)
                {
                    return Err(FlowCheckpointWireError::InvalidValue(
                        "queue-order metadata coverage",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn encode_flow_v3(i: &FlowCheckpointV3, w: &mut Writer) -> WResult {
    w.raw(MAGIC_V3)?;
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
        put_command_v3(w, *c)
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
    w.u64(i.next_lease)?;
    put_vec(w, &i.resource_grant_flags, |w, (resource, enabled)| {
        put_resource(w, *resource)?;
        w.bool(*enabled)
    })?;
    put_option_time(w, i.started_frontier)?;
    put_vec(w, &i.allocation_modes, |w, (resource, mode)| {
        put_resource(w, *resource)?;
        w.u8(match mode {
            FlowAllocationMode::Eager => 0,
            FlowAllocationMode::Deferred => 1,
        })
    })
}

fn encode_flow_v4_body(i: &FlowCheckpointV4, w: &mut Writer) -> WResult {
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
        put_command_v3(w, *c)
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
    w.u64(i.next_lease)?;
    put_vec(w, &i.resource_grant_flags, |w, (resource, enabled)| {
        put_resource(w, *resource)?;
        w.bool(*enabled)
    })?;
    put_option_time(w, i.started_frontier)?;
    put_vec(w, &i.allocation_modes, |w, (resource, mode)| {
        put_resource(w, *resource)?;
        w.u8(match mode {
            FlowAllocationMode::Eager => 0,
            FlowAllocationMode::Deferred => 1,
        })
    })
}

fn encode_flow_v4(i: &FlowCheckpointV4, w: &mut Writer) -> WResult {
    w.raw(MAGIC_V4)?;
    w.u16(WIRE_SCHEMA)?;
    encode_flow_v4_body(i, w)?;
    put_vec(w, &i.queue_order_modes, |w, (resource, mode)| {
        put_resource(w, *resource)?;
        w.u8(match mode {
            FlowQueueOrdering::AdmissionOrder => 0,
            FlowQueueOrdering::StableKey => 1,
        })
    })?;
    put_vec(w, &i.request_queue_orders, |w, (request, ordering)| {
        put_request(w, *request)?;
        match ordering {
            FlowRequestQueueOrderV4::AdmissionOrder => w.u8(0),
            FlowRequestQueueOrderV4::StableKey(bytes) => {
                w.u8(1)?;
                w.opaque_key(bytes)
            }
        }
    })
}

impl FlowCheckpointV4 {
    /// Encode a complete V4 image in the direct canonical wire format.
    #[doc(hidden)]
    pub fn encode_wire_v4(
        &self,
        limits: FlowCheckpointWireLimits,
    ) -> Result<Vec<u8>, FlowCheckpointWireError> {
        if self.version != FLOW_CHECKPOINT_VERSION_V4 {
            return Err(FlowCheckpointWireError::InvalidValue(
                "Flow checkpoint version",
            ));
        }
        preflight_counts_v4(self, limits)?;
        let mut measure = Writer::measure(limits);
        measure.key_limit_label = "key bytes";
        encode_flow_v4(self, &mut measure)?;
        validate_canonical_order_v4(self)?;
        #[cfg(test)]
        V4_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        let mut writer = Writer::writing(limits, measure.len)?;
        writer.key_limit_label = "key bytes";
        encode_flow_v4(self, &mut writer)?;
        Ok(writer.finish())
    }

    /// Scan and decode a complete V4 image without falling back to older formats.
    #[doc(hidden)]
    pub fn decode_wire_v4(
        bytes: &[u8],
        limits: FlowCheckpointWireLimits,
    ) -> Result<Self, FlowCheckpointWireError> {
        scan_flow_v4(bytes, limits)?;
        #[cfg(test)]
        V4_OWNED_DECODE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        let mut reader = Reader::new(bytes, limits)?;
        reader.key_limit_label = "key bytes";
        let image = decode_flow_v4(&mut reader)?;
        reader.finish()?;
        preflight_counts_v4(&image, limits)?;
        validate_canonical_order_v4(&image)?;
        Ok(image)
    }
}

fn decode_flow_v3(r: &mut Reader<'_>) -> WResult<FlowCheckpointV3> {
    if r.raw(MAGIC_V3.len())? != MAGIC_V3 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    let version = r.u32()?;
    if version != FLOW_CHECKPOINT_VERSION_V3 {
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
        let event = get_event(r)?;
        let command = get_command_v3(r)?;
        if matches!(
            command,
            FlowCommandV3::GrantEnabled(..) | FlowCommandV3::AllocationFlush(..)
        ) {
            r.pending_count(1)?;
        }
        Ok((event, command))
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
    let created = r.u64()?;
    let destroyed = r.u64()?;
    let scheduled = r.u64()?;
    let next_admission = r.u64()?;
    let next_lease = r.u64()?;
    let resource_grant_flags = get_vec(
        r,
        r.limits.flow.max_resources,
        "resource grant flag",
        13,
        |r| Ok((get_resource(r)?, r.bool()?)),
    )?;
    let started_frontier = get_option_time(r)?;
    let allocation_modes = get_vec(r, r.limits.flow.max_resources, "allocation mode", 13, |r| {
        let resource = get_resource(r)?;
        let mode = match r.u8()? {
            0 => FlowAllocationMode::Eager,
            1 => FlowAllocationMode::Deferred,
            _ => return Err(FlowCheckpointWireError::InvalidTag),
        };
        Ok((resource, mode))
    })?;
    Ok(FlowCheckpointV3 {
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
        created,
        destroyed,
        scheduled,
        next_admission,
        next_lease,
        resource_grant_flags,
        started_frontier,
        allocation_modes,
    })
}
fn decode_flow_v4(r: &mut Reader<'_>) -> WResult<FlowCheckpointV4> {
    if r.raw(MAGIC_V4.len())? != MAGIC_V4 {
        return Err(FlowCheckpointWireError::InvalidTag);
    }
    let schema = r.u16()?;
    if schema != WIRE_SCHEMA {
        return Err(FlowCheckpointWireError::UnsupportedSchema(schema));
    }
    let version = r.u32()?;
    if version != FLOW_CHECKPOINT_VERSION_V4 {
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
        let event = get_event(r)?;
        let command = get_command_v3(r)?;
        if matches!(
            command,
            FlowCommandV3::GrantEnabled(..) | FlowCommandV3::AllocationFlush(..)
        ) {
            r.pending_count(1)?;
        }
        Ok((event, command))
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
    let created = r.u64()?;
    let destroyed = r.u64()?;
    let scheduled = r.u64()?;
    let next_admission = r.u64()?;
    let next_lease = r.u64()?;
    let resource_grant_flags = get_vec(
        r,
        r.limits.flow.max_resources,
        "resource grant flag",
        13,
        |r| Ok((get_resource(r)?, r.bool()?)),
    )?;
    let started_frontier = get_option_time(r)?;
    let allocation_modes = get_vec(r, r.limits.flow.max_resources, "allocation mode", 13, |r| {
        let resource = get_resource(r)?;
        let mode = match r.u8()? {
            0 => FlowAllocationMode::Eager,
            1 => FlowAllocationMode::Deferred,
            _ => return Err(FlowCheckpointWireError::InvalidTag),
        };
        Ok((resource, mode))
    })?;
    let queue_order_modes = get_vec(
        r,
        r.limits.flow.max_resources,
        "queue order mode rows",
        13,
        |r| {
            let resource = get_resource(r)?;
            let mode = match r.u8()? {
                0 => FlowQueueOrdering::AdmissionOrder,
                1 => FlowQueueOrdering::StableKey,
                _ => return Err(FlowCheckpointWireError::InvalidTag),
            };
            Ok((resource, mode))
        },
    )?;
    let request_queue_orders = get_vec(
        r,
        r.limits.flow.max_requests,
        "queue order request rows",
        13,
        |r| {
            let request = get_request(r)?;
            let ordering = match r.u8()? {
                0 => FlowRequestQueueOrderV4::AdmissionOrder,
                1 => FlowRequestQueueOrderV4::StableKey(r.opaque_key()?),
                _ => return Err(FlowCheckpointWireError::InvalidTag),
            };
            Ok((request, ordering))
        },
    )?;
    let base = FlowCheckpointV3 {
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
        created,
        destroyed,
        scheduled,
        next_admission,
        next_lease,
        resource_grant_flags,
        started_frontier,
        allocation_modes,
    };
    Ok(FlowCheckpointV4 {
        version: base.version,
        config: base.config,
        callback_config: base.callback_config,
        next_batch_identity: base.next_batch_identity,
        budget_tick: base.budget_tick,
        budget_consumed: base.budget_consumed,
        budget_halt: base.budget_halt,
        scheduler: base.scheduler,
        world: base.world,
        resources: base.resources,
        requests: base.requests,
        actors: base.actors,
        actor_domains: base.actor_domains,
        work_registrations: base.work_registrations,
        contexts: base.contexts,
        handlers: base.handlers,
        continuations: base.continuations,
        domains: base.domains,
        builtins: base.builtins,
        context_stores: base.context_stores,
        restart_store_manifest: base.restart_store_manifest,
        restart_stores: base.restart_stores,
        commands: base.commands,
        notifications: base.notifications,
        pending_releases: base.pending_releases,
        pending_despawns: base.pending_despawns,
        created: base.created,
        destroyed: base.destroyed,
        scheduled: base.scheduled,
        next_admission: base.next_admission,
        next_lease: base.next_lease,
        resource_grant_flags: base.resource_grant_flags,
        started_frontier: base.started_frontier,
        allocation_modes: base.allocation_modes,
        queue_order_modes,
        request_queue_orders,
    })
}

#[cfg(test)]
mod v4_wire_preflight_tests {
    use super::*;

    fn first_grant_flag_offset(bytes: &[u8]) -> usize {
        let limits = FlowCheckpointWireLimits::default();
        let mut r = Reader::new(bytes, limits).unwrap();
        r.raw(8).unwrap();
        r.u16().unwrap();
        r.u32().unwrap();
        r.u64().unwrap();
        r.usize().unwrap();
        r.u64().unwrap();
        scan_opt_time(&mut r).unwrap();
        r.u64().unwrap();
        scan_halt(&mut r).unwrap();
        scan_scheduler(&mut r).unwrap();
        scan_world(&mut r).unwrap();
        scan_vec(&mut r, limits.flow.max_resources, "resource", 12, |r| {
            get_resource(r)?;
            Ok(())
        })
        .unwrap();
        scan_vec(&mut r, limits.flow.max_requests, "request", 12, |r| {
            get_request(r)?;
            Ok(())
        })
        .unwrap();
        scan_vec(&mut r, limits.flow.max_actors, "actor", 12, |r| {
            get_entity(r)?;
            Ok(())
        })
        .unwrap();
        scan_vec(&mut r, limits.flow.max_actors, "actor domain", 24, |r| {
            get_entity(r)?;
            get_entity(r)?;
            Ok(())
        })
        .unwrap();
        scan_vec(
            &mut r,
            limits.flow.max_works,
            "work registration",
            30,
            scan_work_registration,
        )
        .unwrap();
        scan_registration_vec(&mut r, 20, scan_context_registration).unwrap();
        scan_registration_vec(&mut r, 26, scan_callback_registration).unwrap();
        scan_registration_vec(&mut r, 26, scan_callback_registration).unwrap();
        scan_registration_vec(&mut r, 26, scan_callback_registration).unwrap();
        scan_builtin_stores_v3(&mut r).unwrap();
        scan_registration_vec(&mut r, 28, scan_context_store).unwrap();
        scan_registration_vec(&mut r, 12, |r| {
            r.scan_key()?;
            r.u32()?;
            Ok(())
        })
        .unwrap();
        scan_registration_vec(&mut r, 28, scan_restart_store).unwrap();
        let commands = r.count(limits.flow.max_commands, "command").unwrap();
        r.require_min_items(commands, 13).unwrap();
        for _ in 0..commands {
            get_event(&mut r).unwrap();
            scan_command_v3(&mut r).unwrap();
        }
        scan_vec(
            &mut r,
            limits.flow.max_notifications,
            "notification",
            126,
            |r| {
                get_event(r)?;
                scan_notification(r)
            },
        )
        .unwrap();
        for _ in 0..2 {
            let count = r
                .count(limits.flow.max_pending_operations, "pending operation")
                .unwrap();
            assert_eq!(count, 0);
            r.pending_count(count).unwrap();
        }
        // The test fixture has no pending releases or despawns, so their two
        // count words are zero and the five counters follow directly.
        r.u64().unwrap();
        r.u64().unwrap();
        r.u64().unwrap();
        r.u64().unwrap();
        r.u64().unwrap();
        let count = r
            .count(limits.flow.max_resources, "resource grant flag")
            .unwrap();
        assert_eq!(count, 1);
        get_resource(&mut r).unwrap();
        r.at
    }

    #[test]
    fn v4_scanner_rejects_final_row_before_owned_decode() {
        let mut flow = FlowRuntime::new();
        flow.create_resource(0).unwrap();
        let image = flow
            .capture_checkpoint_v4(
                &FlowCheckpointCodecs::new(),
                FlowCheckpointLimits::default(),
            )
            .unwrap();
        let mut bytes = image
            .encode_wire_v4(FlowCheckpointWireLimits::default())
            .unwrap();
        // One mode row (ResourceId plus tag), followed by the empty request vector.
        let tag = bytes.len() - 8 - 1;
        bytes[tag] = 2;
        V4_OWNED_DECODE_CALLS.with(|calls| calls.set(0));
        assert_eq!(
            FlowCheckpointV4::decode_wire_v4(&bytes, FlowCheckpointWireLimits::default()),
            Err(FlowCheckpointWireError::InvalidTag)
        );
        V4_OWNED_DECODE_CALLS.with(|calls| assert_eq!(calls.get(), 0));
        let mut trailing = image
            .encode_wire_v4(FlowCheckpointWireLimits::default())
            .unwrap();
        trailing.push(0);
        V4_OWNED_DECODE_CALLS.with(|calls| calls.set(0));
        assert_eq!(
            FlowCheckpointV4::decode_wire_v4(&trailing, FlowCheckpointWireLimits::default()),
            Err(FlowCheckpointWireError::TrailingBytes)
        );
        V4_OWNED_DECODE_CALLS.with(|calls| assert_eq!(calls.get(), 0));
    }

    #[test]
    fn v4_scanner_rejects_invalid_resource_grant_boolean() {
        let mut flow = FlowRuntime::new();
        flow.create_resource(0).unwrap();
        let image = flow
            .capture_checkpoint_v4(
                &FlowCheckpointCodecs::new(),
                FlowCheckpointLimits::default(),
            )
            .unwrap();
        let mut bytes = image
            .encode_wire_v4(FlowCheckpointWireLimits::default())
            .unwrap();
        let offset = first_grant_flag_offset(&bytes);
        assert_eq!(bytes[offset], 1);
        bytes[offset] = 2;
        assert_eq!(
            FlowCheckpointV4::decode_wire_v4(&bytes, FlowCheckpointWireLimits::default()),
            Err(FlowCheckpointWireError::InvalidBoolean)
        );
    }

    #[test]
    fn v4_measurement_failure_precedes_single_output_allocation() {
        let image = FlowRuntime::new()
            .capture_checkpoint_v4(
                &FlowCheckpointCodecs::new(),
                FlowCheckpointLimits::default(),
            )
            .unwrap();
        V4_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.set(0));
        assert!(matches!(
            image.encode_wire_v4(FlowCheckpointWireLimits {
                max_wire_bytes: 1,
                ..FlowCheckpointWireLimits::default()
            }),
            Err(FlowCheckpointWireError::LimitExceeded("total byte"))
        ));
        V4_OUTPUT_ALLOCATION_CALLS.with(|calls| assert_eq!(calls.get(), 0));
        image
            .encode_wire_v4(FlowCheckpointWireLimits::default())
            .unwrap();
        V4_OUTPUT_ALLOCATION_CALLS.with(|calls| assert_eq!(calls.get(), 1));
    }

    #[test]
    fn v4_wire_lengths_and_record_sums_use_checked_arithmetic() {
        let mut writer = Writer::measure(FlowCheckpointWireLimits::default());
        writer.len = usize::MAX;
        assert_eq!(
            writer.raw(&[0]),
            Err(FlowCheckpointWireError::IntegerOverflow)
        );
        let mut writer = Writer::measure(FlowCheckpointWireLimits::default());
        writer.records = usize::MAX;
        assert_eq!(
            writer.count(1),
            Err(FlowCheckpointWireError::IntegerOverflow)
        );
        let mut writer = Writer::measure(FlowCheckpointWireLimits::default());
        writer.key_bytes = usize::MAX;
        assert_eq!(
            writer.opaque_key(b"k"),
            Err(FlowCheckpointWireError::IntegerOverflow)
        );

        #[cfg(target_pointer_width = "32")]
        {
            let bytes = u64::MAX.to_le_bytes();
            let mut reader = Reader::new(&bytes, FlowCheckpointWireLimits::default()).unwrap();
            assert_eq!(
                reader.usize(),
                Err(FlowCheckpointWireError::IntegerOverflow)
            );
        }
    }
}
fn put_command_v3(w: &mut Writer, c: FlowCommandV3) -> WResult {
    match c {
        FlowCommandV3::Submit(x) => {
            w.u8(0)?;
            put_request(w, x)
        }
        FlowCommandV3::Release(x) => {
            w.u8(1)?;
            put_lease(w, x)
        }
        FlowCommandV3::Capacity(x, n) => {
            w.u8(2)?;
            put_resource(w, x)?;
            w.u32(n)
        }
        FlowCommandV3::Remove(x) => {
            w.u8(3)?;
            put_resource(w, x)
        }
        FlowCommandV3::Despawn(x) => {
            w.u8(4)?;
            put_entity(w, x)
        }
        FlowCommandV3::Deadline(x) => {
            w.u8(5)?;
            put_request(w, x)
        }
        FlowCommandV3::Cancel(x) => {
            w.u8(6)?;
            put_request(w, x)
        }
        FlowCommandV3::Reprioritize(x, n) => {
            w.u8(7)?;
            put_request(w, x)?;
            w.i32(n)
        }
        FlowCommandV3::Completion(req, lease, rev, at) => {
            w.u8(8)?;
            put_request(w, req)?;
            put_lease(w, lease)?;
            w.u64(rev)?;
            put_time(w, at)
        }
        FlowCommandV3::Notify => w.u8(9),
        FlowCommandV3::Domain(work, kind) => {
            w.u8(10)?;
            put_work(w, work)?;
            put_kind(w, kind)
        }
        FlowCommandV3::DomainControl(work, kind, action) => {
            w.u8(11)?;
            put_work(w, work)?;
            put_kind(w, kind)?;
            put_domain_control(w, action)
        }
        FlowCommandV3::AllocationFlush(resource) => {
            w.u8(13)?;
            put_resource(w, resource)
        }
        FlowCommandV3::GrantEnabled(resource, enabled) => {
            w.u8(12)?;
            put_resource(w, resource)?;
            w.bool(enabled)
        }
    }
}
fn get_command_v3(r: &mut Reader<'_>) -> WResult<FlowCommandV3> {
    match r.u8()? {
        0 => Ok(FlowCommandV3::Submit(get_request(r)?)),
        1 => Ok(FlowCommandV3::Release(get_lease(r)?)),
        2 => Ok(FlowCommandV3::Capacity(get_resource(r)?, r.u32()?)),
        3 => Ok(FlowCommandV3::Remove(get_resource(r)?)),
        4 => Ok(FlowCommandV3::Despawn(get_entity(r)?)),
        5 => Ok(FlowCommandV3::Deadline(get_request(r)?)),
        6 => Ok(FlowCommandV3::Cancel(get_request(r)?)),
        7 => Ok(FlowCommandV3::Reprioritize(get_request(r)?, r.i32()?)),
        8 => Ok(FlowCommandV3::Completion(
            get_request(r)?,
            get_lease(r)?,
            r.u64()?,
            get_time(r)?,
        )),
        9 => Ok(FlowCommandV3::Notify),
        10 => Ok(FlowCommandV3::Domain(get_work(r)?, get_kind(r)?)),
        11 => Ok(FlowCommandV3::DomainControl(
            get_work(r)?,
            get_kind(r)?,
            get_domain_control(r)?,
        )),
        12 => Ok(FlowCommandV3::GrantEnabled(get_resource(r)?, r.bool()?)),
        13 => Ok(FlowCommandV3::AllocationFlush(get_resource(r)?)),
        _ => Err(FlowCheckpointWireError::InvalidTag),
    }
}

#[cfg(test)]
mod wire_v3_tests {
    use super::*;
    thread_local! { static DECODER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
    fn codecs() -> FlowCheckpointCodecs {
        FlowCheckpointCodecs::new()
    }
    fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        if remaining < 8 {
            return Err(FlowCheckpointCodecError("payload budget".into()));
        }
        Ok(value.to_le_bytes().to_vec())
    }
    fn decode_u64(
        bytes: &[u8],
        _: &FlowCheckpointRebindV1,
    ) -> Result<u64, FlowCheckpointCodecError> {
        DECODER_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        let value: [u8; 8] = bytes
            .try_into()
            .map_err(|_| FlowCheckpointCodecError("u64 payload".into()))?;
        Ok(u64::from_le_bytes(value))
    }
    fn restart_factory(template: &u64) -> u64 {
        *template + 1
    }
    fn completed(_: &mut u64, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {}
    fn continuations() -> FlowContinuations<u64> {
        FlowContinuations {
            on_complete: Some(completed),
            ..FlowContinuations::default()
        }
    }
    fn rich_image() -> (FlowCheckpointV3, FlowCheckpointCodecs) {
        let mut codecs = codecs();
        codecs
            .register_context::<u64>("scan.context", 1, encode_u64, decode_u64)
            .unwrap();
        codecs
            .register_restart_template::<u64, u64>("scan.restart", 1, encode_u64, decode_u64)
            .unwrap();
        codecs
            .register_restart_factory::<u64, u64>("scan.restart", "scan.factory", restart_factory)
            .unwrap();
        let mut runtime = FlowRuntime::new();
        for key in ["scan.context", "scan.restart"] {
            runtime
                .register_work_continuations(key, continuations())
                .unwrap();
            codecs
                .register_work_continuations(
                    key,
                    FlowContinuationCodeIds {
                        on_complete: Some(FlowCallbackCodeV1 {
                            stable_id: "scan.complete".into(),
                            version: 1,
                        }),
                        ..FlowContinuationCodeIds::default()
                    },
                    continuations(),
                )
                .unwrap();
        }
        let resource = runtime
            .create_resource_with_allocation_mode(2, FlowAllocationMode::Deferred)
            .unwrap();
        let owners: Vec<_> = (0..5).map(|_| runtime.spawn_actor().unwrap()).collect();
        let mut requests = Vec::new();
        for (index, &owner) in owners.iter().take(4).enumerate() {
            let work = if index == 0 {
                runtime
                    .create_restartable_work(
                        owner,
                        SimDuration::from_ticks(12),
                        "scan.restart",
                        100_u64,
                        restart_factory,
                    )
                    .unwrap()
            } else {
                runtime
                    .create_work(
                        owner,
                        SimDuration::from_ticks(12),
                        "scan.context",
                        index as u64,
                    )
                    .unwrap()
            };
            requests.push(
                runtime
                    .acquire(resource)
                    .owner(owner)
                    .timed_work(work)
                    .priority(index as i32)
                    .submit()
                    .unwrap(),
            );
        }
        for _ in 0..16 {
            if requests
                .iter()
                .all(|request| runtime.request(*request).unwrap().state != RequestState::Pending)
            {
                break;
            }
            runtime.step().unwrap().unwrap();
        }
        assert!(requests
            .iter()
            .all(|r| runtime.request(*r).unwrap().state == RequestState::Queued));
        let flush = runtime
            .schedule_allocation_flush(resource, SimTime::ZERO, 0)
            .unwrap();
        let mut reached_flush = false;
        for _ in 0..16 {
            let dispatch = runtime.step().unwrap().unwrap();
            if dispatch.event == flush {
                assert_eq!(dispatch.error, None);
                reached_flush = true;
                break;
            }
        }
        assert!(reached_flush);
        assert_eq!(
            requests
                .iter()
                .filter(|r| runtime.request(**r).unwrap().state == RequestState::Active)
                .count(),
            2
        );
        assert_eq!(
            requests
                .iter()
                .filter(|r| runtime.request(**r).unwrap().state == RequestState::Queued)
                .count(),
            2
        );
        let eager = runtime.create_resource(1).unwrap();
        let notify_work = runtime
            .create_work(owners[4], SimDuration::ZERO, "scan.context", 999_u64)
            .unwrap();
        runtime
            .acquire(eager)
            .owner(owners[4])
            .timed_work(notify_work)
            .submit()
            .unwrap();
        let mut reached_completion = false;
        for _ in 0..16 {
            runtime.step().unwrap().unwrap();
            if runtime.work_progress(notify_work).unwrap().state == WorkState::Completed {
                reached_completion = true;
                break;
            }
        }
        assert!(reached_completion);
        runtime
            .schedule_allocation_flush(resource, SimTime::from_ticks(12), 1)
            .unwrap();
        runtime
            .schedule_allocation_flush(resource, SimTime::from_ticks(16), 1)
            .unwrap();
        assert_eq!(
            runtime.resource_allocation_mode(eager).unwrap(),
            FlowAllocationMode::Eager
        );
        let lease = requests
            .iter()
            .find_map(|request| runtime.request(*request).ok().and_then(|r| r.lease))
            .unwrap();
        runtime.release(lease, SimTime::from_ticks(12)).unwrap();
        runtime
            .despawn_actor_at_with_scheduler_priority(owners[4], SimTime::from_ticks(13), 0)
            .unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(14), 0)
            .unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(15), 0)
            .unwrap();
        let image = runtime
            .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert!(image
            .builtins
            .queues
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, q)| q.requests.len() >= 2)));
        assert!(image
            .builtins
            .allocations
            .as_ref()
            .is_some_and(|s| s.rows.iter().any(|(_, a)| a.leases.len() >= 2)));
        assert!(image.restart_stores.iter().any(|s| !s.rows.is_empty()));
        assert!(image
            .context_stores
            .iter()
            .flat_map(|s| &s.rows)
            .any(|(_, p)| !p.is_empty()));
        assert!(image
            .restart_stores
            .iter()
            .flat_map(|s| &s.rows)
            .any(|(_, f, p)| f == "scan.factory" && !p.is_empty()));
        let mut restored = FlowRuntime::restore_checkpoint_v3(
            image.clone(),
            &codecs,
            FlowCheckpointLimits::default(),
        )
        .unwrap();
        assert_eq!(
            restored
                .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
                .unwrap(),
            image
        );
        let mut dispatched = 0;
        let mut terminal = false;
        for _ in 0..128 {
            let expected = runtime.step().unwrap();
            let actual = restored.step().unwrap();
            assert_eq!(actual, expected);
            if expected.is_none() {
                terminal = true;
                break;
            }
            dispatched += 1;
        }
        assert!(terminal);
        assert!(dispatched > 0);
        (image, codecs)
    }
    fn component_totals(i: &FlowCheckpointV3) -> (usize, usize) {
        let mut rows = 0usize;
        let mut sparse = 0usize;
        macro_rules! add {
            ($s:expr) => {
                if let Some(s) = $s {
                    rows += s.rows.len();
                    sparse += s.sparse_slots;
                }
            };
        }
        add!(i.builtins.capacities.as_ref());
        add!(i.builtins.queues.as_ref());
        add!(i.builtins.requests.as_ref());
        add!(i.builtins.deadlines.as_ref());
        add!(i.builtins.preempting.as_ref());
        add!(i.builtins.allocations.as_ref());
        add!(i.builtins.work_specs.as_ref());
        add!(i.builtins.work_roles.as_ref());
        add!(i.builtins.work_progress.as_ref());
        for s in &i.context_stores {
            rows += s.rows.len();
            sparse += s.sparse_slots;
        }
        for s in &i.restart_stores {
            rows += s.rows.len();
            sparse += s.sparse_slots;
        }
        (rows, sparse)
    }

    #[test]
    fn v3_structural_tags_0_through_13_roundtrip_wire_only() {
        let mut runtime = FlowRuntime::new();
        // WIRE ONLY: this synthetic complete command vector does not certify restore.
        let resource = runtime
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        runtime.create_resource(1).unwrap();
        runtime
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
            .unwrap();
        let mut image = runtime
            .capture_checkpoint_v3(&codecs(), FlowCheckpointLimits::default())
            .unwrap();
        let entity = EntityId::new(55, 3);
        let request = RequestId(entity);
        let work = WorkId(entity);
        let lease = LeaseId {
            request,
            revision: 7,
        };
        let commands = [
            FlowCommandV3::Submit(request),
            FlowCommandV3::Release(lease),
            FlowCommandV3::Capacity(resource, 3),
            FlowCommandV3::Remove(resource),
            FlowCommandV3::Despawn(entity),
            FlowCommandV3::Deadline(request),
            FlowCommandV3::Cancel(request),
            FlowCommandV3::Reprioritize(request, -4),
            FlowCommandV3::Completion(request, lease, 9, SimTime::from_ticks(4)),
            FlowCommandV3::AllocationFlush(resource),
            FlowCommandV3::Notify,
            FlowCommandV3::Domain(work, EventKind::custom(771)),
            FlowCommandV3::DomainControl(work, EventKind::custom(771), FlowDomainControl::Pause),
            FlowCommandV3::GrantEnabled(resource, false),
            FlowCommandV3::GrantEnabled(resource, true),
            FlowCommandV3::DomainControl(work, EventKind::custom(771), FlowDomainControl::Resume),
        ];
        image.commands = commands
            .into_iter()
            .enumerate()
            .map(|(n, c)| (EventId::new(n as u64, 0), c))
            .collect();
        image.started_frontier = None;
        image.resource_grant_flags[0].1 = false;
        assert!(image
            .resource_grant_flags
            .iter()
            .any(|(_, enabled)| *enabled));
        assert!(image
            .resource_grant_flags
            .iter()
            .any(|(_, enabled)| !*enabled));
        assert!(image
            .allocation_modes
            .iter()
            .any(|(_, mode)| *mode == FlowAllocationMode::Deferred));
        assert!(image
            .allocation_modes
            .iter()
            .any(|(_, mode)| *mode == FlowAllocationMode::Eager));
        let bytes = image
            .encode_wire_v3(FlowCheckpointWireLimits::default())
            .unwrap();
        let decoded =
            FlowCheckpointV3::decode_wire_v3(&bytes, FlowCheckpointWireLimits::default()).unwrap();
        assert_eq!(decoded, image);
        assert_eq!(
            decoded
                .encode_wire_v3(FlowCheckpointWireLimits::default())
                .unwrap(),
            bytes
        );
    }

    #[test]
    fn v3_measurement_precedes_canonical_validation_and_output_allocation() {
        let (image, _) = rich_image();
        let defaults = FlowCheckpointWireLimits::default();
        let mut measured = Writer::measure(defaults);
        encode_flow_v3(&image, &mut measured).unwrap();
        assert!(measured.payload_bytes > 0);
        let mut too_small = defaults;
        too_small.flow.max_payload_bytes = measured.payload_bytes - 1;
        V3_CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(0));
        V3_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.set(0));
        assert!(matches!(
            image.encode_wire_v3(too_small),
            Err(FlowCheckpointWireError::LimitExceeded(
                "aggregate payload byte"
            ))
        ));
        assert_eq!(V3_CANONICAL_VALIDATION_CALLS.with(|calls| calls.get()), 0);
        assert_eq!(V3_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.get()), 0);
        let mut short_bytes = defaults;
        short_bytes.max_wire_bytes = measured.len - 1;
        assert_eq!(
            image.encode_wire_v3(short_bytes).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("total byte")
        );
        assert_eq!(V3_CANONICAL_VALIDATION_CALLS.with(|calls| calls.get()), 0);
        assert_eq!(V3_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.get()), 0);
        V3_CANONICAL_VALIDATION_CALLS.with(|calls| calls.set(0));
        image.encode_wire_v3(defaults).unwrap();
        assert_eq!(V3_CANONICAL_VALIDATION_CALLS.with(|calls| calls.get()), 1);
        assert_eq!(V3_OUTPUT_ALLOCATION_CALLS.with(|calls| calls.get()), 1);
        let bytes = image.encode_wire_v3(defaults).unwrap();
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, defaults)
                .unwrap()
                .encode_wire_v3(defaults)
                .unwrap(),
            bytes
        );
    }

    #[test]
    fn v3_canonical_mode_resource_coverage_and_nested_order_errors() {
        let (image, _) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v3(limits).unwrap();
        V3_OWNED_DECODE_CALLS.with(|n| n.set(0));
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, limits).unwrap(),
            image
        );
        assert_eq!(V3_OWNED_DECODE_CALLS.with(|n| n.get()), 1);
        // Locate an entire canonical nested payload uniquely in the complete image.
        // Replace it without changing lengths, then exercise the public scan boundary.
        let reject = |canonical: &[u8], replacement: &[u8], message| {
            assert_eq!(canonical.len(), replacement.len());
            let positions: Vec<_> = bytes
                .windows(canonical.len())
                .enumerate()
                .filter_map(|(offset, value)| (value == canonical).then_some(offset))
                .collect();
            assert_eq!(positions.len(), 1);
            let mut malformed = bytes.clone();
            let offset = positions[0];
            malformed[offset..offset + canonical.len()].copy_from_slice(replacement);
            V3_OWNED_DECODE_CALLS.with(|n| n.set(0));
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&malformed, limits).unwrap_err(),
                FlowCheckpointWireError::InvalidValue(message)
            );
            assert_eq!(V3_OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        };
        let queue = image
            .builtins
            .queues
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|(_, q)| q)
            .find(|q| q.requests.len() >= 2)
            .unwrap();
        let keys: Vec<_> = queue.requests.iter().copied().collect();
        assert_eq!(keys.len(), 2);
        let encode_queue = |keys: &[PriorityKey]| {
            let mut writer = Writer::writing(limits, 128).unwrap();
            writer.count(keys.len()).unwrap();
            for key in keys {
                put_priority(&mut writer, *key).unwrap();
            }
            writer.finish()
        };
        let canonical = encode_queue(&keys);
        reject(
            &canonical,
            &encode_queue(&[keys[1], keys[0]]),
            "queue key order or duplicate",
        );
        reject(
            &canonical,
            &encode_queue(&[keys[0], keys[0]]),
            "queue key order or duplicate",
        );
        let active = image
            .builtins
            .allocations
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|(_, a)| a)
            .find(|a| a.leases.len() >= 2)
            .unwrap();
        let allocations: Vec<_> = active.leases.iter().collect();
        assert_eq!(allocations.len(), 2);
        let encode_allocations = |rows: &[(&LeaseId, &Allocation)]| {
            let mut writer = Writer::writing(limits, 512).unwrap();
            writer.count(rows.len()).unwrap();
            for (lease, allocation) in rows {
                put_lease(&mut writer, **lease).unwrap();
                put_allocation(&mut writer, allocation).unwrap();
            }
            writer.finish()
        };
        let canonical = encode_allocations(&allocations);
        reject(
            &canonical,
            &encode_allocations(&[allocations[1], allocations[0]]),
            "allocation key order",
        );
        // Keep payload widths/content unchanged: duplicate only the lease identity.
        reject(
            &canonical,
            &encode_allocations(&[allocations[0], (allocations[0].0, allocations[1].1)]),
            "allocation key order",
        );
        let order = FlowCheckpointWireError::InvalidValue("noncanonical order or duplicate entry");
        let coverage = FlowCheckpointWireError::InvalidValue("allocation modes");
        let mut mutations = Vec::new();
        let mut missing = image.clone();
        missing.allocation_modes.pop();
        mutations.push((missing, coverage.clone()));
        let mut extra = image.clone();
        extra.allocation_modes.push((
            ResourceId(EntityId::new(u64::MAX, 0)),
            FlowAllocationMode::Eager,
        ));
        mutations.push((extra, coverage.clone()));
        let mut wrong = image.clone();
        wrong.allocation_modes[0].0 .0.generation += 1;
        mutations.push((wrong, coverage));
        let mut duplicate = image.clone();
        duplicate.allocation_modes[1] = duplicate.allocation_modes[0];
        mutations.push((duplicate, order.clone()));
        let mut unsorted = image.clone();
        unsorted.allocation_modes.reverse();
        mutations.push((unsorted, order));
        let mut frontier = image.clone();
        frontier.started_frontier = None;
        mutations.push((
            frontier,
            FlowCheckpointWireError::InvalidValue("started frontier"),
        ));
        let mut flags = image.clone();
        flags.resource_grant_flags.pop();
        mutations.push((
            flags,
            FlowCheckpointWireError::InvalidValue("resource grant flags"),
        ));
        for (bad, error) in mutations {
            assert_eq!(bad.encode_wire_v3(limits).unwrap_err(), error);
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&bad), limits).unwrap_err(),
                error
            );
        }
    }

    fn collection_bound(
        image: &FlowCheckpointV3,
        name: &str,
        exact: usize,
        set: impl Fn(&mut FlowCheckpointWireLimits, usize),
        decode_error: &'static str,
    ) {
        assert!(exact > 0, "{name} positive control");
        let bytes = image
            .encode_wire_v3(FlowCheckpointWireLimits::default())
            .unwrap();
        let mut cap = FlowCheckpointWireLimits::default();
        set(&mut cap, exact);
        assert_eq!(
            image.encode_wire_v3(cap).unwrap(),
            bytes,
            "{name} exact encode"
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap(),
            *image,
            "{name} exact decode"
        );
        set(&mut cap, exact - 1);
        assert_eq!(
            image.encode_wire_v3(cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("Flow collection"),
            "{name} cap+1 encode"
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded(decode_error),
            "{name} cap+1 decode"
        );
    }
    fn unchecked_bytes(image: &FlowCheckpointV3) -> Vec<u8> {
        let mut writer = Writer::writing(FlowCheckpointWireLimits::default(), 16384).unwrap();
        encode_flow_v3(image, &mut writer).unwrap();
        writer.finish()
    }
    #[test]
    fn v3_exact_wire_record_collection_and_pending_limits() {
        let (image, _) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v3(limits).unwrap();
        let mut measure = Writer::measure(limits);
        encode_flow_v3(&image, &mut measure).unwrap();
        assert!(measure.records > 0);
        let mut cap = limits;
        cap.max_wire_bytes = bytes.len();
        cap.max_total_records = measure.records;
        assert_eq!(image.encode_wire_v3(cap).unwrap(), bytes);
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap(),
            image
        );
        cap.max_wire_bytes -= 1;
        assert_eq!(
            image.encode_wire_v3(cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("total byte")
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("total byte")
        );
        cap.max_wire_bytes += 1;
        cap.max_total_records -= 1;
        assert_eq!(
            image.encode_wire_v3(cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("aggregate record")
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("allocation mode")
        );
        collection_bound(
            &image,
            "resources",
            image.resources.len(),
            |l, v| l.flow.max_resources = v,
            "resource",
        );
        collection_bound(
            &image,
            "entities",
            image.world.slots.len(),
            |l, v| l.flow.max_entities = v,
            "world slot",
        );
        collection_bound(
            &image,
            "scheduler",
            image.scheduler.entries.len(),
            |l, v| l.flow.max_scheduler_entries = v,
            "scheduler entry",
        );
        collection_bound(
            &image,
            "requests",
            image.requests.len(),
            |l, v| l.flow.max_requests = v,
            "request",
        );
        collection_bound(
            &image,
            "actors",
            image.actors.len(),
            |l, v| l.flow.max_actors = v,
            "actor",
        );
        collection_bound(
            &image,
            "works",
            image.work_registrations.len(),
            |l, v| l.flow.max_works = v,
            "work registration",
        );
        collection_bound(
            &image,
            "commands",
            image.commands.len(),
            |l, v| l.flow.max_commands = v,
            "command",
        );
        collection_bound(
            &image,
            "notifications",
            image.notifications.len(),
            |l, v| l.flow.max_notifications = v,
            "notification",
        );
        // WIRE ONLY detached DTOs isolate collection caps shared by several fields.
        let mut detached = image.clone();
        detached.actors.clear();
        detached.actor_domains = vec![
            (EntityId::new(1000, 0), EntityId::new(2000, 0)),
            (EntityId::new(1001, 0), EntityId::new(2001, 0)),
        ];
        collection_bound(
            &detached,
            "actor domains",
            2,
            |l, v| l.flow.max_actors = v,
            "actor domain",
        );
        detached = image.clone();
        detached.world.slots.clear();
        detached.world.live_entities.clear();
        detached.world.free_indices = vec![1000, 1001];
        collection_bound(
            &detached,
            "free indices",
            2,
            |l, v| l.flow.max_entities = v,
            "free index",
        );
        detached.world.free_indices.clear();
        detached.world.live_entities = vec![EntityId::new(1000, 0), EntityId::new(1001, 0)];
        collection_bound(
            &detached,
            "live IDs",
            2,
            |l, v| l.flow.max_entities = v,
            "live entity",
        );
        let pending = image.pending_releases.len()
            + image.pending_despawns.len()
            + image
                .commands
                .iter()
                .filter(|(_, c)| {
                    matches!(
                        c,
                        FlowCommandV3::GrantEnabled(..) | FlowCommandV3::AllocationFlush(..)
                    )
                })
                .count();
        assert_eq!(image.pending_releases.len(), 1);
        assert_eq!(image.pending_despawns.len(), 1);
        assert_eq!(
            image
                .commands
                .iter()
                .filter(|(_, c)| matches!(c, FlowCommandV3::GrantEnabled(..)))
                .count(),
            2
        );
        assert_eq!(
            image
                .commands
                .iter()
                .filter(|(_, c)| matches!(c, FlowCommandV3::AllocationFlush(..)))
                .count(),
            2
        );
        collection_bound(
            &image,
            "four-kind pending",
            pending,
            |l, v| l.flow.max_pending_operations = v,
            "aggregate pending operation",
        );
        // WIRE ONLY malformed mode/flag counts independently hit their native cap.
        for modes in [false, true] {
            let mut bad = image.clone();
            if modes {
                bad.allocation_modes.push((
                    ResourceId(EntityId::new(u64::MAX, 0)),
                    FlowAllocationMode::Eager,
                ));
            } else {
                bad.resource_grant_flags
                    .push((ResourceId(EntityId::new(u64::MAX, 0)), true));
            }
            let mut cap = limits;
            cap.flow.max_resources = image.resources.len();
            assert_eq!(
                bad.encode_wire_v3(cap).unwrap_err(),
                FlowCheckpointWireError::LimitExceeded("Flow collection")
            );
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&bad), cap).unwrap_err(),
                FlowCheckpointWireError::LimitExceeded(if modes {
                    "allocation mode"
                } else {
                    "resource grant flag"
                })
            );
        }
        // Independently reach each nested request cap with a detached structural DTO.
        let mut queue_image = image.clone();
        queue_image.requests.clear();
        let mut cap = limits;
        cap.flow.max_requests = 2;
        assert!(queue_image.encode_wire_v3(cap).is_ok());
        assert!(FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&queue_image), cap).is_ok());
        cap.flow.max_requests = 1;
        assert_eq!(
            queue_image.encode_wire_v3(cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("Flow collection")
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&queue_image), cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("queue key")
        );
        queue_image.builtins.queues = None;
        queue_image.builtins.deadlines = None;
        queue_image.builtins.preempting = None;
        cap.flow.max_requests = 2;
        assert!(queue_image.encode_wire_v3(cap).is_ok());
        assert!(FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&queue_image), cap).is_ok());
        cap.flow.max_requests = 1;
        assert_eq!(
            queue_image.encode_wire_v3(cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("Flow collection")
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&queue_image), cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("allocation")
        );
        let keys: Vec<_> = image
            .builtins
            .queues
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .find(|(_, q)| q.requests.len() == 2)
            .unwrap()
            .1
            .requests
            .iter()
            .copied()
            .collect();
        assert_eq!(keys.len(), 2);
        queue_image.builtins.allocations = None;
        let entity = image.resources[0].0;
        for deadline in [true, false] {
            if deadline {
                queue_image.builtins.deadlines = Some(ComponentStoreCheckpointV1 {
                    version: 1,
                    sparse_slots: 1,
                    rows: vec![(
                        entity,
                        FlowDeadlineIndexV1 {
                            expected_len: 2,
                            entries: vec![
                                (SimTime::from_ticks(10), keys[0]),
                                (SimTime::from_ticks(11), keys[1]),
                            ],
                        },
                    )],
                });
            } else {
                queue_image.builtins.deadlines = None;
                queue_image.builtins.preempting = Some(ComponentStoreCheckpointV1 {
                    version: 1,
                    sparse_slots: 1,
                    rows: vec![(
                        entity,
                        FlowPreemptingIndexV1 {
                            expected_len: 2,
                            keys: keys.clone(),
                        },
                    )],
                });
            }
            collection_bound(
                &queue_image,
                if deadline {
                    "nested deadlines"
                } else {
                    "nested preemption"
                },
                2,
                |l, v| l.flow.max_requests = v,
                if deadline {
                    "deadline entry"
                } else {
                    "preemption key"
                },
            );
            let mut bad = queue_image.clone();
            if deadline {
                bad.builtins.deadlines.as_mut().unwrap().rows[0]
                    .1
                    .entries
                    .reverse();
            } else {
                bad.builtins.preempting.as_mut().unwrap().rows[0]
                    .1
                    .keys
                    .reverse();
            }
            let error = FlowCheckpointWireError::InvalidValue(if deadline {
                "deadline key order"
            } else {
                "preemption key order"
            });
            assert_eq!(bad.encode_wire_v3(limits).unwrap_err(), error);
            V3_OWNED_DECODE_CALLS.with(|n| n.set(0));
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&bad), limits).unwrap_err(),
                error
            );
            assert_eq!(V3_OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        }
    }

    #[test]
    fn v3_scan_rejects_bad_last_mode_and_trailing_before_ownership() {
        let (image, codecs) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v3(limits).unwrap();
        let mut last = bytes.clone();
        *last.last_mut().unwrap() = 2;
        let mut trailing = bytes.clone();
        trailing.push(0);
        for (bad, error) in [
            (last, FlowCheckpointWireError::InvalidTag),
            (trailing, FlowCheckpointWireError::TrailingBytes),
        ] {
            V3_OWNED_DECODE_CALLS.with(|n| n.set(0));
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&bad, limits).unwrap_err(),
                error
            );
            assert_eq!(V3_OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        }
        V3_OWNED_DECODE_CALLS.with(|n| n.set(0));
        let decoded = FlowCheckpointV3::decode_wire_v3(&bytes, limits).unwrap();
        assert_eq!(decoded, image);
        assert_eq!(V3_OWNED_DECODE_CALLS.with(|n| n.get()), 1);
        let mut original =
            FlowRuntime::restore_checkpoint_v3(image.clone(), &codecs, limits.flow).unwrap();
        DECODER_CALLS.with(|n| n.set(0));
        let mut restored =
            FlowRuntime::restore_checkpoint_v3(decoded, &codecs, limits.flow).unwrap();
        assert!(DECODER_CALLS.with(|n| n.get()) > 0);
        assert_eq!(
            restored
                .capture_checkpoint_v3(&codecs, limits.flow)
                .unwrap(),
            image
        );
        let mut steps = 0;
        let mut terminal = false;
        for _ in 0..128 {
            let expected = original.step().unwrap();
            assert_eq!(restored.step().unwrap(), expected);
            if expected.is_none() {
                terminal = true;
                break;
            }
            steps += 1;
        }
        assert!(terminal);
        assert!(steps > 0);
        // Native membership checks operate on complete decoded Flush commands,
        // and precede all context/template codec calls.
        let (event, resource) = image
            .commands
            .iter()
            .find_map(|(e, c)| {
                if let FlowCommandV3::AllocationFlush(r) = c {
                    Some((*e, *r))
                } else {
                    None
                }
            })
            .unwrap();
        let eager = image
            .allocation_modes
            .iter()
            .find(|(_, m)| *m == FlowAllocationMode::Eager)
            .unwrap()
            .0;
        for change in 0..5 {
            let mut bad = image.clone();
            match change {
                0 => {
                    bad.scheduler
                        .entries
                        .iter_mut()
                        .find(|s| s.id == event)
                        .unwrap()
                        .request
                        .kind = EventKind::custom(u32::MAX)
                }
                1 => {
                    bad.scheduler
                        .entries
                        .iter_mut()
                        .find(|s| s.id == event)
                        .unwrap()
                        .request
                        .entity = Some(EntityId::new(u64::MAX, 0))
                }
                2 => {
                    bad.commands
                        .iter_mut()
                        .find(|(e, _)| *e == event)
                        .unwrap()
                        .1 = FlowCommandV3::AllocationFlush(eager)
                }
                3 => {
                    bad.allocation_modes
                        .iter_mut()
                        .find(|(r, _)| *r == resource)
                        .unwrap()
                        .1 = FlowAllocationMode::Eager
                }
                _ => {
                    bad.commands
                        .iter_mut()
                        .find(|(_, c)| matches!(c, FlowCommandV3::GrantEnabled(..)))
                        .unwrap()
                        .1 = FlowCommandV3::Remove(resource)
                }
            }
            let bad_bytes = bad.encode_wire_v3(limits).unwrap();
            let bad = FlowCheckpointV3::decode_wire_v3(&bad_bytes, limits).unwrap();
            DECODER_CALLS.with(|n| n.set(0));
            assert!(matches!(
                FlowRuntime::restore_checkpoint_v3(bad, &codecs, limits.flow),
                Err(FlowCheckpointError::InvalidState(_))
            ));
            assert_eq!(DECODER_CALLS.with(|n| n.get()), 0);
        }
    }

    #[test]
    fn v3_registration_component_key_payload_limits_and_overflow() {
        let (mut image, _) = rich_image();
        // WIRE ONLY callback rows exercise all three nested registration classes.
        let callback = FlowCallbackRegistrationV1 {
            runtime_key: "bound.handler".into(),
            context_codec_key: "scan.context".into(),
            kind: None,
            variant: 0,
            callback_ids: vec![(
                "bound.on_resume".into(),
                FlowCallbackCodeV1 {
                    stable_id: "bound.resume.code".into(),
                    version: 1,
                },
            )],
        };
        image.handlers.push(callback.clone());
        image.continuations.push(callback.clone());
        image
            .continuations
            .sort_by(|a, b| a.runtime_key.cmp(&b.runtime_key));
        let mut domain = callback;
        domain.kind = Some(EventKind::custom(910));
        image.domains.push(domain);
        let limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v3(limits).unwrap();
        let mut measure = Writer::measure(limits);
        encode_flow_v3(&image, &mut measure).unwrap();
        let registrations = image.contexts.len()
            + image.handlers.len()
            + image.continuations.len()
            + image.domains.len()
            + image.context_stores.len()
            + image.restart_stores.len()
            + image.restart_store_manifest.len()
            + image
                .handlers
                .iter()
                .chain(&image.continuations)
                .chain(&image.domains)
                .map(|c| c.callback_ids.len())
                .sum::<usize>();
        let (rows, sparse) = component_totals(&image);
        collection_bound(
            &image,
            "nested registrations",
            registrations,
            |l, v| l.flow.max_registrations = v,
            "aggregate registrations",
        );
        collection_bound(
            &image,
            "component rows",
            rows,
            |l, v| l.flow.max_component_rows = v,
            "aggregate component",
        );
        collection_bound(
            &image,
            "sparse slots",
            sparse,
            |l, v| l.flow.max_sparse_slots = v,
            "aggregate component",
        );
        for (key, value) in [(true, measure.key_bytes), (false, measure.payload_bytes)] {
            assert!(value > 0);
            let mut cap = limits;
            if key {
                cap.flow.max_key_bytes = value;
            } else {
                cap.flow.max_payload_bytes = value;
            }
            assert_eq!(image.encode_wire_v3(cap).unwrap(), bytes);
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap(),
                image
            );
            if key {
                cap.flow.max_key_bytes -= 1;
            } else {
                cap.flow.max_payload_bytes -= 1;
            }
            let error = FlowCheckpointWireError::LimitExceeded(if key {
                "aggregate key byte"
            } else {
                "aggregate payload byte"
            });
            assert_eq!(image.encode_wire_v3(cap).unwrap_err(), error);
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap_err(),
                error
            );
        }
        let mut overflow = image.clone();
        overflow.builtins.capacities.as_mut().unwrap().sparse_slots = usize::MAX;
        let mut cap = limits;
        cap.flow.max_sparse_slots = usize::MAX;
        assert_eq!(
            overflow.encode_wire_v3(cap).unwrap_err(),
            FlowCheckpointWireError::IntegerOverflow
        );
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&unchecked_bytes(&overflow), cap).unwrap_err(),
            FlowCheckpointWireError::IntegerOverflow
        );
        // Resource count occurs after nonzero scheduler/world record counts:
        // u64::MAX overflows the aggregate; count2/cap1 remains a cap error.
        let mut r = Reader::new(&bytes, limits).unwrap();
        r.raw(8).unwrap();
        r.u16().unwrap();
        r.u32().unwrap();
        r.u64().unwrap();
        r.usize().unwrap();
        r.u64().unwrap();
        scan_opt_time(&mut r).unwrap();
        r.u64().unwrap();
        scan_halt(&mut r).unwrap();
        scan_scheduler(&mut r).unwrap();
        scan_world(&mut r).unwrap();
        let at = r.at;
        assert_eq!(u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()), 2);
        let mut huge = bytes.clone();
        huge[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        cap = limits;
        cap.flow.max_resources = usize::MAX;
        cap.max_total_records = usize::MAX;
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&huge, cap).unwrap_err(),
            FlowCheckpointWireError::IntegerOverflow
        );
        cap = limits;
        cap.flow.max_resources = 1;
        assert_eq!(
            FlowCheckpointV3::decode_wire_v3(&bytes, cap).unwrap_err(),
            FlowCheckpointWireError::LimitExceeded("resource")
        );
    }

    fn unique_offset(bytes: &[u8], needle: &[u8]) -> usize {
        let positions: Vec<_> = bytes
            .windows(needle.len())
            .enumerate()
            .filter_map(|(i, w)| (w == needle).then_some(i))
            .collect();
        assert_eq!(positions.len(), 1);
        positions[0]
    }
    #[test]
    fn v3_header_primitive_truncation_and_error_precedence() {
        let (image, _) = rich_image();
        let limits = FlowCheckpointWireLimits::default();
        let bytes = image.encode_wire_v3(limits).unwrap();
        let reject = |bad: &[u8], error| {
            V3_OWNED_DECODE_CALLS.with(|n| n.set(0));
            assert_eq!(
                FlowCheckpointV3::decode_wire_v3(bad, limits).unwrap_err(),
                error
            );
            assert_eq!(V3_OWNED_DECODE_CALLS.with(|n| n.get()), 0);
        };
        for magic in [MAGIC, MAGIC_V2, b"UNKNOWN\0"] {
            let mut bad = bytes.clone();
            bad[..8].copy_from_slice(magic);
            reject(&bad, FlowCheckpointWireError::InvalidTag);
        }
        for reader in [
            FlowCheckpointV1::decode_wire_v1(&bytes, limits).map(|_| ()),
            FlowCheckpointV2::decode_wire_v2(&bytes, limits).map(|_| ()),
        ] {
            assert_eq!(reader.unwrap_err(), FlowCheckpointWireError::InvalidTag);
        }
        assert_eq!(
            FlowCheckpointImage::decode_wire(&bytes, limits).unwrap(),
            FlowCheckpointImage::V3(image.clone())
        );
        let mut bad = bytes.clone();
        bad[8..10].copy_from_slice(&77u16.to_le_bytes());
        reject(&bad, FlowCheckpointWireError::UnsupportedSchema(77));
        bad = bytes.clone();
        bad[10..14].copy_from_slice(&2u32.to_le_bytes());
        reject(
            &bad,
            FlowCheckpointWireError::InvalidValue("Flow checkpoint version"),
        );
        let mut wrong_version = image.clone();
        wrong_version.version = 2;
        assert_eq!(
            wrong_version.encode_wire_v3(limits).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("Flow checkpoint version")
        );
        bad = bytes.clone();
        bad[14..22].fill(0);
        reject(
            &bad,
            FlowCheckpointWireError::InvalidValue("zero Flow limit"),
        );
        bad = bytes.clone();
        bad[38] = 2;
        reject(&bad, FlowCheckpointWireError::InvalidTag); // budget_tick tag
        let (event, command) = image
            .commands
            .iter()
            .find(|(_, c)| matches!(c, FlowCommandV3::GrantEnabled(..)))
            .unwrap();
        let mut w = Writer::writing(limits, 64).unwrap();
        put_event(&mut w, *event).unwrap();
        put_command_v3(&mut w, *command).unwrap();
        let needle = w.finish();
        let at = unique_offset(&bytes, &needle);
        bad = bytes.clone();
        bad[at + 12] = 99;
        reject(&bad, FlowCheckpointWireError::InvalidTag);
        bad = bytes.clone();
        bad[at + 25] = 2;
        reject(&bad, FlowCheckpointWireError::InvalidBoolean);
        let request = &image.builtins.requests.as_ref().unwrap().rows[0].1;
        let mut w = Writer::writing(limits, 256).unwrap();
        put_resource_request(&mut w, request).unwrap();
        let needle = w.finish();
        let request_at = unique_offset(&bytes, &needle);
        bad = bytes.clone();
        bad[request_at + 24] = 99;
        reject(&bad, FlowCheckpointWireError::InvalidTag);
        let key = b"scan.context";
        let at = bytes.windows(key.len()).position(|w| w == key).unwrap();
        bad = bytes.clone();
        bad[at] = 0xff;
        reject(&bad, FlowCheckpointWireError::InvalidUtf8);
        for end in [0, 7, 9, 13, 21, 37, bytes.len() - 1] {
            reject(&bytes[..end], FlowCheckpointWireError::Truncated);
        }
        // A schema discriminator precedes every later mode/trailing violation.
        bad = bytes.clone();
        bad[8..10].copy_from_slice(&2u16.to_le_bytes());
        *bad.last_mut().unwrap() = 9;
        bad.push(0);
        reject(&bad, FlowCheckpointWireError::UnsupportedSchema(2));
        // Opposite-version valid bytes are rejected without fallback.
        let runtime = FlowRuntime::new();
        let codecs = FlowCheckpointCodecs::new();
        let v1 = runtime
            .capture_checkpoint(&codecs, limits.flow)
            .unwrap()
            .encode_wire_v1(limits)
            .unwrap();
        reject(&v1, FlowCheckpointWireError::InvalidTag);
        let v2 = runtime
            .capture_checkpoint_v2(&codecs, limits.flow)
            .unwrap()
            .encode_wire_v2(limits)
            .unwrap();
        reject(&v2, FlowCheckpointWireError::InvalidTag);
    }
}
