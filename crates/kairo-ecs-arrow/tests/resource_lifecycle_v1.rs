#![cfg(feature = "resource-lifecycle-io")]

use arrow_array::{Array, FixedSizeBinaryArray, StringArray, UInt64Array};
use arrow_schema::DataType;
use kairo_ecs_arrow::{
    event_log_schema_fingerprint,
    resource_lifecycle::{encode, schema, schema_fingerprint, RESOURCE_LIFECYCLE_STREAM},
};
use kairo_ecs_arrow_io::{
    read_ipc_file, read_ipc_stream, write_ipc_file, write_ipc_stream, IoLimits,
};
use kairo_ecs_des::{
    FlowRuntime, LifecycleRecord, LifecycleTransition as T, PreemptionStrategy, RequestState,
};
use kairo_ecs_types::{EntityId, EventId, SimDuration, SimTime};

const LIFECYCLE_FINGERPRINT: &str = "kairo_ecs.resource_lifecycle.v1;major=1;fields=schema_version:UInt16:required|run_id:Utf8:required|causal_event_id:FixedSizeBinary(12):required|transition_ordinal:UInt32:required|time_ticks:FixedSizeBinary(16):required|time_scale:Utf8:required|resource_id:FixedSizeBinary(12):required|request_id:FixedSizeBinary(12):required|owner_id:FixedSizeBinary(12):required|work_id:FixedSizeBinary(12):nullable|lease_revision:UInt64:nullable|causal_lease_revision:UInt64:nullable|transition:Utf8:required|request_state:Utf8:required|strategy:Utf8:nullable|preemptor_request_id:FixedSizeBinary(12):nullable|priority:Int32:required|queue_len:UInt32:required|active_count:UInt32:required|capacity:UInt32:required|original_duration_ticks:FixedSizeBinary(16):nullable|useful_elapsed_ticks:FixedSizeBinary(16):nullable|remaining_ticks:FixedSizeBinary(16):nullable|cumulative_busy_ticks:FixedSizeBinary(16):nullable|attempt_revision:UInt64:nullable|execution_revision:UInt64:nullable|reason:Utf8:nullable";
const EVENT_LOG_FINGERPRINT: &str = "kairo_ecs.event_log.v1;major=1;fields=schema_version:UInt16:required|run_id:Utf8:required|event_id:FixedSizeBinary(12):required|entity_id:FixedSizeBinary(12):nullable|time_ticks:FixedSizeBinary(16):required|time_scale:Utf8:required|priority:Int32:required|sequence:UInt64:required|event_kind:Utf8:required|status:Utf8:required|payload_ref:Utf8:nullable";

fn t(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn d(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn drain(flow: &mut FlowRuntime) -> Vec<LifecycleRecord> {
    let mut rows = Vec::new();
    for _ in 0..128 {
        match flow.step().unwrap() {
            Some(dispatch) => {
                assert!(dispatch.error.is_none(), "{:?}", dispatch.error);
                rows.extend(dispatch.records);
            }
            None => return rows,
        }
    }
    panic!("dispatch bound exceeded");
}

fn zero_work_records() -> Vec<LifecycleRecord> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let work = flow
        .create_work(owner, d(0), "arrow.lifecycle.zero", ())
        .unwrap();
    flow.acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    drain(&mut flow)
}

fn manual_rows() -> Vec<LifecycleRecord> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    flow.submit(resource, owner, t(0)).unwrap();
    drain(&mut flow)
}

fn two_active_rows() -> Vec<LifecycleRecord> {
    let mut flow = FlowRuntime::new();
    let first_owner = flow.spawn_actor().unwrap();
    let second_owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(2).unwrap();
    flow.submit(resource, first_owner, t(0)).unwrap();
    flow.submit(resource, second_owner, t(0)).unwrap();
    drain(&mut flow)
}

