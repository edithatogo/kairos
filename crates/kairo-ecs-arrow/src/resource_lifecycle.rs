//! Optional Arrow encoding for immutable experimental Flow lifecycle records.
use arrow_array::builder::{
    FixedSizeBinaryBuilder, Int32Builder, StringBuilder, UInt16Builder, UInt32Builder,
    UInt64Builder,
};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use kairo_ecs_des::{
    LeaseId, LifecycleRecord, LifecycleTransition, PreemptionStrategy, RequestState,
};
use kairo_ecs_types::EntityId;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

pub const RESOURCE_LIFECYCLE_STREAM: &str = "kairo_ecs.resource_lifecycle.v1";
const SCHEMA_VERSION: u16 = 1;
const TIME_SCALE: &str = "ticks";

const FINGERPRINT: &str = "kairo_ecs.resource_lifecycle.v1;major=1;fields=schema_version:UInt16:required|run_id:Utf8:required|causal_event_id:FixedSizeBinary(12):required|transition_ordinal:UInt32:required|time_ticks:FixedSizeBinary(16):required|time_scale:Utf8:required|resource_id:FixedSizeBinary(12):required|request_id:FixedSizeBinary(12):required|owner_id:FixedSizeBinary(12):required|work_id:FixedSizeBinary(12):nullable|lease_revision:UInt64:nullable|causal_lease_revision:UInt64:nullable|transition:Utf8:required|request_state:Utf8:required|strategy:Utf8:nullable|preemptor_request_id:FixedSizeBinary(12):nullable|priority:Int32:required|queue_len:UInt32:required|active_count:UInt32:required|capacity:UInt32:required|original_duration_ticks:FixedSizeBinary(16):nullable|useful_elapsed_ticks:FixedSizeBinary(16):nullable|remaining_ticks:FixedSizeBinary(16):nullable|cumulative_busy_ticks:FixedSizeBinary(16):nullable|attempt_revision:UInt64:nullable|execution_revision:UInt64:nullable|reason:Utf8:nullable";

static SCHEMA: OnceLock<SchemaRef> = OnceLock::new();

/// The frozen v1 Arrow schema, with empty schema and field metadata.
pub fn schema() -> SchemaRef {
    SCHEMA
        .get_or_init(|| {
            let fields = vec![
                Field::new("schema_version", DataType::UInt16, false),
                Field::new("run_id", DataType::Utf8, false),
                Field::new("causal_event_id", DataType::FixedSizeBinary(12), false),
                Field::new("transition_ordinal", DataType::UInt32, false),
                Field::new("time_ticks", DataType::FixedSizeBinary(16), false),
                Field::new("time_scale", DataType::Utf8, false),
                Field::new("resource_id", DataType::FixedSizeBinary(12), false),
                Field::new("request_id", DataType::FixedSizeBinary(12), false),
                Field::new("owner_id", DataType::FixedSizeBinary(12), false),
                Field::new("work_id", DataType::FixedSizeBinary(12), true),
                Field::new("lease_revision", DataType::UInt64, true),
                Field::new("causal_lease_revision", DataType::UInt64, true),
                Field::new("transition", DataType::Utf8, false),
                Field::new("request_state", DataType::Utf8, false),
                Field::new("strategy", DataType::Utf8, true),
                Field::new("preemptor_request_id", DataType::FixedSizeBinary(12), true),
                Field::new("priority", DataType::Int32, false),
                Field::new("queue_len", DataType::UInt32, false),
                Field::new("active_count", DataType::UInt32, false),
                Field::new("capacity", DataType::UInt32, false),
                Field::new(
                    "original_duration_ticks",
                    DataType::FixedSizeBinary(16),
                    true,
                ),
                Field::new("useful_elapsed_ticks", DataType::FixedSizeBinary(16), true),
                Field::new("remaining_ticks", DataType::FixedSizeBinary(16), true),
                Field::new("cumulative_busy_ticks", DataType::FixedSizeBinary(16), true),
                Field::new("attempt_revision", DataType::UInt64, true),
                Field::new("execution_revision", DataType::UInt64, true),
                Field::new("reason", DataType::Utf8, true),
            ];
            Arc::new(Schema::new(fields))
        })
        .clone()
}

