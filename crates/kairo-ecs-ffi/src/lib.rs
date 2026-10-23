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

#[derive(Debug, Default)]
struct EngineState {
    scheduler: Scheduler,
    _world: World,
    stats: KairoEcsStats,
    _run_seed: u64,
    next_event_handle: u64,
    event_handles: HashMap<u64, ScheduledEventHandle>,
    event_handles_by_id: HashMap<EventId, u64>,
    // Track 70 Phase 2 (Task 2.2): monotonic per-engine event-log retention
    // feeding the populated Arrow C Data export. `next_sequence` numbers every
    // retained record; `dispatched_log` preserves dispatch order;
    // `cancelled_log` appends in cancellation order.
    next_sequence: u64,
    dispatched_log: Vec<EngineEventRecord>,
    cancelled_log: Vec<EngineEventRecord>,
}

/// Retained event columns the FFI event log can express today (see
/// `engine_event_log_records`): FFI scheduling carries no entity and no
/// payload reference, so v2 entity/payload columns stay null for this
/// producer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EngineEventRecord {
    id: EventId,
    at_ticks: u64,
    priority: i32,
    kind: u32,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ScheduledEventHandle {
    id: EventId,
    at_ticks: u64,
    priority: i32,
    kind: u32,
}

impl EngineState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn record_event(&mut self, event_id: EventId, at_ticks: u64, priority: i32, kind: u32) -> u64 {
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
                priority,
                kind,
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
        self.record_event(event_id, at_ticks, priority, kind)
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
                if let Some(handle) = self.event_handles_by_id.get(&event.id).copied() {
                    if let Some(scheduled) = self.event_handles.get(&handle) {
                        self.dispatched_log.push(EngineEventRecord {
                            id: event.id,
                            at_ticks: scheduled.at_ticks,
                            priority: scheduled.priority,
                            kind: scheduled.kind,
                            sequence: self.next_sequence,
                        });
                        self.next_sequence += 1;
                    }
                }
                self.forget_event(event.id);
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
            self.cancelled_log.push(EngineEventRecord {
                id: scheduled.id,
                at_ticks: scheduled.at_ticks,
                priority: scheduled.priority,
                kind: scheduled.kind,
                sequence: self.next_sequence,
            });
            self.next_sequence += 1;
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

#[cfg(not(feature = "native-data"))]
const ARROW_SCHEMA_FORMAT_STRUCT: &[u8] = b"+s\0";
#[cfg(not(feature = "native-data"))]
const ARROW_SCHEMA_NAME_EVENT_LOG: &[u8] = b"kairo_ecs.event_log.v1\0";

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

/// ABI-compatible mirror of the Arrow C Stream Interface struct
/// (https://arrow.apache.org/docs/format/CStreamInterface.html),
/// field order matching polars-arrow's `ArrowArrayStream` for consumption
/// through the populated stream export below. Like `KairoArrowSchema`, the
/// producer's `release` callback acts on the struct whichever mirror the
/// consumer holds, through identical offsets.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KairoArrowArrayStream {
    pub get_schema: Option<
        unsafe extern "C" fn(
            *mut KairoArrowArrayStream,
            out: *mut KairoArrowSchema,
        ) -> core::ffi::c_int,
    >,
    pub get_next: Option<
        unsafe extern "C" fn(
            *mut KairoArrowArrayStream,
            out: *mut KairoArrowArray,
        ) -> core::ffi::c_int,
    >,
    pub get_last_error: Option<unsafe extern "C" fn(*mut KairoArrowArrayStream) -> *const c_char>,
    pub release: Option<unsafe extern "C" fn(*mut KairoArrowArrayStream)>,
    pub private_data: *mut core::ffi::c_void,
}

impl Default for KairoArrowArrayStream {
    fn default() -> Self {
        Self {
            get_schema: None,
            get_next: None,
            get_last_error: None,
            release: None,
            private_data: core::ptr::null_mut(),
        }
    }
}

#[cfg(not(feature = "native-data"))]
unsafe extern "C" fn release_arrow_schema(schema: *mut KairoArrowSchema) {
    if schema.is_null() {
        return;
    }
    (*schema).format = core::ptr::null();
    (*schema).name = core::ptr::null();
    (*schema).metadata = core::ptr::null();
    (*schema).release = None;
    (*schema).private_data = core::ptr::null_mut();
}