fn entity_bytes(id: EntityId) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[..8].copy_from_slice(&id.index.to_le_bytes());
    bytes[8..].copy_from_slice(&id.generation.to_le_bytes());
    bytes
}

fn u128_bytes(value: u128) -> [u8; 16] {
    value.to_le_bytes()
}

fn strings(batch: &arrow_array::RecordBatch, column: usize) -> &StringArray {
    batch
        .column(column)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap()
}

fn fixed(batch: &arrow_array::RecordBatch, column: usize) -> &FixedSizeBinaryArray {
    batch
        .column(column)
        .as_any()
        .downcast_ref::<FixedSizeBinaryArray>()
        .unwrap()
}

fn limits() -> IoLimits {
    IoLimits {
        max_input_bytes: 1 << 20,
        max_output_bytes: 1 << 20,
        max_batch_rows: 64,
        max_total_rows: 64,
        max_batches: 2,
        max_columns: 27,
    }
}

#[test]
fn frozen_schema_order_types_nullability_and_metadata_match_contract() {
    let schema = schema();
    let expected = vec![
        ("schema_version", DataType::UInt16, false),
        ("run_id", DataType::Utf8, false),
        ("causal_event_id", DataType::FixedSizeBinary(12), false),
        ("transition_ordinal", DataType::UInt32, false),
        ("time_ticks", DataType::FixedSizeBinary(16), false),
        ("time_scale", DataType::Utf8, false),
        ("resource_id", DataType::FixedSizeBinary(12), false),
        ("request_id", DataType::FixedSizeBinary(12), false),
        ("owner_id", DataType::FixedSizeBinary(12), false),
        ("work_id", DataType::FixedSizeBinary(12), true),
        ("lease_revision", DataType::UInt64, true),
        ("causal_lease_revision", DataType::UInt64, true),
        ("transition", DataType::Utf8, false),
        ("request_state", DataType::Utf8, false),
        ("strategy", DataType::Utf8, true),
        ("preemptor_request_id", DataType::FixedSizeBinary(12), true),
        ("priority", DataType::Int32, false),
        ("queue_len", DataType::UInt32, false),
        ("active_count", DataType::UInt32, false),
        ("capacity", DataType::UInt32, false),
        (
            "original_duration_ticks",
            DataType::FixedSizeBinary(16),
            true,
        ),
        ("useful_elapsed_ticks", DataType::FixedSizeBinary(16), true),
        ("remaining_ticks", DataType::FixedSizeBinary(16), true),
        ("cumulative_busy_ticks", DataType::FixedSizeBinary(16), true),
        ("attempt_revision", DataType::UInt64, true),
        ("execution_revision", DataType::UInt64, true),
        ("reason", DataType::Utf8, true),
    ];
    assert_eq!(RESOURCE_LIFECYCLE_STREAM, "kairo_ecs.resource_lifecycle.v1");
    assert_eq!(schema.fields().len(), 27);
    assert_eq!(schema_fingerprint(), LIFECYCLE_FINGERPRINT);
    assert!(schema.metadata().is_empty());
    for (actual, (name, data_type, nullable)) in schema.fields().iter().zip(expected) {
        assert_eq!(actual.name(), name);
        assert_eq!(actual.data_type(), &data_type);
        assert_eq!(actual.is_nullable(), nullable);
        assert!(
            actual.metadata().is_empty(),
            "field metadata: {}",
            actual.name()
        );
    }

    let frozen_json = include_str!("../../../schemas/arrow/resource_lifecycle_v1.schema.json");
    let mut previous = 0;
    for (name, data_type, nullable) in expected_schema_markers() {
        let position = frozen_json[previous..].find(name).unwrap() + previous;
        let field = frozen_json[position..].split('}').next().unwrap();
        assert!(field.contains(&format!("\"type\": \"{data_type}\"")));
        assert!(field.contains(&format!("\"nullable\": {nullable}")));
        previous = position + name.len();
    }
    assert_eq!(
        event_log_schema_fingerprint(),
        EVENT_LOG_FINGERPRINT,
        "resource lifecycle must not change event_log.v1"
    );
}

