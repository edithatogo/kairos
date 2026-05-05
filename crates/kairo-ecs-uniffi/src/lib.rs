#![forbid(unsafe_code)]

//! UniFFI-ready bridge crate.
//!
//! The real code generation layer will attach to this small, owned Rust facade
//! while the stable C ABI remains the source of truth.

pub use kairo_ecs_ffi::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BridgeStats {
    pub now_ticks: u64,
    pub scheduled_events: u64,
    pub dispatched_events: u64,
    pub cancelled_events: u64,
    pub pending_events: u64,
}

impl From<KairoEcsStats> for BridgeStats {
    fn from(stats: KairoEcsStats) -> Self {
        Self {
            now_ticks: stats.now_ticks,
            scheduled_events: stats.scheduled_events,
            dispatched_events: stats.dispatched_events,
            cancelled_events: stats.cancelled_events,
            pending_events: stats.pending_events,
        }
    }
}

#[derive(Debug)]
pub struct BridgeEngine {
    handle: u64,
}

impl BridgeEngine {
    pub fn new() -> Result<Self, String> {
        let handle = kairo_ecs_engine_new();
        if handle == 0 {
            Err(last_error())
        } else {
            Ok(Self { handle })
        }
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn schedule_at(&self, at_ticks: u64, priority: i32, kind: u32) -> Result<u64, String> {
        let event = kairo_ecs_schedule_at(self.handle, at_ticks, priority, kind);
        if event == 0 {
            Err(last_error())
        } else {
            Ok(event)
        }
    }

    pub fn step(&self) -> Result<(), String> {
        status_to_result(kairo_ecs_step(self.handle))
    }

    pub fn stats(&self) -> BridgeStats {
        kairo_ecs_stats(self.handle).into()
    }

    pub fn close(mut self) -> Result<(), String> {
        let handle = std::mem::take(&mut self.handle);
        status_to_result(kairo_ecs_engine_free(handle))
    }
}

impl Drop for BridgeEngine {
    fn drop(&mut self) {
        if self.handle != 0 {
            let _ = kairo_ecs_engine_free(self.handle);
            self.handle = 0;
        }
    }
}

pub fn status_to_result(status: KairoEcsStatusCode) -> Result<(), String> {
    if status == KairoEcsStatusCode::KAIRO_ECS_OK {
        Ok(())
    } else {
        Err(format!("kairo ecs ffi returned {status:?}"))
    }
}

pub fn last_error() -> String {
    "kairo ecs ffi operation failed".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_bridge_version() {
        assert_eq!(
            kairo_ecs_ffi::kairo_ecs_ffi_version(),
            KAIRO_ECS_FFI_VERSION
        );
    }

    #[test]
    fn facade_schedules_and_steps() {
        let engine = BridgeEngine::new().expect("engine");
        let event = engine.schedule_at(7, 0, 42).expect("event");
        assert_ne!(event, 0);
        engine.step().expect("step");
        assert_eq!(engine.stats().now_ticks, 7);
        engine.close().expect("close");
    }
}
