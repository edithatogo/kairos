#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const MAX_U32: u64 = u32::MAX as u64;
pub const MAX_U64: u128 = u64::MAX as u128;
pub const TYPES_DTO_VERSION_V1: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypesError {
    TicksOverflow,
    IndexOverflow,
    GenerationOverflow,
    PriorityOverflow,
    UnsupportedDtoVersion(u16),
}

impl Display for TypesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TicksOverflow => f.write_str("ticks must fit in an unsigned 128-bit integer"),
            Self::IndexOverflow => f.write_str("index must fit in an unsigned 64-bit integer"),
            Self::GenerationOverflow => {
                f.write_str("generation must fit in an unsigned 32-bit integer")
            }
            Self::PriorityOverflow => f.write_str("priority must fit in a signed 32-bit integer"),
            Self::UnsupportedDtoVersion(version) => {
                write!(f, "unsupported DTO version: {version}")
            }
        }
    }
}

impl Error for TypesError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Versioned<T> {
    pub version: u16,
    pub value: T,
}

impl<T> Versioned<T> {
    pub const fn new(version: u16, value: T) -> Self {
        Self { version, value }
    }
}

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

    pub const fn checked_sub(self, duration: SimDuration) -> Option<Self> {
        match self.ticks.checked_sub(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    pub const fn saturating_add(self, duration: SimDuration) -> Self {
        Self {
            ticks: self.ticks.saturating_add(duration.ticks),
        }
    }

    pub const fn duration_since(self, earlier: Self) -> Option<SimDuration> {
        match self.ticks.checked_sub(earlier.ticks) {
            Some(ticks) => Some(SimDuration { ticks }),
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

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.ticks.checked_add(other.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.ticks.checked_sub(other.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }
}

/// Generational event handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventId {
    pub index: u64,
    pub generation: u32,
}

impl EventId {
    pub const fn new(index: u64, generation: u32) -> Self {
        Self { index, generation }
    }

    pub fn try_new(index: u128, generation: u128) -> Result<Self, TypesError> {
        let index = u64::try_from(index).map_err(|_| TypesError::IndexOverflow)?;
        let generation = u32::try_from(generation).map_err(|_| TypesError::GenerationOverflow)?;
        Ok(Self { index, generation })
    }
}

/// Generational entity handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId {
    pub index: u64,
    pub generation: u32,
}

impl EntityId {
    pub const fn new(index: u64, generation: u32) -> Self {
        Self { index, generation }
    }

    pub fn try_new(index: u128, generation: u128) -> Result<Self, TypesError> {
        let index = u64::try_from(index).map_err(|_| TypesError::IndexOverflow)?;
        let generation = u32::try_from(generation).map_err(|_| TypesError::GenerationOverflow)?;
        Ok(Self { index, generation })
    }
}

/// Scheduler-visible event class.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventKind {
    Custom(u32),
}

impl EventKind {
    pub const fn custom(code: u32) -> Self {
        Self::Custom(code)
    }

    pub const fn code(self) -> u32 {
        match self {
            Self::Custom(code) => code,
        }
    }
}

/// Scheduler input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduleRequest {
    pub at: SimTime,
    pub priority: i32,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

impl ScheduleRequest {
    pub fn try_new(
        at: SimTime,
        priority: i64,
        entity: Option<EntityId>,
        kind: EventKind,
    ) -> Result<Self, TypesError> {
        let priority = i32::try_from(priority).map_err(|_| TypesError::PriorityOverflow)?;
        Ok(Self {
            at,
            priority,
            entity,
            kind,
        })
    }
}

/// Event emitted by the scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchedEvent {
    pub id: EventId,
    pub at: SimTime,
    pub priority: i32,
    pub sequence: u64,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

impl DispatchedEvent {
    pub const fn new(
        id: EventId,
        at: SimTime,
        priority: i32,
        sequence: u64,
        entity: Option<EntityId>,
        kind: EventKind,
    ) -> Self {
        Self {
            id,
            at,
            priority,
            sequence,
            entity,
            kind,
        }
    }
}

/// Result of advancing the scheduler by one step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    Dispatched(DispatchedEvent),
    Empty,
    LimitReached,
}

impl StepOutcome {
    pub const fn dispatched(event: DispatchedEvent) -> Self {
        Self::Dispatched(event)
    }

    pub const fn is_dispatched(&self) -> bool {
        matches!(self, Self::Dispatched(_))
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub const fn is_limit_reached(&self) -> bool {
        matches!(self, Self::LimitReached)
    }

    pub const fn dispatched_event(&self) -> Option<&DispatchedEvent> {
        match self {
            Self::Dispatched(event) => Some(event),
            Self::Empty | Self::LimitReached => None,
        }
    }

    pub fn into_dispatched(self) -> Option<DispatchedEvent> {
        match self {
            Self::Dispatched(event) => Some(event),
            Self::Empty | Self::LimitReached => None,
        }
    }
}

impl Ord for ScheduleRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.at
            .cmp(&other.at)
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| self.entity.cmp(&other.entity))
            .then_with(|| self.kind.cmp(&other.kind))
    }
}

