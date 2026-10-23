//! Track 70 Phase 2 Task 2.2 — FFI and polyglot consumer acceptance tests for
//! the populated owned Arrow C Data arrays/streams export (populated
//! multi-batch / empty / null / lifetime / release).
//!
//! Dual-mode suite:
//!
//! - **Default features** (no `native-data`): the pre-Task-2.2 descriptor-only
//!   capability remains, expressed by `#[should_panic]` probes with explicit
//!   markers (absent capability panics by design, per the Phase 0
//!   `should_panic` convention).
//! - **`--features native-data`**: the populated export implementation from
//!   Task 2.2 replaces those capabilities, so the same file carries real
//!   behavioural acceptance tests.
//!
//! Validation commands (recorded in the track handoff):
//! - `cargo test -p kairo-ecs-ffi --test track70_phase2_ffi_consumers`
//! - `cargo test -p kairo-ecs-ffi --features native-data --test track70_phase2_ffi_consumers`

use kairo_ecs_ffi::{
    kairo_ecs_cancel_event, kairo_ecs_engine_free, kairo_ecs_engine_new,
    kairo_ecs_export_event_log_arrow_array, kairo_ecs_export_event_log_arrow_schema,
    kairo_ecs_schedule_at, kairo_ecs_step, KairoArrowArray, KairoArrowSchema, KairoEcsStatusCode,
};

/// Explicit v2 event-log identifiers required by spec.md ("Replace alpha
/// tabular schemas with explicit v2 identifiers").
const V2_EVENT_LOG_NAME: &str = "kairo_ecs.event_log.v2";
/// Schema v2 event log carries 14 typed columns (see
/// `crates/kairo-ecs-arrow/tests/track70_phase1_native_tables.rs`).
const V2_EVENT_LOG_COLUMNS: i64 = 14;

/// Build an engine with exactly `events` dispatched custom events.
fn dispatched_engine(events: u64) -> u64 {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0, "engine creation must succeed");
    for tick in 1..=events {
        let event = kairo_ecs_schedule_at(handle, tick * 10, 0, 7);
        assert_ne!(event, 0, "scheduling must return a usable event handle");
        assert_eq!(
            kairo_ecs_step(handle),
            KairoEcsStatusCode::KAIRO_ECS_OK,
            "step must dispatch the scheduled event"
        );
    }
    handle
}

/// The shared Arrow C ABI release-once contract holds on every build: the
/// consumer nulls the holder before invoking `release`, release runs exactly
/// once, and freeing the engine afterwards stays sound.
#[test]
fn exported_array_release_invoked_exactly_once_and_engine_free_stays_sound() {
    let mut engine_handle = dispatched_engine(1);

    let mut array = KairoArrowArray::default();
    let status = unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
    assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);

    let release = array
        .release
        .expect("export must set an array release callback");
    // Arrow C ABI: the consumer nulls the holder before invoking release.
    array.release = None;
    unsafe { release(&mut array) };
    assert!(
        array.release.is_none(),
        "release must invalidate the holder exactly once"
    );
    // The spent exported struct must remain a plain value across engine
    // destruction; freeing the engine never dangles the released surface.
    assert_eq!(
        kairo_ecs_engine_free(engine_handle),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );
}

#[cfg(not(feature = "native-data"))]
mod capability_absent {
    use super::*;
    use core::ffi::CStr;

    #[test]
    #[should_panic(expected = "populated Arrow C Data buffers required")]
    fn event_log_array_export_requires_populated_buffers() {
        let mut engine_handle = dispatched_engine(1);
        let mut array = KairoArrowArray::default();
        let status =
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
        assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
        assert_eq!(
            array.length, 1,
            "exported length must reflect actually dispatched events"
        );

        // v2 contract: populated Arrow C Data buffers. The descriptor-only
        // fallback (`buffers == NULL`, `n_children == 0`) is precisely what
        // Task 2.2 must remove.
        if array.buffers.is_null() || array.n_children == 0 {
            panic!("populated Arrow C Data buffers required");
        }
    }

