use std::ffi::CStr;

use kairo_ecs_ffi::{
    kairo_ecs_buffer_free, kairo_ecs_cancel_event, kairo_ecs_engine_current_time,
    kairo_ecs_engine_free, kairo_ecs_engine_new, kairo_ecs_engine_reset,
    kairo_ecs_ffi_version, kairo_ecs_last_error_message, kairo_ecs_run_for,
    kairo_ecs_run_until, kairo_ecs_schedule_at, kairo_ecs_stats, kairo_ecs_step,
    kairo_ecs_telemetry_flush_ipc, KairoEcsStatusCode,
};

fn free_engine(handle: u64) {
    kairo_ecs_engine_free(handle);
}

#[test]
fn test_ffi_version() {
    assert_ne!(kairo_ecs_ffi_version(), 0);
}

#[test]
fn test_engine_create_free() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);
    let status = kairo_ecs_engine_free(handle);
    assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
}

#[test]
fn test_engine_double_free() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);
    assert_eq!(
        kairo_ecs_engine_free(handle),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );
    assert_eq!(
        kairo_ecs_engine_free(handle),
        KairoEcsStatusCode::KAIRO_ECS_ERR_ALREADY_FREED
    );
}

#[test]
fn test_schedule_and_step() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    let event = kairo_ecs_schedule_at(handle, 10, 0, 1);
    assert_ne!(event, 0);

    assert_eq!(kairo_ecs_step(handle), KairoEcsStatusCode::KAIRO_ECS_OK);

    assert_eq!(kairo_ecs_engine_current_time(handle), 10);

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.scheduled_events, 1);
    assert_eq!(stats.dispatched_events, 1);
    assert_eq!(stats.pending_events, 0);

    free_engine(handle);
}

#[test]
fn test_schedule_ordering() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    kairo_ecs_schedule_at(handle, 30, 0, 1);
    kairo_ecs_schedule_at(handle, 10, 0, 2);
    kairo_ecs_schedule_at(handle, 20, 0, 3);

    kairo_ecs_step(handle);
    assert_eq!(kairo_ecs_engine_current_time(handle), 10);

    kairo_ecs_step(handle);
    assert_eq!(kairo_ecs_engine_current_time(handle), 20);

    kairo_ecs_step(handle);
    assert_eq!(kairo_ecs_engine_current_time(handle), 30);

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.scheduled_events, 3);
    assert_eq!(stats.dispatched_events, 3);
    assert_eq!(stats.pending_events, 0);

    free_engine(handle);
}

#[test]
fn test_schedule_cancel() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    let event = kairo_ecs_schedule_at(handle, 10, 0, 1);
    assert_ne!(event, 0);

    assert_eq!(
        kairo_ecs_cancel_event(handle, event),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );

    kairo_ecs_step(handle);

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.dispatched_events, 0);
    assert_eq!(stats.cancelled_events, 1);

    free_engine(handle);
}

#[test]
fn test_error_message() {
    // Trigger error by passing an invalid handle to a function
    kairo_ecs_stats(99999);

    let ptr = kairo_ecs_last_error_message();
    assert!(!ptr.is_null());

    // SAFETY: ptr is a valid CString from last_error_message
    let message = unsafe { CStr::from_ptr(ptr) };
    assert!(!message.to_bytes().is_empty());
}

#[test]
fn test_run_for_limit() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    for i in 0..10 {
        let event = kairo_ecs_schedule_at(handle, 10, 0, i);
        assert_ne!(event, 0);
    }

    assert_eq!(
        kairo_ecs_run_for(handle, 5),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.scheduled_events, 10);
    assert_eq!(stats.dispatched_events, 5);
    assert_eq!(stats.pending_events, 5);

    free_engine(handle);
}

#[test]
fn test_run_until_limit() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    kairo_ecs_schedule_at(handle, 10, 0, 1);
    kairo_ecs_schedule_at(handle, 20, 0, 2);
    kairo_ecs_schedule_at(handle, 30, 0, 3);

    assert_eq!(
        kairo_ecs_run_until(handle, 15),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.scheduled_events, 3);
    assert_eq!(stats.dispatched_events, 1);
    assert_eq!(stats.pending_events, 2);

    free_engine(handle);
}

#[test]
fn test_stats() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    kairo_ecs_schedule_at(handle, 1, 0, 1);
    kairo_ecs_schedule_at(handle, 2, 0, 2);
    kairo_ecs_schedule_at(handle, 3, 0, 3);

    kairo_ecs_step(handle);
    kairo_ecs_step(handle);
    kairo_ecs_step(handle);

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.dispatched_events, 3);
    assert_eq!(stats.scheduled_events, 3);
    assert_eq!(stats.pending_events, 0);

    free_engine(handle);
}

#[test]
fn test_engine_reset() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    kairo_ecs_schedule_at(handle, 10, 0, 1);
    kairo_ecs_schedule_at(handle, 20, 0, 2);
    kairo_ecs_step(handle);

    assert_eq!(
        kairo_ecs_engine_reset(handle),
        KairoEcsStatusCode::KAIRO_ECS_OK
    );

    let stats = kairo_ecs_stats(handle);
    assert_eq!(stats.scheduled_events, 0);
    assert_eq!(stats.dispatched_events, 0);
    assert_eq!(stats.pending_events, 0);

    let time = kairo_ecs_engine_current_time(handle);
    assert_eq!(time, 0);

    free_engine(handle);
}

#[test]
fn test_telemetry_flush() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    kairo_ecs_schedule_at(handle, 5, 0, 1);
    kairo_ecs_schedule_at(handle, 10, 0, 2);
    kairo_ecs_step(handle);
    kairo_ecs_step(handle);

    let buffer = kairo_ecs_telemetry_flush_ipc(handle);
    assert!(!buffer.data.is_null());
    assert_ne!(buffer.len, 0);

    // SAFETY: buffer.data is a valid pointer to buffer.len bytes from telemetry_flush_ipc
    let data = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) };
    assert!(!data.is_empty());

    kairo_ecs_buffer_free(buffer);
    free_engine(handle);
}