impl PartialOrd for ScheduleRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DispatchedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.at
            .cmp(&other.at)
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| self.sequence.cmp(&other.sequence))
            .then_with(|| self.id.cmp(&other.id))
            .then_with(|| self.entity.cmp(&other.entity))
            .then_with(|| self.kind.cmp(&other.kind))
    }
}

impl PartialOrd for DispatchedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_time_uses_fixed_ticks() {
        let time = SimTime::from_ticks(10);
        let duration = SimDuration::from_ticks(5);

        assert_eq!(time.checked_add(duration), Some(SimTime::from_ticks(15)));
        assert_eq!(time.checked_sub(duration), Some(SimTime::from_ticks(5)));
        assert_eq!(
            SimTime::from_ticks(15).duration_since(time),
            Some(SimDuration::from_ticks(5))
        );
    }

    #[test]
    fn sim_time_checked_add_overflow_is_none() {
        let time = SimTime::from_ticks(u128::MAX);
        let duration = SimDuration::from_ticks(1);

        assert_eq!(time.checked_add(duration), None);
        assert_eq!(
            time.saturating_add(duration),
            SimTime::from_ticks(u128::MAX)
        );
    }

    #[test]
    fn handles_are_deterministically_ordered() {
        let low = EventId::new(1, 0);
        let high = EventId::new(2, 0);
        let next_generation = EventId::new(1, 1);

        assert!(low < high);
        assert!(low < next_generation);
        assert_eq!(EntityId::try_new(3, 4).unwrap(), EntityId::new(3, 4));
    }

    #[test]
    fn constructors_validate_wide_inputs() {
        assert_eq!(
            EventId::try_new(u128::from(u64::MAX), u128::from(u32::MAX)).unwrap(),
            EventId::new(u64::MAX, u32::MAX)
        );
        assert_eq!(
            EntityId::try_new(u128::from(u64::MAX), u128::from(u32::MAX)).unwrap(),
            EntityId::new(u64::MAX, u32::MAX)
        );
        assert!(matches!(
            EventId::try_new(u128::from(u64::MAX) + 1, 0),
            Err(TypesError::IndexOverflow)
        ));
        assert!(matches!(
            EntityId::try_new(0, u128::from(u32::MAX) + 1),
            Err(TypesError::GenerationOverflow)
        ));
    }

    #[test]
    fn schedule_request_and_dispatch_are_stable_and_ordered() {
        let entity = EntityId::new(7, 2);
        let request = ScheduleRequest::try_new(
            SimTime::from_ticks(12),
            4,
            Some(entity),
            EventKind::custom(9),
        )
        .unwrap();
        let id = EventId::new(33, 1);
        let dispatched = DispatchedEvent::new(
            id,
            request.at,
            request.priority,
            8,
            request.entity,
            request.kind,
        );

        assert_eq!(request.at.ticks(), 12);
        assert_eq!(request.priority, 4);
        assert_eq!(request.entity, Some(entity));
        assert_eq!(request.kind.code(), 9);
        assert_eq!(dispatched.id, id);
        assert!(dispatched >= DispatchedEvent::new(id, request.at, 3, 7, None, request.kind));
        assert!(StepOutcome::dispatched(dispatched).is_dispatched());
    }

    #[test]
    fn request_construction_validates_priority_range() {
        assert!(matches!(
            ScheduleRequest::try_new(
                SimTime::ZERO,
                i64::from(i32::MAX) + 1,
                None,
                EventKind::custom(0)
            ),
            Err(TypesError::PriorityOverflow)
        ));
    }

    #[test]
    fn versioned_wrapper_carries_version_and_value() {
        let versioned = Versioned::new(TYPES_DTO_VERSION_V1, SimTime::ZERO);

        assert_eq!(versioned.version, TYPES_DTO_VERSION_V1);
        assert_eq!(versioned.value, SimTime::ZERO);
    }

    #[test]
    fn step_outcome_accessors_work() {
        let event = DispatchedEvent::new(
            EventId::new(1, 0),
            SimTime::from_ticks(2),
            0,
            3,
            None,
            EventKind::custom(4),
        );
        let dispatched = StepOutcome::Dispatched(event);

        assert!(dispatched.is_dispatched());
        assert!(!dispatched.is_empty());
        assert!(!dispatched.is_limit_reached());
        assert_eq!(dispatched.dispatched_event(), Some(&event));
        assert_eq!(dispatched.into_dispatched(), Some(event));
        assert!(StepOutcome::Empty.is_empty());
        assert!(StepOutcome::LimitReached.is_limit_reached());
    }
}
