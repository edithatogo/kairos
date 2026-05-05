#![forbid(unsafe_code)]

/// Fixed-tick simulation time.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SimTime {
    ticks: u128,
}

impl SimTime {
    pub const ZERO: Self = Self { ticks: 0 };

    pub const fn from_ticks(ticks: u128) -> Self {
        Self { ticks }
    }

    pub const fn ticks(self) -> u128 {
        self.ticks
    }

    pub const fn checked_add(self, duration: SimDuration) -> Option<Self> {
        match self.ticks.checked_add(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }
}

/// Fixed-tick simulation duration.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SimDuration {
    ticks: u128,
}

impl SimDuration {
    pub const ZERO: Self = Self { ticks: 0 };

    pub const fn from_ticks(ticks: u128) -> Self {
        Self { ticks }
    }

    pub const fn ticks(self) -> u128 {
        self.ticks
    }
}

/// Generational event handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EventId {
    pub index: u64,
    pub generation: u32,
}

/// Generational entity handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntityId {
    pub index: u64,
    pub generation: u32,
}

/// Minimal event kind used before domain-specific DES/ABM event contracts land.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKind {
    Custom(u32),
}

/// Scheduler input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleRequest {
    pub at: SimTime,
    pub priority: i32,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

/// Event emitted by the scheduler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchedEvent {
    pub id: EventId,
    pub at: SimTime,
    pub priority: i32,
    pub sequence: u64,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

/// Result of advancing the scheduler by one step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    Dispatched(DispatchedEvent),
    Empty,
    LimitReached,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_time_uses_fixed_ticks() {
        let time = SimTime::from_ticks(10);
        let duration = SimDuration::from_ticks(5);

        assert_eq!(time.checked_add(duration), Some(SimTime::from_ticks(15)));
    }

    #[test]
    fn sim_time_checked_add_overflow_is_none() {
        let time = SimTime::from_ticks(u128::MAX);
        let duration = SimDuration::from_ticks(1);

        assert_eq!(time.checked_add(duration), None);
    }
}
