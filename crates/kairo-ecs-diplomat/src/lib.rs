#![forbid(unsafe_code)]

//! Diplomat-ready bridge crate.
//!
//! This crate keeps the future generated Diplomat surface attached to the same
//! stable FFI lifecycle and status-code contract used by the C header.

pub use kairo_ecs_ffi::*;

#[derive(Debug)]
pub struct DiplomatEngine {
    handle: u64,
}

impl DiplomatEngine {
    pub fn new() -> Option<Self> {
        let handle = kairo_ecs_engine_new();
        (handle != 0).then_some(Self { handle })
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn schedule_at(&self, at_ticks: u64, priority: i32, kind: u32) -> Option<u64> {
        let event = kairo_ecs_schedule_at(self.handle, at_ticks, priority, kind);
        (event != 0).then_some(event)
    }

    pub fn step(&self) -> KairoEcsStatusCode {
        kairo_ecs_step(self.handle)
    }

    pub fn current_time(&self) -> u64 {
        kairo_ecs_engine_current_time(self.handle)
    }

    pub fn stats(&self) -> KairoEcsStats {
        kairo_ecs_stats(self.handle)
    }

    pub fn close(mut self) -> KairoEcsStatusCode {
        let handle = std::mem::take(&mut self.handle);
        kairo_ecs_engine_free(handle)
    }
}

impl Drop for DiplomatEngine {
    fn drop(&mut self) {
        if self.handle != 0 {
            let _ = kairo_ecs_engine_free(self.handle);
            self.handle = 0;
        }
    }
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
    fn diplomat_facade_uses_ffi_lifecycle() {
        let engine = DiplomatEngine::new().expect("engine");
        assert_ne!(engine.handle(), 0);
        assert!(engine.schedule_at(3, 0, 9).is_some());
        assert_eq!(engine.step(), KairoEcsStatusCode::KAIRO_ECS_OK);
        assert_eq!(engine.current_time(), 3);
        assert_eq!(engine.close(), KairoEcsStatusCode::KAIRO_ECS_OK);
    }
}