    #[test]
    #[should_panic(expected = "v2 schema identifiers and typed columns required")]
    fn event_log_schema_export_requires_v2_identifiers_and_typed_children() {
        let mut schema = KairoArrowSchema::default();
        let status = unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) };
        assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
        assert!(
            !schema.name.is_null(),
            "schema export must expose the stream name"
        );

        let name = unsafe { CStr::from_ptr(schema.name) }
            .to_string_lossy()
            .into_owned();
        if name != V2_EVENT_LOG_NAME || schema.n_children != V2_EVENT_LOG_COLUMNS {
            panic!("v2 schema identifiers and typed columns required");
        }
    }

    #[test]
    #[should_panic(expected = "multi-batch populated stream export required")]
    fn event_log_export_supports_multiple_populated_batches() {
        let mut engine_handle = dispatched_engine(2);

        // Two successive exports of the same engine can never express batch
        // boundaries: the descriptor-only surface exports one struct per call
        // and no ArrowArrayStream.
        let mut first = KairoArrowArray::default();
        let status =
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut first) };
        assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);

        panic!("multi-batch populated stream export required");
    }

    #[test]
    #[should_panic(expected = "empty export must stay schema-valid for independent consumers")]
    fn empty_event_log_export_requires_schema_valid_array() {
        // Engine with zero dispatched events: independent consumers (pyarrow,
        // Polars) must receive a length-0 array that still carries the complete
        // v2 typed schema, never a fabricated capsule with unknown columns.
        let mut engine_handle = dispatched_engine(0);
        let mut array = KairoArrowArray::default();
        let status =
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
        assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
        assert_eq!(array.length, 0, "empty engine must export length 0");

        panic!("empty export must stay schema-valid for independent consumers");
    }

    #[test]
    #[should_panic(expected = "nullable generational IDs must null together in populated export")]
    fn exported_nullable_generational_ids_null_together() {
        // spec.md: generational IDs use UInt64 index plus UInt32 generation,
        // "both null together for nullable IDs" — an entity-less dispatch leaves
        // both columns null in the same row; an entity dispatch populates both.
        // This row-level co-nullability contract is only testable once populated
        // export exists (Task 2.2); it is also guarded on the Python consumer side
        // by bindings/python/tests/test_track70_phase2_native_handoff.py.
        panic!("nullable generational IDs must null together in populated export");
    }
}

#[cfg(feature = "native-data")]
mod populated_acceptance {
    use super::*;
    use kairo_ecs_arrow::arrow_c_data::arrow_ffi as abi;
    use kairo_ecs_arrow::arrow_c_data::{
        Array, ArrowDataType as PlDataType, ArrowField as PlField, PrimitiveArray, StructArray,
        Utf8Array,
    };
    use kairo_ecs_arrow::{EVENT_LOG_FIELDS_V2, EVENT_LOG_STREAM_V2};

    fn read_u64(child: &dyn Array, name: &str) -> PrimitiveArray<u64> {
        child
            .as_any()
            .downcast_ref::<PrimitiveArray<u64>>()
            .unwrap_or_else(|| panic!("{name} must be a UInt64 array"))
            .clone()
    }

    fn read_u32(child: &dyn Array, name: &str) -> PrimitiveArray<u32> {
        child
            .as_any()
            .downcast_ref::<PrimitiveArray<u32>>()
            .unwrap_or_else(|| panic!("{name} must be a UInt32 array"))
            .clone()
    }

    fn read_i32(child: &dyn Array, name: &str) -> PrimitiveArray<i32> {
        child
            .as_any()
            .downcast_ref::<PrimitiveArray<i32>>()
            .unwrap_or_else(|| panic!("{name} must be an Int32 array"))
            .clone()
    }

    fn read_string(child: &dyn Array, name: &str) -> Utf8Array<i64> {
        child
            .as_any()
            .downcast_ref::<Utf8Array<i64>>()
            .unwrap_or_else(|| panic!("{name} must export as LargeUtf8"))
            .clone()
    }