fn expected_schema_markers() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        ("\"name\": \"schema_version\"", "UInt16", false),
        ("\"name\": \"run_id\"", "Utf8", false),
        (
            "\"name\": \"causal_event_id\"",
            "FixedSizeBinary(12)",
            false,
        ),
        ("\"name\": \"transition_ordinal\"", "UInt32", false),
        ("\"name\": \"time_ticks\"", "FixedSizeBinary(16)", false),
        ("\"name\": \"time_scale\"", "Utf8", false),
        ("\"name\": \"resource_id\"", "FixedSizeBinary(12)", false),
        ("\"name\": \"request_id\"", "FixedSizeBinary(12)", false),
        ("\"name\": \"owner_id\"", "FixedSizeBinary(12)", false),
        ("\"name\": \"work_id\"", "FixedSizeBinary(12)", true),
        ("\"name\": \"lease_revision\"", "UInt64", true),
        ("\"name\": \"causal_lease_revision\"", "UInt64", true),
        ("\"name\": \"transition\"", "Utf8", false),
        ("\"name\": \"request_state\"", "Utf8", false),
        ("\"name\": \"strategy\"", "Utf8", true),
        (
            "\"name\": \"preemptor_request_id\"",
            "FixedSizeBinary(12)",
            true,
        ),
        ("\"name\": \"priority\"", "Int32", false),
        ("\"name\": \"queue_len\"", "UInt32", false),
        ("\"name\": \"active_count\"", "UInt32", false),
        ("\"name\": \"capacity\"", "UInt32", false),
        (
            "\"name\": \"original_duration_ticks\"",
            "FixedSizeBinary(16)",
            true,
        ),
        (
            "\"name\": \"useful_elapsed_ticks\"",
            "FixedSizeBinary(16)",
            true,
        ),
        ("\"name\": \"remaining_ticks\"", "FixedSizeBinary(16)", true),
        (
            "\"name\": \"cumulative_busy_ticks\"",
            "FixedSizeBinary(16)",
            true,
        ),
        ("\"name\": \"attempt_revision\"", "UInt64", true),
        ("\"name\": \"execution_revision\"", "UInt64", true),
        ("\"name\": \"reason\"", "Utf8", true),
    ]
}

#[test]
fn empty_batch_and_manual_work_nulls_are_typed_and_distinct_from_zero() {
    let empty = encode("run", &[]).unwrap();
    assert_eq!(empty.num_rows(), 0);
    assert_eq!(empty.schema().as_ref(), schema().as_ref());
    assert_eq!(empty.num_columns(), 27);
    for (column, field) in empty.columns().iter().zip(schema().fields()) {
        assert_eq!(column.data_type(), field.data_type());
        assert_eq!(column.null_count(), 0);
    }

    let manual = manual_rows();
    let queued = manual
        .iter()
        .find(|row| row.transition == T::Queued)
        .unwrap();
    let batch = encode("  manual run  ", std::slice::from_ref(queued)).unwrap();
    assert_eq!(strings(&batch, 1).value(0), "  manual run  ");
    assert!(fixed(&batch, 9).is_null(0));
    assert!(batch.column(10).is_null(0));
    assert!(batch.column(11).is_null(0));
    for column in 20..=25 {
        assert!(batch.column(column).is_null(0), "column {column}");
    }
    assert!(batch.column(14).is_null(0));
    assert!(batch.column(15).is_null(0));
    assert!(batch.column(26).is_null(0));

    let zero_rows = zero_work_records();
    let completed = zero_rows
        .iter()
        .find(|row| row.transition == T::Completed)
        .unwrap();
    let mut completed = completed.clone();
    completed.transition_ordinal = 0;
    let zero = encode("zero", &[completed]).unwrap();
    assert_eq!(fixed(&zero, 20).value(0), u128_bytes(0));
    assert_eq!(fixed(&zero, 21).value(0), u128_bytes(0));
    assert_eq!(fixed(&zero, 22).value(0), u128_bytes(0));
    assert_eq!(fixed(&zero, 23).value(0), u128_bytes(0));
    assert_eq!(
        zero.column(24)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        0
    );
    for column in 20..=25 {
        assert!(
            !zero.column(column).is_null(0),
            "zero-valued column {column}"
        );
    }

    let mut work_without_timed_progress = zero_rows[0].clone();
    work_without_timed_progress.snapshot.progress = None;
    let no_progress = encode("work-no-progress", &[work_without_timed_progress]).unwrap();
    assert!(!fixed(&no_progress, 9).is_null(0));
    for column in 20..=25 {
        assert!(no_progress.column(column).is_null(0), "column {column}");
    }
}

