use std::{
    collections::BinaryHeap,
    error::Error,
    fmt::{self, Display},
};

use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime};
use serde::{Deserialize, Serialize};

use crate::{QueueEntry, Scheduler};

const FORMAT_VERSION: u16 = 1;

/// Failure to encode or restore a durable scheduler checkpoint.
#[derive(Debug)]
pub enum SchedulerCheckpointError {
    /// The input did not decode as the checkpoint schema.
    Json(serde_json::Error),
    /// The decoder does not support the encoded version.
    UnsupportedVersion(u16),
    /// The decoded checkpoint cannot represent a valid pending queue.
    InvalidState(&'static str),
}

impl Display for SchedulerCheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "scheduler checkpoint JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported scheduler checkpoint JSON version {version}")
            }
            Self::InvalidState(reason) => write!(f, "invalid scheduler checkpoint: {reason}"),
        }
    }
}

impl Error for SchedulerCheckpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct CheckpointVersion {
    version: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointV1 {
    version: u16,
    now_ticks: u128,
    scheduled_events: u64,
    dispatched_events: u64,
    cancelled_events: u64,
    pending: Vec<PendingEventV1>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingEventV1 {
    sequence: u64,
    at_ticks: u128,
    priority: i32,
    entity: Option<(u64, u32)>,
    kind: u32,
}

impl Scheduler {
    /// Experimental Rust API: encode the scheduler's live queue, clock and
    /// counters as deterministic versioned JSON.
    ///
    /// Persist these bytes alongside the model and RNG state at the same
    /// event boundary. The caller owns durable publication and input integrity.
    ///
    /// # Errors
    /// Returns an error if the schedule counter is exhausted or encoding fails.
    pub fn checkpoint_json(&self) -> Result<Vec<u8>, SchedulerCheckpointError> {
        require_schedule_capacity(self.scheduled_events)?;
        let mut pending: Vec<_> = self
            .heap
            .iter()
            .filter(|entry| self.pending.contains(&entry.id))
            .map(|entry| PendingEventV1 {
                sequence: entry.sequence,
                at_ticks: entry.request.at.ticks(),
                priority: entry.request.priority,
                entity: entry.request.entity.map(|id| (id.index, id.generation)),
                kind: entry.request.kind.code(),
            })
            .collect();
        pending.sort_unstable_by_key(|entry| entry.sequence);

        serde_json::to_vec(&CheckpointV1 {
            version: FORMAT_VERSION,
            now_ticks: self.now.ticks(),
            scheduled_events: self.scheduled_events,
            dispatched_events: self.dispatched_events,
            cancelled_events: self.cancelled_events,
            pending,
        })
        .map_err(SchedulerCheckpointError::Json)
    }

    /// Experimental Rust API: restore an independent scheduler from checkpoint
    /// bytes. Feed trusted, integrity-checked bytes from the matching model run.
    ///
    /// # Errors
    /// Rejects malformed JSON, an unsupported version, inconsistent event
    /// counts or clock, an exhausted schedule counter, or pending sequences
    /// outside the canonical order and range.
    pub fn from_checkpoint_json(bytes: &[u8]) -> Result<Self, SchedulerCheckpointError> {
        let CheckpointVersion { version } =
            serde_json::from_slice(bytes).map_err(SchedulerCheckpointError::Json)?;
        if version != FORMAT_VERSION {
            return Err(SchedulerCheckpointError::UnsupportedVersion(version));
        }
        let checkpoint: CheckpointV1 =
            serde_json::from_slice(bytes).map_err(SchedulerCheckpointError::Json)?;
        let scheduled = checkpoint.scheduled_events;
        require_schedule_capacity(scheduled)?;
        if checkpoint.dispatched_events == 0 && checkpoint.now_ticks != 0 {
            return Err(SchedulerCheckpointError::InvalidState(
                "the clock can advance only after an event dispatch",
            ));
        }
        let pending = checkpoint.pending;
        let expected_pending = scheduled
            .checked_sub(checkpoint.dispatched_events)
            .and_then(|remaining| remaining.checked_sub(checkpoint.cancelled_events));
        if expected_pending != Some(pending.len() as u64) {
            return Err(SchedulerCheckpointError::InvalidState(
                "event counts do not match the pending queue",
            ));
        }
        if pending
            .windows(2)
            .any(|entries| entries[0].sequence >= entries[1].sequence)
            || pending
                .last()
                .is_some_and(|entry| entry.sequence >= scheduled)
        {
            return Err(SchedulerCheckpointError::InvalidState(
                "pending sequences must be unique, increasing and below the next sequence",
            ));
        }

        // In v1, every schedule assigns the same zero-based index and sequence.
        // The generation is that counter modulo 2^32 even after cancel or dispatch.
        let entries: Vec<_> = pending
            .into_iter()
            .map(|entry| QueueEntry {
                request: ScheduleRequest {
                    at: SimTime::from_ticks(entry.at_ticks),
                    priority: entry.priority,
                    entity: entry
                        .entity
                        .map(|(index, generation)| EntityId::new(index, generation)),
                    kind: EventKind::custom(entry.kind),
                },
                sequence: entry.sequence,
                id: EventId::new(entry.sequence, entry.sequence as u32),
            })
            .collect();
        let pending = entries.iter().map(|entry| entry.id).collect();
        Ok(Self {
            heap: BinaryHeap::from(entries),
            pending,
            next_event_index: scheduled,
            next_event_generation: scheduled as u32,
            next_sequence: scheduled,
            now: SimTime::from_ticks(checkpoint.now_ticks),
            scheduled_events: scheduled,
            dispatched_events: checkpoint.dispatched_events,
            cancelled_events: checkpoint.cancelled_events,
        })
    }
}

fn require_schedule_capacity(scheduled: u64) -> Result<(), SchedulerCheckpointError> {
    if scheduled == u64::MAX {
        return Err(SchedulerCheckpointError::InvalidState(
            "the schedule counter is exhausted",
        ));
    }
    Ok(())
}