/// Stable textual fingerprint for the exact ordered v1 field contract.
pub fn schema_fingerprint() -> String {
    FINGERPRINT.to_owned()
}

/// Errors found in the supplied run label or immutable record batch.
#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("run_id must not be whitespace-only")]
    EmptyRunId,
    #[error("causal event {event_index}:{event_generation} reappears after its block")]
    EventReappears {
        event_index: u64,
        event_generation: u32,
    },
    #[error(
        "causal event {event_index}:{event_generation} has ordinal {actual}; expected {expected}"
    )]
    InvalidOrdinal {
        event_index: u64,
        event_generation: u32,
        expected: u64,
        actual: u32,
    },
    #[error("{lease_kind} lease request does not match lifecycle record request")]
    LeaseRequestMismatch { lease_kind: &'static str },
    #[error("timed progress is present without a work ID")]
    ProgressWithoutWork,
    #[error("Arrow record batch construction failed: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
}

/// Encode lifecycle records in input order without consulting runtime state.
pub fn encode(run_id: &str, records: &[LifecycleRecord]) -> Result<RecordBatch, LifecycleError> {
    if run_id.trim().is_empty() {
        return Err(LifecycleError::EmptyRunId);
    }
    validate_records(records)?;

    let capacity = records.len();
    let mut schema_version = UInt16Builder::with_capacity(capacity);
    let mut run_ids = StringBuilder::new();
    let mut causal_event_ids = FixedSizeBinaryBuilder::new(12);
    let mut ordinals = UInt32Builder::with_capacity(capacity);
    let mut times = FixedSizeBinaryBuilder::new(16);
    let mut scales = StringBuilder::new();
    let mut resource_ids = FixedSizeBinaryBuilder::new(12);
    let mut request_ids = FixedSizeBinaryBuilder::new(12);
    let mut owner_ids = FixedSizeBinaryBuilder::new(12);
    let mut work_ids = FixedSizeBinaryBuilder::new(12);
    let mut lease_revisions = UInt64Builder::with_capacity(capacity);
    let mut causal_lease_revisions = UInt64Builder::with_capacity(capacity);
    let mut transitions = StringBuilder::new();
    let mut request_states = StringBuilder::new();
    let mut strategies = StringBuilder::new();
    let mut preemptors = FixedSizeBinaryBuilder::new(12);
    let mut priorities = Int32Builder::with_capacity(capacity);
    let mut queue_lengths = UInt32Builder::with_capacity(capacity);
    let mut active_counts = UInt32Builder::with_capacity(capacity);
    let mut capacities = UInt32Builder::with_capacity(capacity);
    let mut original_durations = FixedSizeBinaryBuilder::new(16);
    let mut useful_elapsed = FixedSizeBinaryBuilder::new(16);
    let mut remaining = FixedSizeBinaryBuilder::new(16);
    let mut cumulative_busy = FixedSizeBinaryBuilder::new(16);
    let mut attempts = UInt64Builder::with_capacity(capacity);
    let mut executions = UInt64Builder::with_capacity(capacity);
    let mut reasons = StringBuilder::new();

    for record in records {
        schema_version.append_value(SCHEMA_VERSION);
        run_ids.append_value(run_id);
        append_entity_bytes(
            &mut causal_event_ids,
            record.causal_event_id.index,
            record.causal_event_id.generation,
        )?;
        ordinals.append_value(record.transition_ordinal);
        times.append_value(record.at.ticks().to_le_bytes())?;
        scales.append_value(TIME_SCALE);
        append_entity(&mut resource_ids, record.resource.entity_id())?;
        append_entity(&mut request_ids, record.request.entity_id())?;
        append_entity(&mut owner_ids, record.snapshot.owner)?;
        if let Some(work) = record.snapshot.work {
            append_entity(&mut work_ids, work.entity_id())?;
        } else {
            work_ids.append_null();
        }
        append_optional_revision(&mut lease_revisions, record.lease.map(LeaseId::revision));
        append_optional_revision(
            &mut causal_lease_revisions,
            record.snapshot.causal_lease.map(LeaseId::revision),
        );
        transitions.append_value(transition_token(record.transition));
        request_states.append_value(state_token(record.state));
        if let Some(strategy) = record.snapshot.strategy {
            strategies.append_value(strategy_token(strategy));
        } else {
            strategies.append_null();
        }
        if let Some(preemptor) = record.snapshot.preemptor_request {
            append_entity(&mut preemptors, preemptor.entity_id())?;
        } else {
            preemptors.append_null();
        }
        priorities.append_value(record.snapshot.priority_level);
        queue_lengths.append_value(record.snapshot.queue_len);
        active_counts.append_value(record.snapshot.active_count);
        capacities.append_value(record.snapshot.capacity);
        if let Some(progress) = &record.snapshot.progress {
            original_durations.append_value(progress.original_duration.ticks().to_le_bytes())?;
            useful_elapsed.append_value(progress.useful_elapsed.ticks().to_le_bytes())?;
            remaining.append_value(progress.remaining.ticks().to_le_bytes())?;
            cumulative_busy.append_value(progress.cumulative_busy.ticks().to_le_bytes())?;
            attempts.append_value(progress.attempt_revision);
            executions.append_value(progress.execution_revision);
        } else {
            original_durations.append_null();
            useful_elapsed.append_null();
            remaining.append_null();
            cumulative_busy.append_null();
            attempts.append_null();
            executions.append_null();
        }
        reasons.append_null();
    }

    let columns: Vec<ArrayRef> = vec![
        Arc::new(schema_version.finish()),
        Arc::new(run_ids.finish()),
        Arc::new(causal_event_ids.finish()),
        Arc::new(ordinals.finish()),
        Arc::new(times.finish()),
        Arc::new(scales.finish()),
        Arc::new(resource_ids.finish()),
        Arc::new(request_ids.finish()),
        Arc::new(owner_ids.finish()),
        Arc::new(work_ids.finish()),
        Arc::new(lease_revisions.finish()),
        Arc::new(causal_lease_revisions.finish()),
        Arc::new(transitions.finish()),
        Arc::new(request_states.finish()),
        Arc::new(strategies.finish()),
        Arc::new(preemptors.finish()),
        Arc::new(priorities.finish()),
        Arc::new(queue_lengths.finish()),
        Arc::new(active_counts.finish()),
        Arc::new(capacities.finish()),
        Arc::new(original_durations.finish()),
        Arc::new(useful_elapsed.finish()),
        Arc::new(remaining.finish()),
        Arc::new(cumulative_busy.finish()),
        Arc::new(attempts.finish()),
        Arc::new(executions.finish()),
        Arc::new(reasons.finish()),
    ];
    Ok(RecordBatch::try_new(schema(), columns)?)
}

