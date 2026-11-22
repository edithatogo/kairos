use kairo_ecs_arrow::polars_interop::read_ipc_stream;
use polars_arrow::io::ipc::read::read_stream_metadata;
use std::{io::Cursor, process::Command};

fn emit(name: &str) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cohort-consumer"))
        .args(["--emit", name])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn executable_emits_all_three_ipc_streams() {
    let baseline_bytes = emit("cohort-baseline");
    let policy_bytes = emit("cohort-policy");
    let baseline_header = read_stream_metadata(&mut baseline_bytes.as_slice()).unwrap();
    let policy_header = read_stream_metadata(&mut policy_bytes.as_slice()).unwrap();
    assert_eq!(baseline_header.schema, policy_header.schema);
    assert!(policy_header
        .schema
        .iter_values()
        .all(|field| !field.is_nullable));
    assert_eq!(
        baseline_header.custom_schema_metadata,
        policy_header.custom_schema_metadata
    );
    let cohort_metadata = policy_header.custom_schema_metadata.unwrap();
    assert_eq!(
        cohort_metadata["kairo_ecs.projected_schema"].as_str(),
        "kairo_ecs.cohort_transition.v1"
    );
    assert_eq!(
        cohort_metadata["kairo_ecs.tick_unit"].as_str(),
        "millisecond"
    );
    assert_eq!(cohort_metadata["kairo_ecs.run_seed"].as_str(), "753");

    let baseline = read_ipc_stream(Cursor::new(baseline_bytes)).unwrap();
    let policy = read_ipc_stream(Cursor::new(policy_bytes)).unwrap();
    assert!(baseline.height() > 0 && policy.height() > baseline.height());
    assert_eq!(baseline.schema(), policy.schema());
    assert!(policy.column("entity_id_index").unwrap().u64().is_ok());

    let exposure_bytes = emit("spatial-exposure");
    let exposure_header = read_stream_metadata(&mut exposure_bytes.as_slice()).unwrap();
    let nullable_fields: Vec<_> = exposure_header
        .schema
        .iter_values()
        .filter(|field| field.is_nullable)
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(nullable_fields, ["nearest_feature_metres"]);
    let exposure_metadata = exposure_header.custom_schema_metadata.unwrap();
    assert_eq!(
        exposure_metadata["kairo_ecs.projected_schema"].as_str(),
        "kairo_ecs.spatial_exposure.v1"
    );
    assert_eq!(exposure_metadata["kairo_ecs.length_unit"].as_str(), "metre");

    let exposure = read_ipc_stream(Cursor::new(exposure_bytes)).unwrap();
    assert_eq!(exposure.height(), 2);
    let distances = exposure
        .column("nearest_feature_metres")
        .unwrap()
        .f64()
        .unwrap();
    assert_eq!(distances.get(0), Some(5.0));
    assert_eq!(distances.get(1), None);
}

#[test]
fn unknown_fixture_fails_without_emitting_bytes() {
    let output = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cohort-consumer"))
        .args(["--emit", "unknown"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cohort-baseline"));
}
