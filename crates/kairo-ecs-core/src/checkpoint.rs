//! Owned scheduler checkpoint DTOs for the experimental Track 01B seam.
//!
//! This module contains no byte codec. A higher-level checkpoint owner is
//! responsible for versioned canonical encoding and artifact limits.

use std::collections::{BinaryHeap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use kairo_ecs_types::{EventId, ScheduleRequest, SimTime};

use crate::{QueueEntry, Scheduler};

/// Maximum number of physical scheduler entries accepted by a checkpoint call.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerCheckpointLimits {
    pub max_entries: usize,
}

/// One physical heap entry, including cancelled entries awaiting heap pruning.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerCheckpointEntry {
    pub request: ScheduleRequest,
    pub id: EventId,
    pub sequence: u64,
    pub live: bool,
}

/// Version 1 owned scheduler state, ordered by insertion sequence.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchedulerCheckpointV1 {
    pub schema_version: u16,
    pub now: SimTime,
    pub next_event_index: u64,
    pub next_event_generation: u32,
    pub next_sequence: u64,
    pub scheduled_events: u64,
    pub dispatched_events: u64,
    pub cancelled_events: u64,
    pub entries: Vec<SchedulerCheckpointEntry>,
}

/// Checkpoint validation, bound, or allocation failure.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerCheckpointError {
    LimitExceeded,
    UnsupportedVersion(u16),
    InvalidState,
    AllocationFailed,
}

impl Display for SchedulerCheckpointError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LimitExceeded => formatter.write_str("scheduler checkpoint entry limit exceeded"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "unsupported scheduler checkpoint version: {version}"
                )
            }
            Self::InvalidState => formatter.write_str("invalid scheduler checkpoint state"),
            Self::AllocationFailed => formatter.write_str("scheduler checkpoint allocation failed"),
        }
    }
}

impl Error for SchedulerCheckpointError {}

impl Scheduler {
    /// Export every physical heap entry without pruning or otherwise mutating
    /// this scheduler. Records are canonicalized by insertion sequence.
    #[doc(hidden)]
    pub fn checkpoint_state(
        &self,
        limits: SchedulerCheckpointLimits,
    ) -> Result<SchedulerCheckpointV1, SchedulerCheckpointError> {
        if self.heap.len() > limits.max_entries {
            return Err(SchedulerCheckpointError::LimitExceeded);
        }

        let mut entries = Vec::new();
        entries
            .try_reserve_exact(self.heap.len())
            .map_err(|_| SchedulerCheckpointError::AllocationFailed)?;
        entries.extend(self.heap.iter().map(|entry| SchedulerCheckpointEntry {
            request: entry.request,
            id: entry.id,
            sequence: entry.sequence,
            live: self.pending.contains(&entry.id),
        }));
        entries.sort_unstable_by_key(|entry| entry.sequence);

        Ok(SchedulerCheckpointV1 {
            schema_version: 1,
            now: self.now,
            next_event_index: self.next_event_index,
            next_event_generation: self.next_event_generation,
            next_sequence: self.next_sequence,
            scheduled_events: self.scheduled_events,
            dispatched_events: self.dispatched_events,
            cancelled_events: self.cancelled_events,
            entries,
        })
    }

    /// Restore a detached scheduler state after validating the full DTO.
    ///
    /// Validation completes before any reconstructed queue allocation or state
    /// installation. Heap ordering is rebuilt from the persisted request and
    /// sequence values, so subsequent dispatch IDs and tie order are preserved.
    #[doc(hidden)]
    pub fn from_checkpoint_state(
        state: SchedulerCheckpointV1,
        limits: SchedulerCheckpointLimits,
    ) -> Result<Self, SchedulerCheckpointError> {
        if state.entries.len() > limits.max_entries {
            return Err(SchedulerCheckpointError::LimitExceeded);
        }
        if state.schema_version != 1 {
            return Err(SchedulerCheckpointError::UnsupportedVersion(
                state.schema_version,
            ));
        }
        if state.dispatched_events == 0 && state.now != SimTime::ZERO {
            return Err(SchedulerCheckpointError::InvalidState);
        }
        if state.next_event_index != state.scheduled_events
            || state.next_sequence != state.scheduled_events
            || state.next_event_generation != state.next_event_index as u32
        {
            return Err(SchedulerCheckpointError::InvalidState);
        }

        let mut previous_sequence = None;
        let mut live_count = 0_u64;
        for entry in &state.entries {
            if previous_sequence.is_some_and(|previous| entry.sequence <= previous)
                || entry.id.index != entry.sequence
                || entry.id.generation != entry.id.index as u32
                || entry.id.index >= state.next_event_index
                || entry.sequence >= state.next_sequence
            {
                return Err(SchedulerCheckpointError::InvalidState);
            }
            previous_sequence = Some(entry.sequence);
            if entry.live {
                live_count = live_count
                    .checked_add(1)
                    .ok_or(SchedulerCheckpointError::InvalidState)?;
            }
        }

        let tombstone_count = u64::try_from(state.entries.len())
            .map_err(|_| SchedulerCheckpointError::InvalidState)?
            .checked_sub(live_count)
            .ok_or(SchedulerCheckpointError::InvalidState)?;
        let accounted = state
            .dispatched_events
            .checked_add(state.cancelled_events)
            .and_then(|count| count.checked_add(live_count))
            .ok_or(SchedulerCheckpointError::InvalidState)?;
        if accounted != state.scheduled_events || tombstone_count > state.cancelled_events {
            return Err(SchedulerCheckpointError::InvalidState);
        }

        // Reserve only after all structural and arithmetic validation succeeds.
        let mut heap = BinaryHeap::new();
        heap.try_reserve(state.entries.len())
            .map_err(|_| SchedulerCheckpointError::AllocationFailed)?;
        let mut pending = HashSet::new();
        pending
            .try_reserve(
                usize::try_from(live_count).map_err(|_| SchedulerCheckpointError::InvalidState)?,
            )
            .map_err(|_| SchedulerCheckpointError::AllocationFailed)?;

        for entry in state.entries {
            if entry.live {
                pending.insert(entry.id);
            }
            heap.push(QueueEntry {
                request: entry.request,
                sequence: entry.sequence,
                id: entry.id,
            });
        }

        Ok(Self {
            heap,
            pending,
            next_event_index: state.next_event_index,
            next_event_generation: state.next_event_generation,
            next_sequence: state.next_sequence,
            now: state.now,
            scheduled_events: state.scheduled_events,
            dispatched_events: state.dispatched_events,
            cancelled_events: state.cancelled_events,
        })
    }
}