#[cfg(not(feature = "native-data"))]
unsafe extern "C" fn release_arrow_array(array: *mut KairoArrowArray) {
    if array.is_null() {
        return;
    }
    (*array).length = 0;
    (*array).null_count = 0;
    (*array).offset = 0;
    (*array).n_buffers = 0;
    (*array).n_children = 0;
    (*array).buffers = core::ptr::null_mut();
    (*array).children = core::ptr::null_mut();
    (*array).dictionary = core::ptr::null_mut();
    (*array).release = None;
    (*array).private_data = core::ptr::null_mut();
}

/// Track 70 Phase 2 (Task 2.2): run identifier for FFI event-log exports.
/// Run/replication identity manifests are Phase 3 deliverables; this constant
/// is the FFI producer's stable run label until then.
#[cfg(feature = "native-data")]
const FFI_RUN_ID: &str = "ffi-engine";

/// Build v2 event-log records from the engine's retained log. Output order is
/// dispatch order for dispatched records, then cancellation order for
/// cancelled records; `sequence` is the engine's monotonic record counter
/// (dispatch order for dispatched events, the next free value at cancellation
/// time for cancelled ones), so it is a stable canonical sort key.
///
/// The FFI scheduling surface carries no entity and no payload reference, so
/// the v2 entity and payload columns stay null for every row of this
/// producer.
#[cfg(feature = "native-data")]
fn engine_event_log_records(engine: &EngineState) -> Vec<kairo_ecs_arrow::EventLogRecord> {
    use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2, TIME_SCALE_TICKS};

    let mut records = Vec::with_capacity(engine.dispatched_log.len() + engine.cancelled_log.len());
    for record in engine.dispatched_log.iter().copied() {
        records.push(EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: FFI_RUN_ID.to_string(),
            event_id: record.id,
            entity_id: None,
            time_ticks: u128::from(record.at_ticks),
            time_scale: TIME_SCALE_TICKS.to_string(),
            priority: record.priority,
            sequence: record.sequence,
            event_kind: format!("custom:{}", record.kind),
            status: EventStatus::Dispatched,
            payload_ref: None,
        });
    }
    for record in engine.cancelled_log.iter().copied() {
        records.push(EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: FFI_RUN_ID.to_string(),
            event_id: record.id,
            entity_id: None,
            time_ticks: u128::from(record.at_ticks),
            time_scale: TIME_SCALE_TICKS.to_string(),
            priority: record.priority,
            sequence: record.sequence,
            event_kind: format!("custom:{}", record.kind),
            status: EventStatus::Cancelled,
            payload_ref: None,
        });
    }
    records
}

/// Track 70 Phase 2 (Task 2.2): bridge an owned polars-arrow C Data Interface
/// array export to the crate's ABI-compatible `KairoArrowArray` mirror.
#[cfg(feature = "native-data")]
fn bridge_arrow_array(
    array: kairo_ecs_arrow::arrow_c_data::arrow_ffi::ArrowArray,
) -> KairoArrowArray {
    // SAFETY: `kairo_ecs_arrow::arrow_c_data::arrow_ffi::ArrowArray`
    // (polars-arrow 0.45.1) and `KairoArrowArray` are `#[repr(C)]`
    // declarations of the identical Arrow C Data Interface struct
    // (length, null_count, offset, n_buffers, n_children, buffers,
    // children, dictionary, release, private_data — same order and types,
    // see https://arrow.apache.org/docs/format/CDataInterface.html). The move
    // transfers the owned release callback and its private data as-is; the
    // release callback acts through the same offsets on whichever mirror the
    // consumer holds.
    unsafe { core::mem::transmute(array) }
}

/// Bridge an owned polars-arrow C Data Interface schema export to the crate's
/// ABI-compatible `KairoArrowSchema` mirror.
#[cfg(feature = "native-data")]
fn bridge_arrow_schema(
    schema: kairo_ecs_arrow::arrow_c_data::arrow_ffi::ArrowSchema,
) -> KairoArrowSchema {
    // SAFETY: ABI-compatible mirror of the Arrow C Data Interface
    // `ArrowSchema` struct (format, name, metadata, flags, n_children,
    // children, dictionary, release, private_data — same order and types);
    // ownership transfers unchanged, as with `bridge_arrow_array`.
    unsafe { core::mem::transmute(schema) }
}

/// Populate the v2 event-log schema export (native-data builds).
#[cfg(feature = "native-data")]
fn export_schema_body() -> Result<KairoArrowSchema, KairoEcsStatusCode> {
    use kairo_ecs_arrow::arrow_c_data as acd;

    let field = acd::event_log_v2_arrow_field();
    let schema = acd::arrow_ffi::export_field_to_c(&field);
    Ok(bridge_arrow_schema(schema))
}

