//! Track 70 Phase 2 Task 2.1 — failing characterization tests for FFI and
//! polyglot consumers (populated multi-batch / empty / null / lifetime /
//! release).
//!
//! Baseline observed at commit `2ba9fb9` (`crates/kairo-ecs-ffi/src/lib.rs`):
//! - `kairo_ecs_export_event_log_arrow_schema` returns one bare `+s` struct
//!   named `kairo_ecs.event_log.v1` with `n_children == 0` and no metadata.
//! - `kairo_ecs_export_event_log_arrow_array` returns a descriptor-only
//!   `KairoArrowArray` (`length = dispatched + cancelled`, `n_buffers == 1`,
//!   `buffers == NULL`, `n_children == 0`) without populated Arrow C Data
//!   buffers.
//!
//! The Track 70 Phase 2 contract (spec.md "Data and interface contract")
//! requires owned populated Arrow C Data arrays/streams whose immutable
//! buffers outlive engine mutation/destruction, release called exactly once,
//! explicit empty/null/multi-batch behaviour, explicit v2 schema identifiers,
//! and removal of the descriptor-only/fabricated capsule fallback. Task 2.2
//! owns that implementation and must replace these Phase 0-convention
//! `#[should_panic]` probes (explicit markers documenting absent
//! capabilities) with real behavioural acceptance tests; a probe that stops
//! panicking fails this suite by design, exactly as in
//! `crates/kairo-ecs-arrow/tests/track70_phase0_baseline.rs`.

use core::ffi::CStr;

use kairo_ecs_ffi::{
    kairo_ecs_engine_free, kairo_ecs_engine_new, kairo_ecs_export_event_log_arrow_array,
    kairo_ecs_export_event_log_arrow_schema, kairo_ecs_schedule_at, kairo_ecs_step,
    KairoArrowArray, KairoArrowSchema, KairoEcsStatusCode,
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

#[test]
#[should_panic(expected = "populated Arrow C Data buffers required")]
fn event_log_array_export_requires_populated_buffers() {
    let mut engine_handle = dispatched_engine(1);
    let mut array = KairoArrowArray::default();
    let status = unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
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
    // Reaching here means populated export landed; Task 2.2 must replace this
    // probe with the real behavioural acceptance assertions.
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
    // Reaching here means the v2 C-ABI schema landed; Task 2.2 must replace
    // this probe with per-column type/nullability assertions.
}

#[test]
#[should_panic(expected = "multi-batch populated stream export required")]
fn event_log_export_supports_multiple_populated_batches() {
    let mut engine_handle = dispatched_engine(2);

    // Two successive exports of the same engine can never express batch
    // boundaries: the current surface exports one descriptor-only struct per
    // call and no ArrowArrayStream. Task 2.2 must add owned populated
    // multi-batch (stream) export before this contract can be asserted.
    let mut first = KairoArrowArray::default();
    let status = unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut first) };
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
    let status = unsafe { kairo_ecs_export_event_log_arrow_array(&mut engine_handle, &mut array) };
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

#[test]
fn exported_array_struct_survives_engine_free_until_release_once() {
    // Lifetime/release contract observed on the current struct surface: the
    // export carries a release callback that must be invoked exactly once by
    // the consumer, and the exported struct must stay valid after engine
    // destruction until that single release. With descriptor-only exports no
    // owned buffers exist yet, so the full buffer-outlives-engine contract is
    // asserted by Task 2.2's replacement tests.
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
    assert!(
        array.buffers.is_null(),
        "released array must not retain buffers"
    );

    // The spent exported struct must still be a plain value after engine
    // destruction; freeing the engine never dangles the (already released)
    // export surface.
    assert_eq!(
        kairo_ecs_engine_free(engine_handle),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );
}
