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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KairoEcsBuffer {
    pub data: *const u8,
    pub len: usize,
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
    event_handles: HashMap<u64, EventId>,
    event_handles_by_id: HashMap<EventId, u64>,
}

impl EngineState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn record_event(&mut self, event_id: EventId) -> u64 {
        let handle = self
            .next_event_handle
            .checked_add(1)
            .unwrap_or(self.next_event_handle.wrapping_add(1));
        self.next_event_handle = handle;
        self.event_handles.insert(handle, event_id);
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
        self.record_event(event_id)
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
            }
            StepOutcome::Empty | StepOutcome::LimitReached => {
                self.stats.now_ticks = self.scheduler.now().ticks() as u64;
                self.stats.pending_events = self.scheduler.pending_events() as u64;
            }
        }
    }

    fn cancel(&mut self, event_handle: u64) -> Result<bool, KairoEcsStatusCode> {
        let event = match self.event_handles.remove(&event_handle) {
            Some(event) => event,
            None => {
                set_last_error("event handle not found");
                return Err(KairoEcsStatusCode::KAIRO_ECS_ERR_NOT_FOUND);
            }
        };

        self.event_handles_by_id.remove(&event);
        let cancelled = self.scheduler.cancel(event);
        if cancelled {
            self.stats.cancelled_events += 1;
        }
        self.stats.pending_events = self.scheduler.pending_events() as u64;
        Ok(cancelled)
    }

    fn run_for(&mut self, max_events: u64) -> StepOutcome {
        let outcome = self.scheduler.run_for(max_events);
        self.apply_outcome(outcome.clone());
        outcome
    }

    fn run_until(&mut self, time_limit: u64) -> StepOutcome {
        let outcome = self
            .scheduler
            .run_until(SimTime::from_ticks(time_limit as u128));
        self.apply_outcome(outcome.clone());
        outcome
    }

    fn run_until_or_for(&mut self, time_limit: u64, max_events: u64) -> StepOutcome {
        let outcome = self
            .scheduler
            .run_until_or_for(SimTime::from_ticks(time_limit as u128), max_events);
        self.apply_outcome(outcome.clone());
        outcome
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
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("ok").expect("static string"));
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
        .unwrap_or_else(|_| CString::new("invalid error").expect("static string"));
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
            let mut engine = EngineState::default();
            engine._run_seed = 1;
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
            Ok(_) => KairoEcsStatusCode::KAIRO_ECS_OK,
            Err(code) => code,
        },
    )
}

#[no_mangle]
pub extern "C" fn kairo_ecs_run_until(handle: u64, time_limit_ticks: u64) -> KairoEcsStatusCode {
    ffi_boundary_status(|| {
        match with_engine_mut(handle, |engine| engine.run_until(time_limit_ticks)) {
            Ok(_) => KairoEcsStatusCode::KAIRO_ECS_OK,
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
            Ok(_) => KairoEcsStatusCode::KAIRO_ECS_OK,
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
                stats.now_ticks = u64::MAX;
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
}