#[test]
fn every_column_maps_the_expected_source_value() {
    let records = zero_work_records();
    let mut row = records
        .iter()
        .find(|r| r.transition == T::Granted)
        .unwrap()
        .clone();
    row.transition_ordinal = 0;
    let batch = encode("mapping-run", std::slice::from_ref(&row)).unwrap();
    assert_eq!(batch.num_columns(), 27);
    assert_eq!(
        batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow_array::UInt16Array>()
            .unwrap()
            .value(0),
        1
    );
    assert_eq!(strings(&batch, 1).value(0), "mapping-run");
    assert_eq!(fixed(&batch, 2).value(0), {
        let mut bytes = [0; 12];
        bytes[..8].copy_from_slice(&row.causal_event_id.index.to_le_bytes());
        bytes[8..].copy_from_slice(&row.causal_event_id.generation.to_le_bytes());
        bytes
    });
    assert_eq!(
        batch
            .column(3)
            .as_any()
            .downcast_ref::<arrow_array::UInt32Array>()
            .unwrap()
            .value(0),
        row.transition_ordinal
    );
    assert_eq!(fixed(&batch, 4).value(0), u128_bytes(row.at.ticks()));
    assert_eq!(strings(&batch, 5).value(0), "ticks");
    assert_eq!(
        fixed(&batch, 6).value(0),
        entity_bytes(row.resource.entity_id())
    );
    assert_eq!(
        fixed(&batch, 7).value(0),
        entity_bytes(row.request.entity_id())
    );
    assert_eq!(fixed(&batch, 8).value(0), entity_bytes(row.snapshot.owner));
    assert_eq!(
        fixed(&batch, 9).value(0),
        entity_bytes(row.snapshot.work.unwrap().entity_id())
    );
    assert_eq!(
        batch
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        row.lease.unwrap().revision()
    );
    assert_eq!(
        batch
            .column(11)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        row.snapshot.causal_lease.unwrap().revision()
    );
    assert_eq!(strings(&batch, 12).value(0), "granted");
    assert_eq!(strings(&batch, 13).value(0), "active");
    assert!(batch.column(14).is_null(0));
    assert!(fixed(&batch, 15).is_null(0));
    assert_eq!(
        batch
            .column(16)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap()
            .value(0),
        row.snapshot.priority_level
    );
    assert_eq!(
        batch
            .column(17)
            .as_any()
            .downcast_ref::<arrow_array::UInt32Array>()
            .unwrap()
            .value(0),
        row.snapshot.queue_len
    );
    assert_eq!(
        batch
            .column(18)
            .as_any()
            .downcast_ref::<arrow_array::UInt32Array>()
            .unwrap()
            .value(0),
        row.snapshot.active_count
    );
    assert_eq!(
        batch
            .column(19)
            .as_any()
            .downcast_ref::<arrow_array::UInt32Array>()
            .unwrap()
            .value(0),
        row.snapshot.capacity
    );
    let progress = row.snapshot.progress.as_ref().unwrap();
    assert_eq!(
        fixed(&batch, 20).value(0),
        u128_bytes(progress.original_duration.ticks())
    );
    assert_eq!(
        fixed(&batch, 21).value(0),
        u128_bytes(progress.useful_elapsed.ticks())
    );
    assert_eq!(
        fixed(&batch, 22).value(0),
        u128_bytes(progress.remaining.ticks())
    );
    assert_eq!(
        fixed(&batch, 23).value(0),
        u128_bytes(progress.cumulative_busy.ticks())
    );
    assert_eq!(
        batch
            .column(24)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        progress.attempt_revision
    );
    assert_eq!(
        batch
            .column(25)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        progress.execution_revision
    );
    assert!(batch.column(26).is_null(0));

    let mut completed = records
        .iter()
        .find(|r| r.transition == T::Completed)
        .unwrap()
        .clone();
    completed.transition_ordinal = 0;
    let completed_batch = encode("completed-control", std::slice::from_ref(&completed)).unwrap();
    assert!(completed_batch.column(10).is_null(0));
    assert_eq!(
        completed_batch
            .column(11)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        completed.snapshot.causal_lease.unwrap().revision()
    );

    let manual = manual_rows();
    let manual_queued = manual.iter().find(|r| r.transition == T::Queued).unwrap();
    let manual_batch = encode("manual-null-control", std::slice::from_ref(manual_queued)).unwrap();
    assert!(manual_batch.column(10).is_null(0));
    assert!(manual_batch.column(11).is_null(0));
    let queued = records.iter().find(|r| r.transition == T::Queued).unwrap();
    let queued_batch = encode("queued-null-control", std::slice::from_ref(queued)).unwrap();
    assert!(queued_batch.column(10).is_null(0));
    assert!(queued_batch.column(11).is_null(0));
}

