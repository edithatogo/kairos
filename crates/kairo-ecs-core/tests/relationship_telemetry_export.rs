#![forbid(unsafe_code)]

use kairo_ecs_core::{
    LinkedEntityError, RelationshipTelemetryBatch, RelationshipTelemetryRecord,
    RELATIONSHIP_SCHEMA_VERSION,
};

#[test]
fn test_relationship_telemetry_batch_roundtrip() {
    let rec1 = RelationshipTelemetryRecord {
        schema_version: RELATIONSHIP_SCHEMA_VERSION,
        run_id: "test-run-001".to_string(),
        event_seq: 1,
        time_ticks: 100,
        parent_index: 10,
        parent_generation: 1,
        child_index: 20,
        child_generation: 1,
        parent_model_id: Some(5001),
        child_model_id: Some(6001),
        kind: 1,
        policy: 0,
        lifecycle_event: "attached".to_string(),
    };

    let rec2 = RelationshipTelemetryRecord {
        schema_version: RELATIONSHIP_SCHEMA_VERSION,
        run_id: "test-run-001".to_string(),
        event_seq: 2,
        time_ticks: 150,
        parent_index: 10,
        parent_generation: 1,
        child_index: 21,
        child_generation: 1,
        parent_model_id: Some(5001),
        child_model_id: None,
        kind: 3,
        policy: 1,
        lifecycle_event: "attached".to_string(),
    };

    let rec3 = RelationshipTelemetryRecord {
        schema_version: RELATIONSHIP_SCHEMA_VERSION,
        run_id: "test-run-001".to_string(),
        event_seq: 3,
        time_ticks: 200,
        parent_index: 10,
        parent_generation: 1,
        child_index: 21,
        child_generation: 1,
        parent_model_id: Some(5001),
        child_model_id: None,
        kind: 3,
        policy: 1,
        lifecycle_event: "cascaded_despawn".to_string(),
    };

    let batch =
        RelationshipTelemetryBatch::new(vec![rec1.clone(), rec2.clone(), rec3.clone()]).unwrap();
    assert_eq!(batch.len(), 3);

    // Roundtrip through bytes
    let bytes = batch.to_bytes();
    let restored = RelationshipTelemetryBatch::from_bytes(&bytes).unwrap();
    assert_eq!(batch, restored);
    assert_eq!(restored.records()[0], rec1);
    assert_eq!(restored.records()[1], rec2);
    assert_eq!(restored.records()[2], rec3);

    // Verify model identities and versioned generational handles preserved
    assert_eq!(restored.records()[0].parent_model_id, Some(5001));
    assert_eq!(restored.records()[0].child_model_id, Some(6001));
    assert_eq!(restored.records()[1].child_model_id, None);
}

#[test]
fn test_relationship_telemetry_schema_version_validation() {
    let invalid_version_rec = RelationshipTelemetryRecord {
        schema_version: 999, // incompatible version
        run_id: "run-bad".to_string(),
        event_seq: 1,
        time_ticks: 10,
        parent_index: 1,
        parent_generation: 1,
        child_index: 2,
        child_generation: 1,
        parent_model_id: None,
        child_model_id: None,
        kind: 1,
        policy: 0,
        lifecycle_event: "attached".to_string(),
    };

    let err = invalid_version_rec.validate().unwrap_err();
    assert_eq!(err, LinkedEntityError::UnsupportedSchemaVersion(999));
}

#[test]
fn test_relationship_telemetry_duplicate_attached_link_rejection() {
    let rec1 = RelationshipTelemetryRecord {
        schema_version: RELATIONSHIP_SCHEMA_VERSION,
        run_id: "run-dup".to_string(),
        event_seq: 1,
        time_ticks: 10,
        parent_index: 1,
        parent_generation: 1,
        child_index: 2,
        child_generation: 1,
        parent_model_id: None,
        child_model_id: None,
        kind: 1,
        policy: 0,
        lifecycle_event: "attached".to_string(),
    };

    let rec2 = rec1.clone(); // identical duplicate attachment

    let err = RelationshipTelemetryBatch::new(vec![rec1, rec2]).unwrap_err();
    assert!(matches!(
        err,
        LinkedEntityError::DuplicateRelationship { .. }
    ));
}

#[test]
fn test_relationship_telemetry_self_cycle_rejection() {
    let self_cycle_rec = RelationshipTelemetryRecord {
        schema_version: RELATIONSHIP_SCHEMA_VERSION,
        run_id: "run-cycle".to_string(),
        event_seq: 1,
        time_ticks: 10,
        parent_index: 5,
        parent_generation: 2,
        child_index: 5,
        child_generation: 2,
        parent_model_id: None,
        child_model_id: None,
        kind: 1,
        policy: 0,
        lifecycle_event: "attached".to_string(),
    };

    let err = self_cycle_rec.validate().unwrap_err();
    assert!(matches!(err, LinkedEntityError::CycleDetected { .. }));
}