/// Descriptor-only schema export (default builds; explicit capability-less
/// fallback retained until every native-data consumer enables the feature).
#[cfg(not(feature = "native-data"))]
fn export_schema_body() -> Result<KairoArrowSchema, KairoEcsStatusCode> {
    Ok(KairoArrowSchema {
        format: ARROW_SCHEMA_FORMAT_STRUCT.as_ptr().cast::<c_char>(),
        name: ARROW_SCHEMA_NAME_EVENT_LOG.as_ptr().cast::<c_char>(),
        metadata: core::ptr::null(),
        flags: 0,
        n_children: 0,
        children: core::ptr::null_mut(),
        dictionary: core::ptr::null_mut(),
        release: Some(release_arrow_schema),
        private_data: core::ptr::null_mut(),
    })
}

/// Populate the engine event log as one owned populated array export
/// (native-data builds). Failure shapes map to explicit status codes with a
/// descriptive last error: arrow conversion failures are producer-side
/// schema violations and surface as `KAIRO_ECS_ERR_INVALID_ARGUMENT`.
#[cfg(feature = "native-data")]
fn export_array_body(engine: &EngineState) -> Result<KairoArrowArray, KairoEcsStatusCode> {
    use kairo_ecs_arrow::arrow_c_data as acd;

    let records = engine_event_log_records(engine);
    let dataframe =
        kairo_ecs_arrow::polars_interop::records_to_dataframe(&records).map_err(|error| {
            set_last_error(&format!("event log export failed: {error}"));
            KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
        })?;
    let struct_array = acd::build_v2_batch_arrays(&dataframe, None).map_err(|error| {
        set_last_error(&format!("event log export failed: {error}"));
        KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
    })?;
    let exported = acd::arrow_ffi::export_array_to_c(Box::new(struct_array));
    Ok(bridge_arrow_array(exported))
}

/// Descriptor-only array export (default builds).
#[cfg(not(feature = "native-data"))]
fn export_array_body(engine: &EngineState) -> Result<KairoArrowArray, KairoEcsStatusCode> {
    let row_count = engine.stats.dispatched_events + engine.stats.cancelled_events;
    Ok(KairoArrowArray {
        length: row_count as i64,
        null_count: 0,
        offset: 0,
        n_buffers: 1,
        n_children: 0,
        buffers: core::ptr::null_mut(),
        children: core::ptr::null_mut(),
        dictionary: core::ptr::null_mut(),
        release: Some(release_arrow_array),
        private_data: core::ptr::null_mut(),
    })
}

/// Bridge an owned polars-arrow C Data Interface stream export to the crate's
/// ABI-compatible `KairoArrowArrayStream` mirror.
#[cfg(feature = "native-data")]
fn bridge_arrow_stream(
    stream: kairo_ecs_arrow::arrow_c_data::ArrowArrayStream,
) -> KairoArrowArrayStream {
    use kairo_ecs_arrow::arrow_c_data::arrow_ffi as abi;

    // SAFETY: `abi::ArrowArrayStream` (polars-arrow 0.45.1) and
    // `KairoArrowArrayStream` are `#[repr(C)]` declarations of the identical
    // Arrow C Stream Interface struct (get_schema, get_next, get_last_error,
    // release, private_data — same order and pointer types). The move
    // transfers the owned callbacks and private data as-is; the callbacks act
    // through the same offsets on whichever mirror the consumer holds.
    unsafe { core::mem::transmute::<abi::ArrowArrayStream, KairoArrowArrayStream>(stream) }
}

/// Populate the bounded multi-batch stream export (native-data builds).
#[cfg(feature = "native-data")]
fn export_stream_body(
    engine: &EngineState,
    max_batch_rows: usize,
) -> Result<KairoArrowArrayStream, KairoEcsStatusCode> {
    use kairo_ecs_arrow::arrow_c_data as acd;

    let records = engine_event_log_records(engine);
    let dataframe =
        kairo_ecs_arrow::polars_interop::records_to_dataframe(&records).map_err(|error| {
            set_last_error(&format!("event log export failed: {error}"));
            KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
        })?;
    let stream = acd::export_stream(&dataframe, max_batch_rows, None).map_err(|error| {
        set_last_error(&format!("event log export failed: {error}"));
        KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
    })?;
    Ok(bridge_arrow_stream(stream))
}

/// Explicit capability-absent stream export (default builds): the C symbol
/// stays present for ABI stability but fails with a descriptive last error
/// naming the feature instead of fabricating a stream.
#[cfg(not(feature = "native-data"))]
fn export_stream_body(
    _engine: &EngineState,
    _max_batch_rows: usize,
) -> Result<KairoArrowArrayStream, KairoEcsStatusCode> {
    set_last_error("populated Arrow C stream export requires the native-data feature");
    Err(KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT)
}