#[test]
fn engine_records_cover_every_enum_token_and_full_width_values() {
    let base = zero_work_records();
    let mut row = base
        .iter()
        .find(|r| r.transition == T::Completed)
        .unwrap()
        .clone();
    row.causal_event_id = EventId::new(u64::MAX, u32::MAX);
    row.transition_ordinal = 0;
    row.at = t(u128::MAX);
    row.snapshot.owner = EntityId::new(u64::MAX, u32::MAX);
    let progress = row.snapshot.progress.as_mut().unwrap();
    progress.original_duration = d(u128::MAX);
    progress.useful_elapsed = d(u128::MAX - 1);
    progress.remaining = d(u128::MAX);
    progress.cumulative_busy = d(u128::MAX - 2);
    progress.attempt_revision = u64::MAX;
    progress.execution_revision = u64::MAX;
    let batch = encode("  full-width  ", std::slice::from_ref(&row)).unwrap();
    assert_eq!(
        fixed(&batch, 2).value(0),
        entity_bytes(EntityId::new(u64::MAX, u32::MAX))
    );
    assert_eq!(fixed(&batch, 4).value(0), u128_bytes(u128::MAX));
    assert_eq!(
        fixed(&batch, 8).value(0),
        entity_bytes(EntityId::new(u64::MAX, u32::MAX))
    );
    assert_eq!(fixed(&batch, 6).value(0).len(), 12);
    assert_eq!(fixed(&batch, 7).value(0).len(), 12);
    assert_eq!(fixed(&batch, 9).value(0).len(), 12);
    assert_eq!(fixed(&batch, 20).value(0), u128_bytes(u128::MAX));
    assert_eq!(fixed(&batch, 21).value(0), u128_bytes(u128::MAX - 1));
    assert_eq!(fixed(&batch, 22).value(0), u128_bytes(u128::MAX));
    assert_eq!(fixed(&batch, 23).value(0), u128_bytes(u128::MAX - 2));
    assert_eq!(
        batch
            .column(24)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        u64::MAX
    );
    assert_eq!(
        batch
            .column(25)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        u64::MAX
    );
    assert_eq!(strings(&batch, 1).value(0), "  full-width  ");

    let transitions = [
        (T::Queued, RequestState::Queued),
        (T::Granted, RequestState::Active),
        (T::Released, RequestState::Released),
        (T::Cancelled, RequestState::Cancelled),
        (T::TimedOut, RequestState::TimedOut),
        (T::Preempted, RequestState::Suspended),
        (T::Resumed, RequestState::Active),
        (T::Restarted, RequestState::Active),
        (T::Completed, RequestState::Completed),
        (T::Aborted, RequestState::Aborted),
    ];
    let expected_transitions = [
        "queued",
        "granted",
        "released",
        "cancelled",
        "timed_out",
        "preempted",
        "resumed",
        "restarted",
        "completed",
        "aborted",
    ];
    let expected_states = [
        "queued",
        "active",
        "released",
        "cancelled",
        "timed_out",
        "suspended",
        "active",
        "active",
        "completed",
        "aborted",
        "pending",
    ];
    let expected_strategies = [Some("suspend"), Some("abort"), Some("restart")];
    let mut variants = Vec::new();
    for (i, (transition, state)) in transitions.into_iter().enumerate() {
        let mut clone = row.clone();
        clone.causal_event_id = EventId::new(i as u64 + 100, 3);
        clone.transition_ordinal = 0;
        clone.transition = transition;
        clone.state = state;
        clone.snapshot.strategy = match i % 4 {
            0 => Some(PreemptionStrategy::Suspend),
            1 => Some(PreemptionStrategy::Abort),
            2 => Some(PreemptionStrategy::Restart),
            _ => None,
        };
        clone.snapshot.preemptor_request =
            matches!(transition, T::Preempted | T::Aborted).then_some(clone.request);
        variants.push(clone);
    }
    let mut pending = row.clone();
    pending.causal_event_id = EventId::new(110, 3);
    pending.transition_ordinal = 0;
    pending.state = RequestState::Pending;
    variants.push(pending);
    let batch = encode("enum-values", &variants).unwrap();
    let transition_values = strings(&batch, 12);
    let state_values = strings(&batch, 13);
    let strategy_values = strings(&batch, 14);
    for (i, expected) in expected_transitions.iter().enumerate() {
        assert_eq!(transition_values.value(i), *expected);
    }
    for (i, expected) in expected_states.iter().enumerate() {
        assert_eq!(state_values.value(i), *expected);
    }
    for (i, variant) in variants.iter().enumerate().take(10) {
        if i == 5 || i == 9 {
            assert_eq!(
                fixed(&batch, 15).value(i),
                entity_bytes(variant.request.entity_id())
            );
        } else {
            assert!(
                fixed(&batch, 15).is_null(i),
                "unexpected preemptor at row {i}"
            );
        }
    }
    for i in 0..10 {
        match i % 4 {
            0 => assert_eq!(strategy_values.value(i), expected_strategies[0].unwrap()),
            1 => assert_eq!(strategy_values.value(i), expected_strategies[1].unwrap()),
            2 => assert_eq!(strategy_values.value(i), expected_strategies[2].unwrap()),
            _ => assert!(strategy_values.is_null(i), "None strategy must remain null"),
        }
    }
    assert!(batch.column(26).is_null(0), "v1 reason is always null");
}