fn validate_records(records: &[LifecycleRecord]) -> Result<(), LifecycleError> {
    let mut seen_events = HashSet::new();
    let mut previous = None;
    let mut expected_ordinal = 0u64;
    for record in records {
        let event = record.causal_event_id;
        if previous != Some(event) {
            if previous.is_some_and(|prior| !seen_events.insert(prior)) {
                unreachable!("previous event is inserted exactly when its block closes");
            }
            if seen_events.contains(&event) {
                return Err(LifecycleError::EventReappears {
                    event_index: event.index,
                    event_generation: event.generation,
                });
            }
            if record.transition_ordinal != 0 {
                return Err(LifecycleError::InvalidOrdinal {
                    event_index: event.index,
                    event_generation: event.generation,
                    expected: 0,
                    actual: record.transition_ordinal,
                });
            }
            expected_ordinal = 1;
            previous = Some(event);
        } else {
            expected_ordinal = next_expected_ordinal(expected_ordinal, record.transition_ordinal)
                .ok_or(LifecycleError::InvalidOrdinal {
                event_index: event.index,
                event_generation: event.generation,
                expected: expected_ordinal,
                actual: record.transition_ordinal,
            })?;
        }

        if record
            .lease
            .is_some_and(|lease| lease.request_id() != record.request)
        {
            return Err(LifecycleError::LeaseRequestMismatch {
                lease_kind: "current",
            });
        }
        if record
            .snapshot
            .causal_lease
            .is_some_and(|lease| lease.request_id() != record.request)
        {
            return Err(LifecycleError::LeaseRequestMismatch {
                lease_kind: "causal",
            });
        }
        if record.snapshot.progress.is_some() && record.snapshot.work.is_none() {
            return Err(LifecycleError::ProgressWithoutWork);
        }
    }
    Ok(())
}