/// Export the event log as an owned populated Arrow C Stream split into
/// bounded batches of at most `max_batch_rows` rows.
///
/// Track 70 Phase 2 (Task 2.2): Rust-level export returning the polars-arrow
/// stream type directly. The Task 2.3 C-ABI symbol
/// `kairo_ecs_export_event_log_arrow_stream` wraps the same export through
/// the crate's ABI-compatible mirror. Dropping the returned stream releases
/// it exactly once (Arrow C Stream ABI).
#[cfg(feature = "native-data")]
pub fn export_event_log_arrow_stream(
    engine_handle: u64,
    max_batch_rows: usize,
) -> Result<kairo_ecs_arrow::arrow_c_data::ArrowArrayStream, KairoEcsStatusCode> {
    use kairo_ecs_arrow::arrow_c_data as acd;

    let dataframes = with_engine_mut(engine_handle, |engine| engine_event_log_records(engine))
        .and_then(|records| {
            kairo_ecs_arrow::polars_interop::records_to_dataframe(&records).map_err(|error| {
                set_last_error(&format!("event log export failed: {error}"));
                KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
            })
        })?;
    acd::export_stream(&dataframes, max_batch_rows, None).map_err(|error| {
        set_last_error(&format!("event log export failed: {error}"));
        KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT
    })
}

/// Export the canonical v2 Arrow C Data Interface schema for the engine
/// event log.
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
        *out_schema = match export_schema_body() {
            Ok(schema) => schema,
            Err(code) => return code,
        };
        KairoEcsStatusCode::KAIRO_ECS_OK
    })
}

/// Export the current engine event log as an owned populated Arrow C Data
/// Interface array.
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
        let exported = match with_engine_mut(handle, |engine| export_array_body(engine)) {
            Ok(Ok(array)) => array,
            Ok(Err(code)) => return code,
            Err(code) => return code,
        };
        *out_array = exported;
        KairoEcsStatusCode::KAIRO_ECS_OK
    })
}

/// Export the current engine event log as an owned populated Arrow C Stream
/// split into bounded batches of at most `max_batch_rows` rows.
///
/// Track 70 Phase 2 (Task 2.3): C-ABI surface for Task 2.2's Rust-level
/// stream export. `max_batch_rows` must be positive (`> 0`); larger values
/// widen the batches but never change row order. Default (non-native-data)
/// builds keep the symbol for ABI stability and fail with
/// `KAIRO_ECS_ERR_INVALID_ARGUMENT` and a feature-naming last error rather
/// than fabricating a stream.
///
/// # Safety
/// `engine` must be either null (which returns `KAIRO_ECS_ERR_INVALID_ARGUMENT`)
/// or a valid, aligned pointer to a `KairoEcsEngineHandle` (`u64`) engine
/// handle; `out_stream` must be null (same error) or a valid, writable
/// pointer to a `KairoArrowArrayStream` struct.
#[no_mangle]
pub unsafe extern "C" fn kairo_ecs_export_event_log_arrow_stream(
    engine: *mut u64,
    max_batch_rows: i64,
    out_stream: *mut KairoArrowArrayStream,
) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        if engine.is_null() || out_stream.is_null() {
            set_last_error("engine and out_stream pointers must not be null");
            return KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT;
        }
        if max_batch_rows <= 0 {
            set_last_error("max_batch_rows must be positive");
            return KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT;
        }
        let handle = *engine;
        let exported = match with_engine_mut(handle, |engine| {
            export_stream_body(engine, max_batch_rows as usize)
        }) {
            Ok(Ok(stream)) => stream,
            Ok(Err(code)) => return code,
            Err(code) => return code,
        };
        *out_stream = exported;
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

typedef struct KairoArrowArrayStream {
  int (*get_schema)(struct KairoArrowArrayStream* stream, KairoArrowSchema* out_schema);
  int (*get_next)(struct KairoArrowArrayStream* stream, KairoArrowArray* out_array);
  const char* (*get_last_error)(struct KairoArrowArrayStream* stream);
  void (*release)(struct KairoArrowArrayStream* stream);
  void* private_data;
} KairoArrowArrayStream;

KairoEcsStatusCode kairo_ecs_export_event_log_arrow_stream(KairoEcsEngineHandle* engine, int64_t max_batch_rows, KairoArrowArrayStream* out_stream);

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
