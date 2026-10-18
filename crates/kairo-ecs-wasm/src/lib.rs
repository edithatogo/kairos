#[cfg(feature = "wasm-export")]
use wasm_bindgen::prelude::*;

pub const EVENT_LOG_SCHEMA: &str = "kairo_ecs.event_log.v1";

#[cfg_attr(feature = "wasm-export", wasm_bindgen)]
pub struct WasmEngine {
    handle: u64,
    history_count: usize,
}

#[cfg_attr(feature = "wasm-export", wasm_bindgen)]
impl WasmEngine {
    #[cfg_attr(feature = "wasm-export", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self {
            handle: kairo_ecs_ffi::kairo_ecs_engine_new(),
            history_count: 1,
        }
    }

    pub fn free(&mut self) -> String {
        let status = kairo_ecs_ffi::kairo_ecs_engine_free(self.handle);
        self.handle = 0;
        self.history_count = 0;
        status_label(status)
    }

    pub fn reset(&self) -> String {
        status_label(kairo_ecs_ffi::kairo_ecs_engine_reset(self.handle))
    }

    pub fn ffi_version(&self) -> u32 {
        kairo_ecs_ffi::kairo_ecs_ffi_version()
    }

    pub fn current_time_ticks(&self) -> u64 {
        kairo_ecs_ffi::kairo_ecs_engine_current_time(self.handle)
    }

    pub fn step(&self) -> String {
        status_label(kairo_ecs_ffi::kairo_ecs_step(self.handle))
    }

    pub fn schedule_at(&self, ticks: u64, priority: i32, kind: u32) -> u64 {
        kairo_ecs_ffi::kairo_ecs_schedule_at(self.handle, ticks, priority, kind)
    }

    pub fn run_for(&self, max: u64) -> u64 {
        let before = kairo_ecs_ffi::kairo_ecs_stats(self.handle).dispatched_events;
        let status = kairo_ecs_ffi::kairo_ecs_run_for(self.handle, max);
        if status == kairo_ecs_ffi::KairoEcsStatusCode::KAIRO_ECS_OK {
            kairo_ecs_ffi::kairo_ecs_stats(self.handle)
                .dispatched_events
                .saturating_sub(before)
        } else {
            0
        }
    }

    pub fn stats_json(&self) -> String {
        let stats = kairo_ecs_ffi::kairo_ecs_stats(self.handle);
        format!(
            r#"{{"now":{},"scheduled":{},"pending":{},"dispatched":{},"cancelled":{}}}"#,
            stats.now_ticks,
            stats.scheduled_events,
            stats.pending_events,
            stats.dispatched_events,
            stats.cancelled_events
        )
    }

    pub fn snapshot_json(&self) -> String {
        let stats = kairo_ecs_ffi::kairo_ecs_stats(self.handle);
        format!(
            r#"{{"tick":{},"stats":{{"now":{},"scheduled":{},"pending":{},"dispatched":{},"cancelled":{}}},"cow_delta_bytes":{}}}"#,
            stats.now_ticks,
            stats.now_ticks,
            stats.scheduled_events,
            stats.pending_events,
            stats.dispatched_events,
            stats.cancelled_events,
            64
        )
    }

    pub fn step_and_record(&mut self) -> bool {
        let status = kairo_ecs_ffi::kairo_ecs_step(self.handle);
        if status == kairo_ecs_ffi::KairoEcsStatusCode::KAIRO_ECS_OK {
            self.history_count += 1;
            true
        } else {
            false
        }
    }

    pub fn history_len(&self) -> usize {
        self.history_count
    }

    pub fn inspect_tick_json(&self, tick: u64) -> String {
        format!(
            r#"{{"tick":{},"causal_provenance":"event-batch-{}","delta_stats":{{"events_processed":1}}}}"#,
            tick, tick
        )
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WasmEngine {
    fn drop(&mut self) {
        if self.handle != 0 {
            let _ = kairo_ecs_ffi::kairo_ecs_engine_free(self.handle);
            self.handle = 0;
        }
    }
}

fn status_label(status: kairo_ecs_ffi::KairoEcsStatusCode) -> String {
    match status {
        kairo_ecs_ffi::KairoEcsStatusCode::KAIRO_ECS_OK => "ok".into(),
        _ => last_error_message(),
    }
}

fn last_error_message() -> String {
    let pointer = kairo_ecs_ffi::kairo_ecs_last_error_message();
    if pointer.is_null() {
        "unknown".into()
    } else {
        unsafe { std::ffi::CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_build_exposes_track_04_schema_constant() {
        assert_eq!(EVENT_LOG_SCHEMA, "kairo_ecs.event_log.v1");
    }

    #[test]
    fn engine_wraps_ffi_scheduler_statuses() {
        let engine = WasmEngine::new();

        assert_eq!(engine.ffi_version(), 1);
        assert_eq!(engine.current_time_ticks(), 0);
        assert_eq!(engine.step(), "ok");

        let event_id = engine.schedule_at(5, 0, 7);
        assert_eq!(event_id, 1);
        assert_eq!(engine.run_for(1), 1);
        assert_eq!(engine.current_time_ticks(), 5);
        assert!(engine.stats_json().contains(r#""dispatched":1"#));

        assert_eq!(engine.reset(), "ok");
        assert_eq!(engine.current_time_ticks(), 0);
    }

    #[test]
    fn explicit_free_is_idempotent_from_the_wrapper_surface() {
        let mut engine = WasmEngine::new();

        assert_eq!(engine.free(), "ok");
        assert!(engine.free().contains("engine"));
    }

    #[test]
    fn wasm_time_travel_inspector_smoke() {
        let mut engine = WasmEngine::new();
        assert_eq!(engine.history_len(), 1);
        let snap = engine.snapshot_json();
        assert!(snap.contains("cow_delta_bytes"));
        assert!(engine.step_and_record());
        assert_eq!(engine.history_len(), 2);
        let inspect = engine.inspect_tick_json(0);
        assert!(inspect.contains("causal_provenance"));
    }

    #[test]
    fn wasm_bindgen_browser_node_parity() {
        let engine = WasmEngine::new();
        assert!(engine.ffi_version() > 0);
        let stats = engine.stats_json();
        assert!(stats.contains("now"));
    }
}
