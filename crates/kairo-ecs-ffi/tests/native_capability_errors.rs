use std::ptr;

use kairo_ecs_ffi::{
    KairoArrowArrayStream, KairoEcsStatusCode, kairo_ecs_cancel_event, kairo_ecs_engine_free,
    kairo_ecs_engine_new, kairo_ecs_export_event_log_arrow_stream, kairo_ecs_ffi_version,
    kairo_ecs_last_error_string, kairo_ecs_schedule_after, kairo_ecs_schedule_at,
    kairo_ecs_stats, kairo_ecs_step, kairo_ecs_telemetry_flush_ipc,
};

#[test]
fn invalid_and_freed_handles_fail_explicitly_without_fallback() {
    assert_eq!(kairo_ecs_ffi_version(), 1);

    assert_eq!(
        kairo_ecs_engine_free(0),
        KairoEcsStatusCode::KAIRO_ECS_ERR_ALREADY_FREED
    );
    assert!(!kairo_ecs_last_error_string().is_empty());

    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);
    assert_eq!(kairo_ecs_engine_free(handle), KairoEcsStatusCode::KAIRO_ECS_OK);
    assert_eq!(
        kairo_ecs_engine_free(handle),
        KairoEcsStatusCode::KAIRO_ECS_ERR_ALREADY_FREED
    );
    assert!(!kairo_ecs_last_error_string().is_empty());

    let buf = kairo_ecs_telemetry_flush_ipc(handle);
    assert!(buf.data.is_null());
    assert_eq!(buf.len, 0);
}

#[test]
fn invalid_scheduling_and_foreign_cancellation_do_not_mutate_state() {
    let first = kairo_ecs_engine_new();
    let second = kairo_ecs_engine_new();
    assert_ne!(first, 0);
    assert_ne!(second, 0);

    let ev = kairo_ecs_schedule_at(first, 5, 0, 1);
    assert_ne!(ev, 0);
    assert_eq!(kairo_ecs_step(first), KairoEcsStatusCode::KAIRO_ECS_OK);

    // Past tick must fail without mutating stats.
    assert_eq!(kairo_ecs_schedule_at(first, 4, 0, 1), 0);
    assert!(!kairo_ecs_last_error_string().is_empty());

    // Overflowing relative schedule must fail without mutating stats.
    assert_eq!(kairo_ecs_schedule_after(first, u64::MAX, 0, 1), 0);
    assert!(!kairo_ecs_last_error_string().is_empty());

    // Cancelling an event from a foreign engine handle must fail explicitly.
    assert_eq!(
        kairo_ecs_cancel_event(second, ev),
        KairoEcsStatusCode::KAIRO_ECS_ERR_NOT_FOUND
    );

    let stats = kairo_ecs_stats(first);
    assert_eq!(stats.now_ticks, 5);
    assert_eq!(stats.scheduled_events, 1);
    assert_eq!(stats.dispatched_events, 1);
    assert_eq!(stats.pending_events, 0);

    assert_eq!(kairo_ecs_engine_free(first), KairoEcsStatusCode::KAIRO_ECS_OK);
    assert_eq!(kairo_ecs_engine_free(second), KairoEcsStatusCode::KAIRO_ECS_OK);
}

#[test]
fn arrow_stream_export_rejects_null_and_non_positive_batch_sizes() {
    let mut handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    let mut stream = KairoArrowArrayStream::default();
    let invalid_batch = unsafe {
        kairo_ecs_export_event_log_arrow_stream(&mut handle, 0, &mut stream)
    };
    assert_eq!(invalid_batch, KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT);
    assert!(!kairo_ecs_last_error_string().is_empty());

    let null_out = unsafe {
        kairo_ecs_export_event_log_arrow_stream(&mut handle, 16, ptr::null_mut())
    };
    assert_eq!(null_out, KairoEcsStatusCode::KAIRO_ECS_ERR_INVALID_ARGUMENT);

    assert_eq!(kairo_ecs_engine_free(handle), KairoEcsStatusCode::KAIRO_ECS_OK);
}
