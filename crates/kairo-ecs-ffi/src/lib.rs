#![allow(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use kairo_ecs_core::Scheduler;
use kairo_ecs_state::World;
use kairo_ecs_types::{EventId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};

pub const KAIRO_ECS_FFI_VERSION: u32 = 1;

/// Default seed for FFI engine creation
const FFI_DEFAULT_SEED: u64 = 1;

/// Sentinel for tick overflow in stats
const TICKS_OVERFLOW_SENTINEL: u64 = u64::MAX;

/// Default last-error message when no error has occurred
const DEFAULT_ERROR_MSG: &str = "ok";

/// Fallback error message
const FALLBACK_ERROR_MSG: &str = "invalid error";

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KairoEcsBuffer {
    pub data: *const u8,
    pub len: usize,
}

impl Default for KairoEcsBuffer {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            len: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KairoEcsStats {
    pub now_ticks: u64,
    pub scheduled_events: u64,
    pub dispatched_events: u64,
    pub cancelled_events: u64,
    pub pending_events: u64,
}

#[cfg(feature = "numa")]
#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KairoEcsOwnership {
    Unspecified = 0,
    Borrowed = 1,
    OwnedByRust = 2,
    OwnedByCaller = 3,
}

#[cfg(feature = "numa")]
#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KairoEcsLayoutStatus {
    KAIRO_ECS_LAYOUT_OK = 0,
    KAIRO_ECS_LAYOUT_ERR_NULL = 1,
    KAIRO_ECS_LAYOUT_ERR_EMPTY = 2,
    KAIRO_ECS_LAYOUT_ERR_MISALIGNED = 3,
    KAIRO_ECS_LAYOUT_ERR_OWNERSHIP = 4,
}

#[cfg(feature = "numa")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KairoEcsZeroCopyLayout {
    pub data_addr: usize,
    pub len: usize,
    pub alignment: usize,
    pub ownership: KairoEcsOwnership,
}

#[cfg(feature = "numa")]
impl KairoEcsZeroCopyLayout {
    pub const fn new(
        data_addr: usize,
        len: usize,
        alignment: usize,
        ownership: KairoEcsOwnership,
    ) -> Self {
        Self {
            data_addr,
            len,
            alignment,
            ownership,
        }
    }

    pub fn validate(&self) -> KairoEcsLayoutStatus {
        if self.data_addr == 0 {
            return KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_ERR_NULL;
        }
        if self.len == 0 || self.alignment == 0 {
            return KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_ERR_EMPTY;
        }
        if !self.data_addr.is_multiple_of(self.alignment) {
            return KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_ERR_MISALIGNED;
        }
        if self.ownership == KairoEcsOwnership::Unspecified {
            return KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_ERR_OWNERSHIP;
        }
        KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_OK
    }
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KairoEcsStatusCode {
    KAIRO_ECS_OK = 0,
    KAIRO_ECS_ERR_INVALID_ARGUMENT = 1,
    KAIRO_ECS_ERR_NOT_FOUND = 2,
    KAIRO_ECS_ERR_ALREADY_FREED = 3,
    KAIRO_ECS_ERR_PANIC = 100,
}

#[derive(Clone, Debug)]
struct FfiEventLogRecord {
    schema_version: u16,
    run_id: String,
    event_id_index: u64,
    event_id_gen: u32,
    entity_id_index: Option<u64>,
    entity_id_gen: Option<u32>,
    time_ticks_hi: u64,
    time_ticks_lo: u64,
    time_scale: String,
    priority: i32,
    sequence: u64,
    event_kind: String,
    status: String,
    payload_ref: Option<String>,
}

#[derive(Debug, Default)]
struct EngineState {
    scheduler: Scheduler,
    _world: World,
    stats: KairoEcsStats,
    _run_seed: u64,
    next_event_handle: u64,
    event_handles: HashMap<u64, ScheduledEventHandle>,
    event_handles_by_id: HashMap<EventId, u64>,
    event_log: Vec<FfiEventLogRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ScheduledEventHandle {
    id: EventId,
    at_ticks: u64,
}

impl EngineState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn record_event(&mut self, event_id: EventId, at_ticks: u64) -> u64 {
        let handle = self
            .next_event_handle
            .checked_add(1)
            .unwrap_or(self.next_event_handle.wrapping_add(1));
        self.next_event_handle = handle;
        self.event_handles.insert(
            handle,
            ScheduledEventHandle {
                id: event_id,
                at_ticks,
            },
        );
        self.event_handles_by_id.insert(event_id, handle);
        handle
    }

    fn forget_event(&mut self, event_id: EventId) {
        if let Some(handle) = self.event_handles_by_id.remove(&event_id) {
            self.event_handles.remove(&handle);
        }
    }