#[test]
fn rejects_malformed_event_order_run_ids_and_snapshot_references() {
    let rows = zero_work_records();
    assert!(encode(" \t\n ", &rows[..1]).is_err());

    let mut nonzero_first = rows[0].clone();
    nonzero_first.transition_ordinal = 1;
    assert!(encode("bad", &[nonzero_first]).is_err());

    assert!(encode("duplicate", &[rows[0].clone(), rows[0].clone()]).is_err());

    let mut gap = rows.clone();
    gap[2].transition_ordinal = 9;
    assert!(encode("gap", &gap).is_err());

    let mut reordered = rows.clone();
    reordered[0].transition_ordinal = 0;
    reordered[1].transition_ordinal = 2;
    reordered[2].transition_ordinal = 1;
    assert!(encode("reordered", &reordered).is_err());

    let mut first_block = rows[0].clone();
    first_block.causal_event_id = EventId::new(50, 0);
    first_block.transition_ordinal = 0;
    let mut second_block = rows[1].clone();
    second_block.causal_event_id = EventId::new(51, 0);
    second_block.transition_ordinal = 0;
    let mut repeated_block = rows[2].clone();
    repeated_block.causal_event_id = EventId::new(50, 0);
    repeated_block.transition_ordinal = 1;
    assert!(encode(
        "event-reappears",
        &[first_block, second_block, repeated_block]
    )
    .is_err());

    let active = two_active_rows();
    let grants: Vec<_> = active
        .iter()
        .filter(|row| row.transition == T::Granted)
        .collect();
    assert_eq!(grants.len(), 2);
    let mut valid_grant = (*grants[0]).clone();
    valid_grant.transition_ordinal = 0;
    assert!(encode("valid-grant", &[valid_grant.clone()]).is_ok());

    let mut bad_current_lease = valid_grant.clone();
    bad_current_lease.lease = grants[1].lease;
    assert!(encode("bad-current-lease", &[bad_current_lease]).is_err());

    let mut causal_baseline = valid_grant.clone();
    causal_baseline.snapshot.causal_lease = Some(valid_grant.lease.unwrap());
    assert!(encode("valid-causal-grant", &[causal_baseline.clone()]).is_ok());
    let mut bad_causal_lease = causal_baseline;
    bad_causal_lease.snapshot.causal_lease = grants[1].snapshot.causal_lease;
    assert!(encode("bad-causal-lease", &[bad_causal_lease]).is_err());

    let mut progress_without_work = rows[1].clone();
    progress_without_work.snapshot.work = None;
    progress_without_work.transition_ordinal = 0;
    assert!(progress_without_work.snapshot.progress.is_some());
    assert!(encode("progress-without-work", &[progress_without_work]).is_err());
}

