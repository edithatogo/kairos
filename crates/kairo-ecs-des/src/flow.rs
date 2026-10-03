//! Experimental, single-world Flow facade. No portable checkpoint promise.
use kairo_ecs_core::Scheduler;
use kairo_ecs_state::{ComponentRegistry, World};
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Generational resource identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResourceId(pub EntityId);
/// Capacity is ECS-owned; available capacity is always derived.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCapacity {
    pub total: u32,
}
/// Experimental Flow errors; messages are not a stable compatibility key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowError {
    InvalidEntity,
    InvalidResource,
    InvalidRequest,
    TerminalRequest,
    InvalidLease,
    CapacityInUse,
    ResourceInUse,
    PastCommand,
    CounterOverflow,
    InvalidState,
}