    fn child_position(name: &str) -> usize {
        EVENT_LOG_FIELDS_V2
            .iter()
            .position(|f| f.name == name)
            .unwrap_or_else(|| panic!("contract must declare {name}"))
    }

    /// Import the populated export through the independent polars-arrow
    /// reader and run `body` on the resulting v2 struct; the import consumes
    /// (releases exactly once) the export's private data.
    fn with_imported_array(
        array: KairoArrowArray,
        dtype: PlDataType,
        body: impl FnOnce(&StructArray),
    ) {
        // SAFETY: `KairoArrowArray` and `abi::ArrowArray` are ABI-compatible
        // mirror structs of the Arrow C data interface (identical repr(C)
        // field order and types); the move transfers the owned release
        // callback and its private data unchanged.
        let mirror = unsafe { core::mem::transmute::<KairoArrowArray, abi::ArrowArray>(array) };
        // SAFETY: the export came from the populated export backed by valid
        // polars-arrow C data interface storage, and `dtype` is the dtype of
        // the independently imported v2 schema field, so the pair is valid
        // per the C data interface contract. The import consumes (releases
        // exactly once) the export's private data.
        let imported = unsafe { abi::import_array_from_c(mirror, dtype) }
            .expect("populated export must import into an independent consumer");
        let imported = imported
            .as_any()
            .downcast_ref::<StructArray>()
            .expect("event log export must import as the v2 struct");
        body(imported);
    }

    /// Independently import the canonical v2 schema through the C ABI.
    fn import_event_log_field(schema: &KairoArrowSchema) -> PlField {
        // SAFETY: ABI-compatible mirror structs (schema layout is fixed by
        // the Arrow C data interface; see the bridge rationale in
        // kairo-ecs-ffi `bridge_arrow_schema`).
        let mirror =
            unsafe { core::mem::transmute::<&KairoArrowSchema, &abi::ArrowSchema>(schema) };
        // SAFETY: the export was produced by polars-arrow's export_field_to_c
        // from the canonical v2 field, satisfying import_field_from_c's
        // validity requirements.
        unsafe { abi::import_field_from_c(mirror) }
            .expect("populated schema export must import into an independent consumer")
    }

    fn import_event_log_dtype(schema: &KairoArrowSchema) -> PlDataType {
        import_event_log_field(schema).dtype.clone()
    }

    fn assert_field_contract(field: &PlField) {
        assert_eq!(field.name.as_str(), V2_EVENT_LOG_NAME);
        let children = match &field.dtype {
            PlDataType::Struct(children) => children,
            other => panic!("v2 field must be a struct, got {other:?}"),
        };
        assert_eq!(children.len() as i64, V2_EVENT_LOG_COLUMNS);
        for (child, contract) in children.iter().zip(EVENT_LOG_FIELDS_V2) {
            assert_eq!(child.name.as_str(), contract.name);
            assert_eq!(
                child.is_nullable, contract.nullable,
                "nullability of {} must follow the v2 contract",
                contract.name
            );
        }
    }

    #[test]
    fn populated_array_export_carries_populated_v2_children() {
        let mut engine_handle = dispatched_engine(2);
        let mut array = KairoArrowArray::default();
        let status =
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
        assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
        assert_eq!(array.length, 2, "one row per actually dispatched event");
        assert_eq!(
            array.n_children, 14,
            "populated export must carry 14 typed children"
        );
        assert!(
            !array.buffers.is_null(),
            "populated export must carry real buffers"
        );

        let mut schema = KairoArrowSchema::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );
        let field = import_event_log_field(&schema);
        assert_field_contract(&field);