#[test]
fn bounded_arrowio_stream_and_file_roundtrip_preserve_all_columns_and_metadata() {
    let mut records = zero_work_records();
    for record in &mut records {
        record.causal_event_id = EventId::new(u64::MAX, u32::MAX);
    }
    records[0].at = t(u128::MAX);
    records[0].snapshot.owner = EntityId::new(u64::MAX, u32::MAX);
    let batch = encode("ipc-run", &records).unwrap();
    let expected = batch.schema();
    let limits = limits();
    assert!(expected.metadata().is_empty());
    assert!(expected
        .fields()
        .iter()
        .all(|field| field.metadata().is_empty()));

    let stream = write_ipc_stream(expected.clone(), std::slice::from_ref(&batch), limits).unwrap();
    let stream_batches = read_ipc_stream(&stream, expected.clone(), limits).unwrap();
    let file = write_ipc_file(expected.clone(), std::slice::from_ref(&batch), limits).unwrap();
    let file_batches = read_ipc_file(&file, expected.clone(), limits).unwrap();

    for batches in [stream_batches, file_batches] {
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_columns(), 27);
        assert_eq!(batches[0].num_rows(), batch.num_rows());
        assert_eq!(batches[0].schema().as_ref(), expected.as_ref());
        for column in 0..27 {
            assert_eq!(
                batches[0].column(column).to_data(),
                batch.column(column).to_data()
            );
        }
        assert_eq!(
            fixed(&batches[0], 2).value(0),
            entity_bytes(EntityId::new(u64::MAX, u32::MAX))
        );
        assert_eq!(fixed(&batches[0], 4).value(0), u128_bytes(u128::MAX));
        assert_eq!(
            fixed(&batches[0], 8).value(0),
            entity_bytes(EntityId::new(u64::MAX, u32::MAX))
        );
    }
}