fn next_expected_ordinal(expected: u64, actual: u32) -> Option<u64> {
    (u64::from(actual) == expected).then(|| expected + 1)
}

fn entity_bytes(index: u64, generation: u32) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[..8].copy_from_slice(&index.to_le_bytes());
    bytes[8..].copy_from_slice(&generation.to_le_bytes());
    bytes
}

fn append_entity(
    builder: &mut FixedSizeBinaryBuilder,
    entity: EntityId,
) -> Result<(), arrow_schema::ArrowError> {
    builder.append_value(entity_bytes(entity.index, entity.generation))
}

fn append_entity_bytes(
    builder: &mut FixedSizeBinaryBuilder,
    index: u64,
    generation: u32,
) -> Result<(), arrow_schema::ArrowError> {
    builder.append_value(entity_bytes(index, generation))
}

fn append_optional_revision(builder: &mut UInt64Builder, revision: Option<u64>) {
    if let Some(revision) = revision {
        builder.append_value(revision);
    } else {
        builder.append_null();
    }
}

fn transition_token(transition: LifecycleTransition) -> &'static str {
    match transition {
        LifecycleTransition::Queued => "queued",
        LifecycleTransition::Granted => "granted",
        LifecycleTransition::Released => "released",
        LifecycleTransition::Cancelled => "cancelled",
        LifecycleTransition::TimedOut => "timed_out",
        LifecycleTransition::Preempted => "preempted",
        LifecycleTransition::Resumed => "resumed",
        LifecycleTransition::Restarted => "restarted",
        LifecycleTransition::Completed => "completed",
        LifecycleTransition::Aborted => "aborted",
    }
}

fn state_token(state: RequestState) -> &'static str {
    match state {
        RequestState::Pending => "pending",
        RequestState::Queued => "queued",
        RequestState::Active => "active",
        RequestState::Released => "released",
        RequestState::Cancelled => "cancelled",
        RequestState::TimedOut => "timed_out",
        RequestState::Suspended => "suspended",
        RequestState::Completed => "completed",
        RequestState::Aborted => "aborted",
    }
}

fn strategy_token(strategy: PreemptionStrategy) -> &'static str {
    match strategy {
        PreemptionStrategy::Suspend => "suspend",
        PreemptionStrategy::Abort => "abort",
        PreemptionStrategy::Restart => "restart",
    }
}

#[cfg(test)]
mod tests {
    use super::next_expected_ordinal;

    #[test]
    fn ordinal_validation_does_not_saturate_at_u32_max() {
        let max = u32::MAX;
        assert_eq!(
            next_expected_ordinal(u64::from(max), max),
            Some(u64::from(max) + 1)
        );
        assert_eq!(next_expected_ordinal(u64::from(max) + 1, max), None);
    }
}