        with_imported_array(array, import_event_log_dtype(&schema), |imported| {
            assert_eq!(imported.len(), 2);
            let values = imported.values();
            assert_eq!(values.len(), EVENT_LOG_FIELDS_V2.len());

            // Event identity survives the C ABI boundary; the scheduler numbers
            // event ids from zero in schedule order.
            let indices = read_u64(
                values[child_position("event_id_index")].as_ref(),
                "event_id_index",
            );
            assert_eq!(indices.value(0), 0);
            assert_eq!(indices.value(1), 1);

            // Scheduled ticks (10, 20) live in the full-range v2 tick limbs.
            let ticks_lo = read_u64(
                values[child_position("time_ticks_lo")].as_ref(),
                "time_ticks_lo",
            );
            assert_eq!((ticks_lo.value(0), ticks_lo.value(1)), (10, 20));
            let ticks_hi = read_u64(
                values[child_position("time_ticks_hi")].as_ref(),
                "time_ticks_hi",
            );
            assert_eq!((ticks_hi.value(0), ticks_hi.value(1)), (0, 0));

            // Priorities, dispatch-order sequences and kinds.
            let priorities = read_i32(values[child_position("priority")].as_ref(), "priority");
            assert_eq!((priorities.value(0), priorities.value(1)), (0, 0));
            let sequences = read_u64(values[child_position("sequence")].as_ref(), "sequence");
            assert_eq!((sequences.value(0), sequences.value(1)), (0, 1));
            let kinds = read_string(values[child_position("event_kind")].as_ref(), "event_kind");
            assert_eq!(kinds.value(0), "custom:7");
            assert_eq!(kinds.value(1), "custom:7");
            let statuses = read_string(values[child_position("status")].as_ref(), "status");
            assert_eq!(statuses.value(0), "dispatched");
            assert_eq!(statuses.value(1), "dispatched");
        });
    }

    #[test]
    fn empty_export_stays_schema_valid_for_independent_consumers() {
        let mut engine_handle = dispatched_engine(0);
        let mut array = KairoArrowArray::default();
        let status =
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
        assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
        assert_eq!(array.length, 0, "empty engine must export length 0");

        let mut schema = KairoArrowSchema::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );
        let field = import_event_log_field(&schema);
        assert_field_contract(&field);

        with_imported_array(array, import_event_log_dtype(&schema), |imported| {
            assert_eq!(imported.len(), 0);
            assert_eq!(
                imported.values().len() as i64,
                V2_EVENT_LOG_COLUMNS,
                "empty export must still carry the complete typed schema"
            );
            assert!(imported.values().iter().all(|child| child.is_empty()));
        });
    }

    #[test]
    fn nullable_generational_ids_null_together_in_populated_export() {
        // The FFI scheduling surface carries no entity, so every row leaves
        // the nullable generational ID columns null together.
        let mut engine_handle = dispatched_engine(3);
        let mut array = KairoArrowArray::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );

        let mut schema = KairoArrowSchema::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );

        with_imported_array(array, import_event_log_dtype(&schema), |imported| {
            let values = imported.values();
            let entity_index = read_u64(
                values[child_position("entity_id_index")].as_ref(),
                "entity_id_index",
            );
            let entity_gen = read_u32(
                values[child_position("entity_id_gen")].as_ref(),
                "entity_id_gen",
            );
            assert_eq!(
                entity_index.null_count(),
                imported.len(),
                "entity index must be null for every entity-less FFI row"
            );
            assert_eq!(
                entity_gen.null_count(),
                imported.len(),
                "entity generation must null together with entity index"
            );
        });
    }

    #[test]
    fn mixed_dispatch_and_cancellation_export_with_monotonic_sequences() {
        let handle = kairo_ecs_engine_new();
        assert_ne!(handle, 0);
        let first = kairo_ecs_schedule_at(handle, 10, 0, 7);
        assert_ne!(first, 0);
        let cancelled_event = kairo_ecs_schedule_at(handle, 20, 0, 9);
        assert_ne!(cancelled_event, 0);

        assert_eq!(
            kairo_ecs_step(handle),
            KairoEcsStatusCode::KAIRO_ECS_OK,
            "step must dispatch the first scheduled event"
        );
        assert_eq!(
            kairo_ecs_cancel_event(handle, cancelled_event),
            KairoEcsStatusCode::KAIRO_ECS_OK,
            "cancellation must succeed for a scheduled event"
        );

        let mut engine_handle = handle;
        let mut array = KairoArrowArray::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );
        assert_eq!(
            array.length, 2,
            "dispatched and cancelled records both export"
        );

        let mut schema = KairoArrowSchema::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );

        with_imported_array(array, import_event_log_dtype(&schema), |imported| {
            let values = imported.values();

            // Provenance order: dispatched records first (dispatch order), then
            // cancelled records (cancellation order), with a monotonic sequence.
            let statuses = read_string(values[child_position("status")].as_ref(), "status");
            assert_eq!(statuses.value(0), "dispatched");
            assert_eq!(statuses.value(1), "cancelled");
            let kinds = read_string(values[child_position("event_kind")].as_ref(), "event_kind");
            assert_eq!(kinds.value(0), "custom:7");
            assert_eq!(kinds.value(1), "custom:9");
            let sequences = read_u64(values[child_position("sequence")].as_ref(), "sequence");
            assert_eq!(sequences.value(0), 0);
            assert_eq!(sequences.value(1), 1);
            let ticks_lo = read_u64(
                values[child_position("time_ticks_lo")].as_ref(),
                "time_ticks_lo",
            );
            assert_eq!((ticks_lo.value(0), ticks_lo.value(1)), (10, 20));
        });
    }

    #[test]
    fn multi_batch_stream_export_windows_bounded_batches() {
        let engine_handle = dispatched_engine(3);

        let stream = kairo_ecs_ffi::export_event_log_arrow_stream(engine_handle, 2)
            .expect("native-data stream export must succeed at the Rust surface");
        // SAFETY: the exported stream was produced by polars-arrow's
        // export_iterator over valid populated struct arrays; try_new
        // validates the invariants it relies on.
        let mut reader = unsafe { abi::ArrowArrayStreamReader::try_new(Box::new(stream)) }
            .expect("stream reader must construct");
        assert_eq!(
            reader.field().name.as_str(),
            EVENT_LOG_STREAM_V2,
            "stream must carry the explicit v2 schema identifier"
        );

        let mut batch_lengths = Vec::new();
        loop {
            // SAFETY: the reader was constructed from a valid C stream
            // produced by the populated export; next assumes C data interface
            // conformant producer output.
            let next = unsafe { reader.next() };
            let Some(result) = next else { break };
            let imported = result.expect("stream batches must import");
            let struct_batch = imported
                .as_any()
                .downcast_ref::<StructArray>()
                .expect("stream batches must be struct arrays");
            assert_eq!(struct_batch.values().len(), EVENT_LOG_FIELDS_V2.len());
            batch_lengths.push(struct_batch.len());
        }
        assert_eq!(
            batch_lengths,
            vec![2, 1],
            "3 dispatched events at max_batch_rows=2 must window into bounded batches"
        );
    }

    #[test]
    fn populated_export_survives_engine_free_until_release_once() {
        let exported = {
            let mut engine_handle = dispatched_engine(1);
            let mut array = KairoArrowArray::default();
            assert_eq!(
                unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) },
                KairoEcsStatusCode::KAIRO_ECS_OK
            );
            assert_eq!(
                kairo_ecs_engine_free(engine_handle),
                KairoEcsStatusCode::KAIRO_ECS_OK
            );
            array
            // Engine destroyed here: the export buffers must stay valid.
        };

        let mut schema = KairoArrowSchema::default();
        assert_eq!(
            unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) },
            KairoEcsStatusCode::KAIRO_ECS_OK
        );

        with_imported_array(exported, import_event_log_dtype(&schema), |imported| {
            assert_eq!(
                imported.len(),
                1,
                "populated buffers outlive engine destruction"
            );
            let ticks_lo = read_u64(
                imported.values()[child_position("time_ticks_lo")].as_ref(),
                "time_ticks_lo",
            );
            assert_eq!(ticks_lo.value(0), 10);
        });
    }
}
