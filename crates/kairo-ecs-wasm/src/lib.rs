#![forbid(unsafe_code)]

pub const BINDING_KIND: &str = "typescript-wasm";
pub const EVENT_LOG_SCHEMA: &str = "kairo_ecs.event_log.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmStatus {
    NotConfigured,
}

impl WasmStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "not-configured",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventOrderKey {
    pub time_ticks: u128,
    pub priority: i32,
    pub sequence: u64,
}

impl Ord for EventOrderKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.time_ticks
            .cmp(&other.time_ticks)
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

impl PartialOrd for EventOrderKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub const fn native_wasm_status() -> WasmStatus {
    WasmStatus::NotConfigured
}

pub const fn binding_kind() -> &'static str {
    BINDING_KIND
}

pub const fn event_log_schema() -> &'static str {
    EVENT_LOG_SCHEMA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_browser_smoke_safe_not_configured_contract() {
        assert_eq!(native_wasm_status().as_str(), "not-configured");
        assert_eq!(binding_kind(), "typescript-wasm");
    }

    #[test]
    fn event_order_matches_core_contract() {
        let mut keys = [
            EventOrderKey {
                time_ticks: 10,
                priority: 0,
                sequence: 2,
            },
            EventOrderKey {
                time_ticks: 5,
                priority: 9,
                sequence: 1,
            },
            EventOrderKey {
                time_ticks: 10,
                priority: -1,
                sequence: 3,
            },
        ];

        keys.sort();

        assert_eq!(keys[0].time_ticks, 5);
        assert_eq!(keys[1].priority, -1);
        assert_eq!(keys[2].sequence, 2);
    }
}