    fn schedule_at(&mut self, at_ticks: u64, priority: i32, kind: u32) -> u64 {
        let event_id = self.scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(at_ticks as u128),
            priority,
            entity: None,
            kind: EventKind::Custom(kind),
        });
        self.stats.scheduled_events += 1;
        self.stats.pending_events = self.scheduler.pending_events() as u64;
        self.record_event(event_id, at_ticks)
    }

    fn schedule_after(&mut self, after_ticks: u64, priority: i32, kind: u32) -> Option<u64> {
        let at = self
            .scheduler
            .now()
            .checked_add(SimDuration::from_ticks(after_ticks as u128))
            .unwrap_or_else(|| SimTime::from_ticks(u128::MAX));
        u64::try_from(at.ticks())
            .ok()
            .map(|ticks| self.schedule_at(ticks, priority, kind))
    }

    fn apply_outcome(&mut self, outcome: StepOutcome) {
        match outcome {
            StepOutcome::Dispatched(event) => {
                self.stats.dispatched_events += 1;
                self.stats.now_ticks = self.scheduler.now().ticks() as u64;
                self.stats.pending_events = self.scheduler.pending_events() as u64;
                self.forget_event(event.id);

                let ticks = event.at.ticks();
                let hi = (ticks >> 64) as u64;
                let lo = (ticks & 0xffff_ffff_ffff_ffff) as u64;
                let (ent_idx, ent_gen) = match event.entity {
                    Some(ent) => (Some(ent.index), Some(ent.generation)),
                    None => (None, None),
                };
                let kind_str = match event.kind {
                    EventKind::Custom(k) => format!("custom:{k}"),
                };
                self.event_log.push(FfiEventLogRecord {
                    schema_version: 2,
                    run_id: "default_run".to_string(),
                    event_id_index: event.id.index,
                    event_id_gen: event.id.generation,
                    entity_id_index: ent_idx,
                    entity_id_gen: ent_gen,
                    time_ticks_hi: hi,
                    time_ticks_lo: lo,
                    time_scale: "ticks".to_string(),
                    priority: event.priority,
                    sequence: event.sequence,
                    event_kind: kind_str,
                    status: "dispatched".to_string(),
                    payload_ref: None,
                });
            }
            StepOutcome::Empty | StepOutcome::LimitReached => {
                self.stats.now_ticks = self.scheduler.now().ticks() as u64;
                self.stats.pending_events = self.scheduler.pending_events() as u64;
            }
        }
    }

    fn cancel(&mut self, event_handle: u64) -> Result<bool, KairoEcsStatusCode> {
        let scheduled = match self.event_handles.remove(&event_handle) {
            Some(scheduled) => scheduled,
            None => {
                set_last_error("event handle not found");
                return Err(KairoEcsStatusCode::KAIRO_ECS_ERR_NOT_FOUND);
            }
        };

        self.event_handles_by_id.remove(&scheduled.id);
        let cancelled = self.scheduler.cancel(scheduled.id);
        if cancelled {
            self.stats.cancelled_events += 1;
        } else {
            set_last_error("event handle not found");
        }
        self.stats.pending_events = self.scheduler.pending_events() as u64;
        Ok(cancelled)
    }

    fn next_event_handle_at_or_before(&self, time_limit: u64) -> Option<u64> {
        self.event_handles
            .values()
            .filter(|scheduled| scheduled.at_ticks <= time_limit)
            .map(|scheduled| scheduled.at_ticks)
            .min()
    }

    fn run_for(&mut self, max_events: u64) {
        for _ in 0..max_events {
            let outcome = self.scheduler.step();
            let dispatched = matches!(outcome, StepOutcome::Dispatched(_));
            self.apply_outcome(outcome);
            if !dispatched {
                break;
            }
        }
    }

    fn run_until(&mut self, time_limit: u64) {
        while self.next_event_handle_at_or_before(time_limit).is_some() {
            let outcome = self.scheduler.step();
            let dispatched = matches!(outcome, StepOutcome::Dispatched(_));
            self.apply_outcome(outcome);
            if !dispatched {
                break;
            }
        }
    }

    fn run_until_or_for(&mut self, time_limit: u64, max_events: u64) {
        for _ in 0..max_events {
            if self.next_event_handle_at_or_before(time_limit).is_none() {
                break;
            }

            let outcome = self.scheduler.step();
            let dispatched = matches!(outcome, StepOutcome::Dispatched(_));
            self.apply_outcome(outcome);
            if !dispatched {
                break;
            }
        }
    }
}

#[derive(Default)]
struct BridgeRegistry {
    next_handle: AtomicU64,
    engines: HashMap<u64, EngineState>,
}

impl BridgeRegistry {
    fn insert(&mut self, engine: EngineState) -> u64 {
        let handle = self
            .next_handle
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        self.engines.insert(handle, engine);
        handle
    }

    fn get_mut(&mut self, handle: u64) -> Option<&mut EngineState> {
        self.engines.get_mut(&handle)
    }

    fn remove(&mut self, handle: u64) -> Option<EngineState> {
        self.engines.remove(&handle)
    }
}

static REGISTRY: OnceLock<Mutex<BridgeRegistry>> = OnceLock::new();
static TELEMETRY_STORE: OnceLock<Mutex<HashMap<usize, Vec<u8>>>> = OnceLock::new();

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new(DEFAULT_ERROR_MSG).expect("static string"));
}

fn registry() -> &'static Mutex<BridgeRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(BridgeRegistry::default()))
}

fn telemetry_store() -> &'static Mutex<HashMap<usize, Vec<u8>>> {
    TELEMETRY_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_engine_mut<R>(
    handle: u64,
    f: impl FnOnce(&mut EngineState) -> R,
) -> Result<R, KairoEcsStatusCode> {
    match registry().lock() {
        Ok(mut registry) => registry.get_mut(handle).map(f).ok_or_else(|| {
            set_last_error("engine handle not found");
            KairoEcsStatusCode::KAIRO_ECS_ERR_NOT_FOUND
        }),
        Err(_) => {
            set_last_error("engine registry poisoned");
            Err(KairoEcsStatusCode::KAIRO_ECS_ERR_PANIC)
        }
    }
}

fn set_last_error(message: &str) {
    let c_string = CString::new(message)
        .unwrap_or_else(|_| CString::new(FALLBACK_ERROR_MSG).expect("static string"));
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = c_string;
    });
}

