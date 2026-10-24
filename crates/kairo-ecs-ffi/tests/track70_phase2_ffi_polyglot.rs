use kairo_ecs_ffi::*;
use std::ffi::CStr;

#[test]
fn test_arrow_c_schema_export_v2_and_children() {
    let mut schema = KairoArrowSchema::default();
    let status = unsafe { kairo_ecs_export_event_log_arrow_schema(&mut schema) };
    assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);

    assert!(!schema.format.is_null());
    let format_str = unsafe { CStr::from_ptr(schema.format) }.to_str().unwrap();
    assert_eq!(format_str, "+s");

    assert!(!schema.name.is_null());
    let name_str = unsafe { CStr::from_ptr(schema.name) }.to_str().unwrap();
    assert_eq!(name_str, "kairo_ecs.event_log.v2");

    assert_eq!(schema.n_children, 14);
    assert!(!schema.children.is_null());

    let expected_cols = [
        ("schema_version", "S"),
        ("run_id", "u"),
        ("event_id_index", "L"),
        ("event_id_gen", "I"),
        ("entity_id_index", "L"),
        ("entity_id_gen", "I"),
        ("time_ticks_hi", "L"),
        ("time_ticks_lo", "L"),
        ("time_scale", "u"),
        ("priority", "i"),
        ("sequence", "L"),
        ("event_kind", "u"),
        ("status", "u"),
        ("payload_ref", "u"),
    ];

    for (i, (expected_name, expected_fmt)) in expected_cols.iter().enumerate() {
        let child_ptr = unsafe { *schema.children.add(i) };
        assert!(!child_ptr.is_null());
        let child = unsafe { &*child_ptr };

        let child_name = unsafe { CStr::from_ptr(child.name) }.to_str().unwrap();
        let child_fmt = unsafe { CStr::from_ptr(child.format) }.to_str().unwrap();

        assert_eq!(&child_name, expected_name);
        assert_eq!(&child_fmt, expected_fmt);
    }

    // Single release lifecycle
    assert!(schema.release.is_some());
    unsafe {
        if let Some(rel) = schema.release {
            rel(&mut schema);
        }
    }
    assert!(schema.release.is_none());
}

#[test]
fn test_arrow_c_array_empty_and_populated_release_lifetime() {
    let handle = kairo_ecs_engine_new();
    assert_ne!(handle, 0);

    // Empty batch export
    let mut empty_array = KairoArrowArray::default();
    let status =
        unsafe { kairo_ecs_export_event_log_arrow_array(&mut (handle as u64), &mut empty_array) };
    assert_eq!(status, KairoEcsStatusCode::KAIRO_ECS_OK);
    assert_eq!(empty_array.length, 0);
    assert_eq!(empty_array.n_children, 14);

    unsafe {
        if let Some(rel) = empty_array.release {
            rel(&mut empty_array);
        }
    }
    assert!(empty_array.release.is_none());

    // Schedule and step events
    kairo_ecs_schedule_at(handle, 10, 0, 1);
    kairo_ecs_schedule_at(handle, 20, 1, 2);
    kairo_ecs_step(handle);
    kairo_ecs_step(handle);

    let mut populated_array = KairoArrowArray::default();
    let status2 = unsafe {
        kairo_ecs_export_event_log_arrow_array(&mut (handle as u64), &mut populated_array)
    };
    assert_eq!(status2, KairoEcsStatusCode::KAIRO_ECS_OK);
    assert_eq!(populated_array.length, 2);
    assert_eq!(populated_array.n_children, 14);
    assert!(!populated_array.children.is_null());

    // Now destroy the engine - buffers must outlive engine mutation and destruction!
    let free_status = kairo_ecs_engine_free(handle);
    assert_eq!(free_status, KairoEcsStatusCode::KAIRO_ECS_OK);

    // Buffers and child arrays remain valid
    for i in 0..14 {
        let child_ptr = unsafe { *populated_array.children.add(i) };
        assert!(!child_ptr.is_null());
        let child = unsafe { &*child_ptr };
        assert_eq!(child.length, 2);
        assert!(child.n_buffers >= 2);
        assert!(!child.buffers.is_null());
    }

    // Release array exactly once
    unsafe {
        if let Some(rel) = populated_array.release {
            rel(&mut populated_array);
        }
    }
    assert!(populated_array.release.is_none());
}