fn ffi_boundary<R>(default: R, f: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            set_last_error("panic crossed ffi boundary");
            default
        }
    }
}

fn ffi_boundary_status(f: impl FnOnce() -> KairoEcsStatusCode) -> KairoEcsStatusCode {
    ffi_boundary(KairoEcsStatusCode::KAIRO_ECS_ERR_PANIC, f)
}

fn ticks_to_u64(ticks: u128, label: &str) -> Result<u64, KairoEcsStatusCode> {
    u64::try_from(ticks).map_err(|_| {
        set_last_error(label);
        KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_ffi_version() -> u32 {
    ffi_boundary(0, || KAIRO_ECS_FFI_VERSION)
}

#[no_mangle]
pub extern "C" fn kairo_ecs_engine_new() -> u64 {
    ffi_boundary(0, || match registry().lock() {
        Ok(mut registry) => {
            let engine = EngineState {
                _run_seed: FFI_DEFAULT_SEED,
                ..EngineState::default()
            };
            registry.insert(engine)
        }
        Err(_) => {
            set_last_error("engine registry poisoned");
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_engine_free(handle: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(|| match registry().lock() {
        Ok(mut registry) => match registry.remove(handle) {
            Some(_) => KairoEcsStatusCode::KAIRO_ECS_OK,
            None => {
                set_last_error("engine already freed or unknown");
                KairoEcsStatusCode::KAIRO_ECS_ERR_ALREADY_FREED
            }
        },
        Err(_) => {
            set_last_error("engine registry poisoned");
            KairoEcsStatusCode::KAIRO_ECS_ERR_PANIC
        }
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_engine_reset(handle: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(|| match with_engine_mut(handle, |engine| engine.reset()) {
        Ok(()) => KairoEcsStatusCode::KAIRO_ECS_OK,
        Err(code) => code,
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_engine_current_time(handle: u64) -> u64 {
    ffi_boundary(0, || {
        with_engine_mut(handle, |engine| {
            ticks_to_u64(
                engine.scheduler.now().ticks(),
                "simulation time exceeds ffi tick range",
            )
        })
        .and_then(|result| result)
        .unwrap_or(0)
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_schedule_at(
    handle: u64,
    at_ticks: u64,
    priority: i32,
    kind: u32,
) -> u64 {
    ffi_boundary(0, || {
        with_engine_mut(handle, |engine| {
            engine.schedule_at(at_ticks, priority, kind)
        })
        .unwrap_or(0)
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_schedule_after(
    handle: u64,
    after_ticks: u64,
    priority: i32,
    kind: u32,
) -> u64 {
    ffi_boundary(0, || {
        with_engine_mut(handle, |engine| {
            engine
                .schedule_after(after_ticks, priority, kind)
                .ok_or_else(|| {
                    set_last_error("ffi tick range exceeded");
                    KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
                })
        })
        .and_then(|result| result)
        .unwrap_or(0)
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_cancel_event(handle: u64, event: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(
        || match with_engine_mut(handle, |engine| engine.cancel(event)) {
            Ok(Ok(true)) => KairoEcsStatusCode::KAIRO_ECS_OK,
            Ok(Ok(false)) => KairoEcsStatusCode::KAIRO_ECS_ERR_NOT_FOUND,
            Ok(Err(code)) => code,
            Err(code) => code,
        },
    )
}

#[no_mangle]
pub extern "C" fn kairo_ecs_step(handle: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        match with_engine_mut(handle, |engine| {
            let outcome = engine.scheduler.step();
            engine.apply_outcome(outcome);
        }) {
            Ok(()) => KairoEcsStatusCode::KAIRO_ECS_OK,
            Err(code) => code,
        }
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_run_for(handle: u64, max_events: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(
        || match with_engine_mut(handle, |engine| engine.run_for(max_events)) {
            Ok(()) => KairoEcsStatusCode::KAIRO_ECS_OK,
            Err(code) => code,
        },
    )
}

#[no_mangle]
pub extern "C" fn kairo_ecs_run_until(handle: u64, time_limit_ticks: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        match with_engine_mut(handle, |engine| engine.run_until(time_limit_ticks)) {
            Ok(()) => KairoEcsStatusCode::KAIRO_ECS_OK,
            Err(code) => code,
        }
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_run_until_or_for(
    handle: u64,
    time_limit_ticks: u64,
    max_events: u64,
) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        match with_engine_mut(handle, |engine| {
            engine.run_until_or_for(time_limit_ticks, max_events)
        }) {
            Ok(()) => KairoEcsStatusCode::KAIRO_ECS_OK,
            Err(code) => code,
        }
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_stats(handle: u64) -> KairoEcsStats {
    ffi_boundary(KairoEcsStats::default(), || {
        with_engine_mut(handle, |engine| {
            let mut stats = engine.stats;
            if let Ok(now_ticks) = ticks_to_u64(
                engine.scheduler.now().ticks(),
                "simulation time exceeds ffi tick range",
            ) {
                stats.now_ticks = now_ticks;
            } else {
                stats.now_ticks = TICKS_OVERFLOW_SENTINEL;
            }
            stats.pending_events = engine.scheduler.pending_events() as u64;
            stats
        })
        .unwrap_or_default()
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_last_error_message() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

pub fn kairo_ecs_last_error_string() -> String {
    LAST_ERROR.with(|slot| slot.borrow().to_string_lossy().into_owned())
}

#[no_mangle]
pub extern "C" fn kairo_ecs_telemetry_flush_ipc(handle: u64) -> KairoEcsBuffer {
    ffi_boundary(KairoEcsBuffer::default(), || {
        match with_engine_mut(handle, |engine| {
            format!(
            "{{\"now_ticks\":{},\"scheduled_events\":{},\"dispatched_events\":{},\"cancelled_events\":{},\"pending_events\":{}}}",
            engine.stats.now_ticks,
            engine.stats.scheduled_events,
            engine.stats.dispatched_events,
            engine.stats.cancelled_events,
            engine.stats.pending_events
        )
        .into_bytes()
        }) {
            Ok(bytes) => {
                let mut store = match telemetry_store().lock() {
                    Ok(store) => store,
                    Err(_) => {
                        set_last_error("telemetry store poisoned");
                        return KairoEcsBuffer::default();
                    }
                };
                let ptr = bytes.as_ptr();
                let len = bytes.len();
                store.insert(ptr as usize, bytes);
                KairoEcsBuffer { data: ptr, len }
            }
            Err(_) => KairoEcsBuffer::default(),
        }
    })
}

#[no_mangle]
pub extern "C" fn kairo_ecs_buffer_free(buffer: KairoEcsBuffer) {
    ffi_boundary((), || {
        if buffer.data.is_null() || buffer.len == 0 {
            return;
        }

        if let Ok(mut store) = telemetry_store().lock() {
            store.remove(&(buffer.data as usize));
        }
    })
}

const ARROW_SCHEMA_FORMAT_STRUCT: &[u8] = b"+s\0";
const ARROW_SCHEMA_NAME_EVENT_LOG_V2: &[u8] = b"kairo_ecs.event_log.v2\0";

struct ColumnSchemaDef {
    name: &'static [u8],
    format: &'static [u8],
    flags: i64,
}

const EVENT_LOG_COLUMNS_V2: [ColumnSchemaDef; 14] = [
    ColumnSchemaDef {
        name: b"schema_version\0",
        format: b"S\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"run_id\0",
        format: b"u\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"event_id_index\0",
        format: b"L\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"event_id_gen\0",
        format: b"I\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"entity_id_index\0",
        format: b"L\0",
        flags: 2,
    },
    ColumnSchemaDef {
        name: b"entity_id_gen\0",
        format: b"I\0",
        flags: 2,
    },
    ColumnSchemaDef {
        name: b"time_ticks_hi\0",
        format: b"L\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"time_ticks_lo\0",
        format: b"L\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"time_scale\0",
        format: b"u\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"priority\0",
        format: b"i\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"sequence\0",
        format: b"L\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"event_kind\0",
        format: b"u\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"status\0",
        format: b"u\0",
        flags: 0,
    },
    ColumnSchemaDef {
        name: b"payload_ref\0",
        format: b"u\0",
        flags: 2,
    },
];

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KairoArrowSchema {
    pub format: *const c_char,
    pub name: *const c_char,
    pub metadata: *const c_char,
    pub flags: i64,
    pub n_children: i64,
    pub children: *mut *mut KairoArrowSchema,
    pub dictionary: *mut KairoArrowSchema,
    pub release: Option<unsafe extern "C" fn(*mut KairoArrowSchema)>,
    pub private_data: *mut core::ffi::c_void,
}

impl Default for KairoArrowSchema {
    fn default() -> Self {
        Self {
            format: core::ptr::null(),
            name: core::ptr::null(),
            metadata: core::ptr::null(),
            flags: 0,
            n_children: 0,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: None,
            private_data: core::ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KairoArrowArray {
    pub length: i64,
    pub null_count: i64,
    pub offset: i64,
    pub n_buffers: i64,
    pub n_children: i64,
    pub buffers: *mut *const core::ffi::c_void,
    pub children: *mut *mut KairoArrowArray,
    pub dictionary: *mut KairoArrowArray,
    pub release: Option<unsafe extern "C" fn(*mut KairoArrowArray)>,
    pub private_data: *mut core::ffi::c_void,
}

impl Default for KairoArrowArray {
    fn default() -> Self {
        Self {
            length: 0,
            null_count: 0,
            offset: 0,
            n_buffers: 0,
            n_children: 0,
            buffers: core::ptr::null_mut(),
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: None,
            private_data: core::ptr::null_mut(),
        }
    }
}

#[allow(clippy::vec_box)]
struct SchemaPrivateData {
    _child_schemas: Vec<Box<KairoArrowSchema>>,
    children_ptrs: Vec<*mut KairoArrowSchema>,
}

unsafe extern "C" fn release_arrow_child_schema(schema: *mut KairoArrowSchema) {
    if schema.is_null() {
        return;
    }
    (*schema).release = None;
}

unsafe extern "C" fn release_arrow_schema(schema: *mut KairoArrowSchema) {
    if schema.is_null() {
        return;
    }
    if !(*schema).private_data.is_null() {
        let _ = Box::from_raw((*schema).private_data as *mut SchemaPrivateData);
        (*schema).private_data = core::ptr::null_mut();
    }
    (*schema).format = core::ptr::null();
    (*schema).name = core::ptr::null();
    (*schema).metadata = core::ptr::null();
    (*schema).n_children = 0;
    (*schema).children = core::ptr::null_mut();
    (*schema).release = None;
}

#[allow(clippy::vec_box)]
struct ArrayPrivateData {
    _child_arrays: Vec<Box<KairoArrowArray>>,
    children_ptrs: Vec<*mut KairoArrowArray>,
    _buffers_storage: Vec<Box<[*const core::ffi::c_void]>>,
    _allocated_data: Vec<Box<dyn core::any::Any>>,
}

unsafe extern "C" fn release_arrow_child_array(array: *mut KairoArrowArray) {
    if array.is_null() {
        return;
    }
    (*array).release = None;
}

unsafe extern "C" fn release_arrow_array(array: *mut KairoArrowArray) {
    if array.is_null() {
        return;
    }
    if !(*array).private_data.is_null() {
        let _ = Box::from_raw((*array).private_data as *mut ArrayPrivateData);
        (*array).private_data = core::ptr::null_mut();
    }
    (*array).length = 0;
    (*array).null_count = 0;
    (*array).offset = 0;
    (*array).n_buffers = 0;
    (*array).n_children = 0;
    (*array).buffers = core::ptr::null_mut();
    (*array).children = core::ptr::null_mut();
    (*array).release = None;
}

fn build_validity_bitmap(validity: &[bool]) -> Vec<u8> {
    let num_bytes = validity.len().div_ceil(8);
    let mut bytes = vec![0u8; num_bytes];
    for (i, &valid) in validity.iter().enumerate() {
        if valid {
            bytes[i / 8] |= 1 << (i % 8);
        }
    }
    bytes
}

fn build_utf8_buffers(strings: &[Option<&str>]) -> (Vec<u8>, Vec<i32>, Vec<u8>, usize) {
    let mut null_count = 0;
    let mut validity = Vec::with_capacity(strings.len());
    let mut offsets = Vec::with_capacity(strings.len() + 1);
    let mut data = Vec::new();
    let mut current_offset: i32 = 0;
    offsets.push(current_offset);

    for s in strings {
        match s {
            Some(text) => {
                validity.push(true);
                data.extend_from_slice(text.as_bytes());
                current_offset = current_offset
                    .checked_add(text.len() as i32)
                    .unwrap_or(current_offset);
                offsets.push(current_offset);
            }
            None => {
                null_count += 1;
                validity.push(false);
                offsets.push(current_offset);
            }
        }
    }
    let bitmap = build_validity_bitmap(&validity);
    (bitmap, offsets, data, null_count)
}

/// Export the canonical `kairo_ecs.event_log.v2` Arrow C Data Interface schema.
///
/// # Safety
/// `out_schema` must be either null (which returns `KAIRO_ECS_ERR_INVALID_ARGUMENT`)
/// or a valid, writable pointer to a `KairoArrowSchema` struct.
#[no_mangle]
pub unsafe extern "C" fn kairo_ecs_export_event_log_arrow_schema(
    out_schema: *mut KairoArrowSchema,
) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        if out_schema.is_null() {
            set_last_error("out_schema pointer must not be null");
            return KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT;
        }

        let mut child_schemas: Vec<Box<KairoArrowSchema>> = Vec::with_capacity(14);
        for col in &EVENT_LOG_COLUMNS_V2 {
            child_schemas.push(Box::new(KairoArrowSchema {
                format: col.format.as_ptr().cast::<c_char>(),
                name: col.name.as_ptr().cast::<c_char>(),
                metadata: core::ptr::null(),
                flags: col.flags,
                n_children: 0,
                children: core::ptr::null_mut(),
                dictionary: core::ptr::null_mut(),
                release: Some(release_arrow_child_schema),
                private_data: core::ptr::null_mut(),
            }));
        }

        let children_ptrs: Vec<*mut KairoArrowSchema> = child_schemas
            .iter_mut()
            .map(|b| &mut **b as *mut KairoArrowSchema)
            .collect();

        let private_data = Box::into_raw(Box::new(SchemaPrivateData {
            _child_schemas: child_schemas,
            children_ptrs: children_ptrs.clone(),
        }));

        *out_schema = KairoArrowSchema {
            format: ARROW_SCHEMA_FORMAT_STRUCT.as_ptr().cast::<c_char>(),
            name: ARROW_SCHEMA_NAME_EVENT_LOG_V2.as_ptr().cast::<c_char>(),
            metadata: core::ptr::null(),
            flags: 0,
            n_children: 14,
            children: (*private_data).children_ptrs.as_mut_ptr(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_schema),
            private_data: private_data as *mut core::ffi::c_void,
        };

        KairoEcsStatusCode::KAIRO_ECS_OK
    })
}

/// Export the current engine event log as an Arrow C Data Interface array descriptor.
///
/// # Safety
/// `engine` and `out_array` must be either null (which returns `KAIRO_ECS_ERR_INVALID_ARGUMENT`)
/// or valid, aligned pointers to a `KairoEcsEngineHandle` (`u64`) and writable `KairoArrowArray`.
#[no_mangle]
pub unsafe extern "C" fn kairo_ecs_export_event_log_arrow_array(
    engine: *mut u64,
    out_array: *mut KairoArrowArray,
) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        if engine.is_null() || out_array.is_null() {
            set_last_error("engine and out_array pointers must not be null");
            return KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT;
        }

        let handle = *engine;
        let records = match with_engine_mut(handle, |eng| eng.event_log.clone()) {
            Ok(r) => r,
            Err(code) => return code,
        };

        let row_count = records.len() as i64;

        let mut child_arrays: Vec<Box<KairoArrowArray>> = Vec::with_capacity(14);
        let mut buffers_storage: Vec<Box<[*const core::ffi::c_void]>> = Vec::with_capacity(14);
        let mut allocated_data: Vec<Box<dyn core::any::Any>> = Vec::with_capacity(30);

        // 1. schema_version (UInt16)
        let schema_ver: Vec<u16> = records.iter().map(|r| r.schema_version).collect();
        let schema_ver_ptr = schema_ver.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(schema_ver));
        let mut b1 = Box::new([core::ptr::null(), schema_ver_ptr]);
        let b1_ptr = b1.as_mut_ptr();
        buffers_storage.push(b1);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b1_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 2. run_id (Utf8)
        let run_id_strs: Vec<Option<&str>> =
            records.iter().map(|r| Some(r.run_id.as_str())).collect();
        let (r_bmp, r_off, r_dat, r_nulls) = build_utf8_buffers(&run_id_strs);
        let r_bmp_ptr = if r_nulls > 0 {
            r_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let r_off_ptr = r_off.as_ptr().cast::<core::ffi::c_void>();
        let r_dat_ptr = r_dat.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(r_bmp));
        allocated_data.push(Box::new(r_off));
        allocated_data.push(Box::new(r_dat));
        let mut b2 = Box::new([r_bmp_ptr, r_off_ptr, r_dat_ptr]);
        let b2_ptr = b2.as_mut_ptr();
        buffers_storage.push(b2);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: r_nulls as i64,
            offset: 0,
            n_buffers: 3,
            n_children: 0,
            buffers: b2_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 3. event_id_index (UInt64)
        let ev_idx: Vec<u64> = records.iter().map(|r| r.event_id_index).collect();
        let ev_idx_ptr = ev_idx.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ev_idx));
        let mut b3 = Box::new([core::ptr::null(), ev_idx_ptr]);
        let b3_ptr = b3.as_mut_ptr();
        buffers_storage.push(b3);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b3_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 4. event_id_gen (UInt32)
        let ev_gen: Vec<u32> = records.iter().map(|r| r.event_id_gen).collect();
        let ev_gen_ptr = ev_gen.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ev_gen));
        let mut b4 = Box::new([core::ptr::null(), ev_gen_ptr]);
        let b4_ptr = b4.as_mut_ptr();
        buffers_storage.push(b4);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b4_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 5. entity_id_index (UInt64, nullable)
        let ent_idx_validity: Vec<bool> = records
            .iter()
            .map(|r| r.entity_id_index.is_some())
            .collect();
        let ent_idx_nulls = ent_idx_validity.iter().filter(|&&v| !v).count();
        let ent_idx_bmp = build_validity_bitmap(&ent_idx_validity);
        let ent_idx_bmp_ptr = if ent_idx_nulls > 0 {
            ent_idx_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let ent_idx: Vec<u64> = records
            .iter()
            .map(|r| r.entity_id_index.unwrap_or(0))
            .collect();
        let ent_idx_ptr = ent_idx.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ent_idx_bmp));
        allocated_data.push(Box::new(ent_idx));
        let mut b5 = Box::new([ent_idx_bmp_ptr, ent_idx_ptr]);
        let b5_ptr = b5.as_mut_ptr();
        buffers_storage.push(b5);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: ent_idx_nulls as i64,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b5_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 6. entity_id_gen (UInt32, nullable)
        let ent_gen_validity: Vec<bool> =
            records.iter().map(|r| r.entity_id_gen.is_some()).collect();
        let ent_gen_nulls = ent_gen_validity.iter().filter(|&&v| !v).count();
        let ent_gen_bmp = build_validity_bitmap(&ent_gen_validity);
        let ent_gen_bmp_ptr = if ent_gen_nulls > 0 {
            ent_gen_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let ent_gen: Vec<u32> = records
            .iter()
            .map(|r| r.entity_id_gen.unwrap_or(0))
            .collect();
        let ent_gen_ptr = ent_gen.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ent_gen_bmp));
        allocated_data.push(Box::new(ent_gen));
        let mut b6 = Box::new([ent_gen_bmp_ptr, ent_gen_ptr]);
        let b6_ptr = b6.as_mut_ptr();
        buffers_storage.push(b6);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: ent_gen_nulls as i64,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b6_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 7. time_ticks_hi (UInt64)
        let ticks_hi: Vec<u64> = records.iter().map(|r| r.time_ticks_hi).collect();
        let ticks_hi_ptr = ticks_hi.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ticks_hi));
        let mut b7 = Box::new([core::ptr::null(), ticks_hi_ptr]);
        let b7_ptr = b7.as_mut_ptr();
        buffers_storage.push(b7);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b7_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 8. time_ticks_lo (UInt64)
        let ticks_lo: Vec<u64> = records.iter().map(|r| r.time_ticks_lo).collect();
        let ticks_lo_ptr = ticks_lo.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ticks_lo));
        let mut b8 = Box::new([core::ptr::null(), ticks_lo_ptr]);
        let b8_ptr = b8.as_mut_ptr();
        buffers_storage.push(b8);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b8_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 9. time_scale (Utf8)
        let ts_strs: Vec<Option<&str>> = records
            .iter()
            .map(|r| Some(r.time_scale.as_str()))
            .collect();
        let (ts_bmp, ts_off, ts_dat, ts_nulls) = build_utf8_buffers(&ts_strs);
        let ts_bmp_ptr = if ts_nulls > 0 {
            ts_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let ts_off_ptr = ts_off.as_ptr().cast::<core::ffi::c_void>();
        let ts_dat_ptr = ts_dat.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ts_bmp));
        allocated_data.push(Box::new(ts_off));
        allocated_data.push(Box::new(ts_dat));
        let mut b9 = Box::new([ts_bmp_ptr, ts_off_ptr, ts_dat_ptr]);
        let b9_ptr = b9.as_mut_ptr();
        buffers_storage.push(b9);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: ts_nulls as i64,
            offset: 0,
            n_buffers: 3,
            n_children: 0,
            buffers: b9_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 10. priority (Int32)
        let priority: Vec<i32> = records.iter().map(|r| r.priority).collect();
        let priority_ptr = priority.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(priority));
        let mut b10 = Box::new([core::ptr::null(), priority_ptr]);
        let b10_ptr = b10.as_mut_ptr();
        buffers_storage.push(b10);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b10_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 11. sequence (UInt64)
        let seq: Vec<u64> = records.iter().map(|r| r.sequence).collect();
        let seq_ptr = seq.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(seq));
        let mut b11 = Box::new([core::ptr::null(), seq_ptr]);
        let b11_ptr = b11.as_mut_ptr();
        buffers_storage.push(b11);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 2,
            n_children: 0,
            buffers: b11_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 12. event_kind (Utf8)
        let ek_strs: Vec<Option<&str>> = records
            .iter()
            .map(|r| Some(r.event_kind.as_str()))
            .collect();
        let (ek_bmp, ek_off, ek_dat, ek_nulls) = build_utf8_buffers(&ek_strs);
        let ek_bmp_ptr = if ek_nulls > 0 {
            ek_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let ek_off_ptr = ek_off.as_ptr().cast::<core::ffi::c_void>();
        let ek_dat_ptr = ek_dat.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(ek_bmp));
        allocated_data.push(Box::new(ek_off));
        allocated_data.push(Box::new(ek_dat));
        let mut b12 = Box::new([ek_bmp_ptr, ek_off_ptr, ek_dat_ptr]);
        let b12_ptr = b12.as_mut_ptr();
        buffers_storage.push(b12);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: ek_nulls as i64,
            offset: 0,
            n_buffers: 3,
            n_children: 0,
            buffers: b12_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 13. status (Utf8)
        let st_strs: Vec<Option<&str>> = records.iter().map(|r| Some(r.status.as_str())).collect();
        let (st_bmp, st_off, st_dat, st_nulls) = build_utf8_buffers(&st_strs);
        let st_bmp_ptr = if st_nulls > 0 {
            st_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let st_off_ptr = st_off.as_ptr().cast::<core::ffi::c_void>();
        let st_dat_ptr = st_dat.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(st_bmp));
        allocated_data.push(Box::new(st_off));
        allocated_data.push(Box::new(st_dat));
        let mut b13 = Box::new([st_bmp_ptr, st_off_ptr, st_dat_ptr]);
        let b13_ptr = b13.as_mut_ptr();
        buffers_storage.push(b13);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: st_nulls as i64,
            offset: 0,
            n_buffers: 3,
            n_children: 0,
            buffers: b13_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        // 14. payload_ref (Utf8, nullable)
        let pr_strs: Vec<Option<&str>> = records.iter().map(|r| r.payload_ref.as_deref()).collect();
        let (pr_bmp, pr_off, pr_dat, pr_nulls) = build_utf8_buffers(&pr_strs);
        let pr_bmp_ptr = if pr_nulls > 0 {
            pr_bmp.as_ptr().cast()
        } else {
            core::ptr::null()
        };
        let pr_off_ptr = pr_off.as_ptr().cast::<core::ffi::c_void>();
        let pr_dat_ptr = pr_dat.as_ptr().cast::<core::ffi::c_void>();
        allocated_data.push(Box::new(pr_bmp));
        allocated_data.push(Box::new(pr_off));
        allocated_data.push(Box::new(pr_dat));
        let mut b14 = Box::new([pr_bmp_ptr, pr_off_ptr, pr_dat_ptr]);
        let b14_ptr = b14.as_mut_ptr();
        buffers_storage.push(b14);
        child_arrays.push(Box::new(KairoArrowArray {
            length: row_count,
            null_count: pr_nulls as i64,
            offset: 0,
            n_buffers: 3,
            n_children: 0,
            buffers: b14_ptr,
            children: core::ptr::null_mut(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_child_array),
            private_data: core::ptr::null_mut(),
        }));

        let children_ptrs: Vec<*mut KairoArrowArray> = child_arrays
            .iter_mut()
            .map(|b| &mut **b as *mut KairoArrowArray)
            .collect();

        let private_data = Box::into_raw(Box::new(ArrayPrivateData {
            _child_arrays: child_arrays,
            children_ptrs: children_ptrs.clone(),
            _buffers_storage: buffers_storage,
            _allocated_data: allocated_data,
        }));

        *out_array = KairoArrowArray {
            length: row_count,
            null_count: 0,
            offset: 0,
            n_buffers: 0,
            n_children: 14,
            buffers: core::ptr::null_mut(),
            children: (*private_data).children_ptrs.as_mut_ptr(),
            dictionary: core::ptr::null_mut(),
            release: Some(release_arrow_array),
            private_data: private_data as *mut core::ffi::c_void,
        };

        KairoEcsStatusCode::KAIRO_ECS_OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERATED_HEADER: &str = r#"#ifndef KAIRO_ECS_H
#define KAIRO_ECS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t KairoEcsEngineHandle;
typedef uint64_t KairoEcsEventHandle;

typedef enum KairoEcsStatusCode {
  KAIRO_ECS_OK = 0,
  KAIRO_ECS_ERR_INVALID_ARGUMENT = 1,
  KAIRO_ECS_ERR_NOT_FOUND = 2,
  KAIRO_ECS_ERR_ALREADY_FREED = 3,
  KAIRO_ECS_ERR_PANIC = 100,
} KairoEcsStatusCode;

typedef struct KairoEcsBuffer {
  const uint8_t* data;
  size_t len;
} KairoEcsBuffer;

typedef struct KairoEcsStats {
  uint64_t now_ticks;
  uint64_t scheduled_events;
  uint64_t dispatched_events;
  uint64_t cancelled_events;
  uint64_t pending_events;
} KairoEcsStats;

uint32_t kairo_ecs_ffi_version(void);
KairoEcsEngineHandle kairo_ecs_engine_new(void);
KairoEcsStatusCode kairo_ecs_engine_free(KairoEcsEngineHandle handle);
KairoEcsStatusCode kairo_ecs_engine_reset(KairoEcsEngineHandle handle);
uint64_t kairo_ecs_engine_current_time(KairoEcsEngineHandle handle);
KairoEcsEventHandle kairo_ecs_schedule_at(KairoEcsEngineHandle handle, uint64_t at_ticks, int32_t priority, uint32_t kind);
KairoEcsEventHandle kairo_ecs_schedule_after(KairoEcsEngineHandle handle, uint64_t after_ticks, int32_t priority, uint32_t kind);
KairoEcsStatusCode kairo_ecs_cancel_event(KairoEcsEngineHandle handle, KairoEcsEventHandle event);
KairoEcsStatusCode kairo_ecs_step(KairoEcsEngineHandle handle);
KairoEcsStatusCode kairo_ecs_run_for(KairoEcsEngineHandle handle, uint64_t max_events);
KairoEcsStatusCode kairo_ecs_run_until(KairoEcsEngineHandle handle, uint64_t time_limit_ticks);
KairoEcsStatusCode kairo_ecs_run_until_or_for(KairoEcsEngineHandle handle, uint64_t time_limit_ticks, uint64_t max_events);
KairoEcsStats kairo_ecs_stats(KairoEcsEngineHandle handle);
const char* kairo_ecs_last_error_message(void);
KairoEcsBuffer kairo_ecs_telemetry_flush_ipc(KairoEcsEngineHandle handle);
void kairo_ecs_buffer_free(KairoEcsBuffer buffer);


typedef struct KairoArrowSchema {
  const char* format;
  const char* name;
  const char* metadata;
  int64_t flags;
  int64_t n_children;
  struct KairoArrowSchema** children;
  struct KairoArrowSchema* dictionary;
  void (*release)(struct KairoArrowSchema* schema);
  void* private_data;
} KairoArrowSchema;

typedef struct KairoArrowArray {
  int64_t length;
  int64_t null_count;
  int64_t offset;
  int64_t n_buffers;
  int64_t n_children;
  const void** buffers;
  struct KairoArrowArray** children;
  struct KairoArrowArray* dictionary;
  void (*release)(struct KairoArrowArray* array);
  void* private_data;
} KairoArrowArray;

KairoEcsStatusCode kairo_ecs_export_event_log_arrow_schema(KairoArrowSchema* out_schema);
KairoEcsStatusCode kairo_ecs_export_event_log_arrow_array(KairoEcsEngineHandle* engine, KairoArrowArray* out_array);

#ifdef __cplusplus
}
#endif

#endif
"#;
    fn test_trigger_panic() -> KairoEcsStatusCode {
        ffi_boundary_status(|| panic!("test panic"))
    }

    #[test]
    fn ffi_version_is_stable() {
        assert_eq!(kairo_ecs_ffi_version(), KAIRO_ECS_FFI_VERSION);
    }

    #[test]
    fn engine_lifecycle_is_explicit() {
        let handle = kairo_ecs_engine_new();
        assert_ne!(handle, 0);
        assert_eq!(
            kairo_ecs_engine_free(handle),
            KairoEcsStatusCode::KAIRO_ECS_OK
        );
        assert_eq!(
            kairo_ecs_engine_free(handle),
            KairoEcsStatusCode::KAIRO_ECS_ERR_ALREADY_FREED
        );
    }

    #[test]
    fn schedule_and_step_through_bridge() {
        let handle = kairo_ecs_engine_new();
        let event = kairo_ecs_schedule_at(handle, 10, 0, 1);
        assert_ne!(event, 0);
        assert_eq!(kairo_ecs_step(handle), KairoEcsStatusCode::KAIRO_ECS_OK);
        assert_eq!(kairo_ecs_engine_current_time(handle), 10);
        let stats = kairo_ecs_stats(handle);
        assert_eq!(stats.scheduled_events, 1);
        assert_eq!(stats.dispatched_events, 1);
        assert_eq!(stats.pending_events, 0);
        assert_eq!(
            kairo_ecs_engine_free(handle),
            KairoEcsStatusCode::KAIRO_ECS_OK
        );
    }

    #[test]
    fn panic_boundary_returns_panic_status() {
        assert_eq!(
            test_trigger_panic(),
            KairoEcsStatusCode::KAIRO_ECS_ERR_PANIC
        );
    }

    #[test]
    fn canonical_header_matches_generated_surface() {
        let canonical = include_str!("../../../include/kairo_ecs.h");
        assert_eq!(canonical.replace("\r\n", "\n"), GENERATED_HEADER);
    }

    #[cfg(feature = "numa")]
    #[test]
    fn zero_copy_layout_validation_rejects_misaligned_or_unowned_buffers() {
        let aligned = KairoEcsZeroCopyLayout::new(0x1000, 128, 64, KairoEcsOwnership::Borrowed);
        assert_eq!(
            aligned.validate(),
            KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_OK
        );

        let misaligned = KairoEcsZeroCopyLayout::new(0x1001, 128, 64, KairoEcsOwnership::Borrowed);
        assert_eq!(
            misaligned.validate(),
            KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_ERR_MISALIGNED
        );

        let unowned = KairoEcsZeroCopyLayout::new(0x1000, 128, 64, KairoEcsOwnership::Unspecified);
        assert_eq!(
            unowned.validate(),
            KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_ERR_OWNERSHIP
        );
    }
}
