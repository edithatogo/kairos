#[path = "../src/arrow_output.rs"]
mod output;

use serde_json::{Value, json};

fn residual() -> Value {
    json!({
      "record_type":"calibration_residual.v1", "schema_version":"calibration-v1",
      "dataset_id":"d", "scenario_id":"s", "run_id":"r", "candidate_id":"c",
      "case_key":"case", "task_key":"task", "occurrence":0, "endpoint":"discharge",
      "fidelity":"FreeRunning", "observed_ticks":"0", "predicted_ticks":"340282366920938463463374607431768211455",
      "residual_status":"computed", "residual_sign":"positive", "residual_magnitude":"340282366920938463463374607431768211455",
      "prediction_unclamped":true, "anchor_role":"none", "feasibility":"feasible", "censor_status":"not_censored",
      "study_id":"study", "replication_id":"rep", "seed_schedule_id":"sched", "seed_purpose":"calibration",
      "seed_map_ref":"map", "seed_contract_version":"kairoecs.seed-purpose.v1", "mapping_version":"m",
      "parameter_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "graph_hash":null, "causal_ref":null
    })
}
fn metric() -> Value {
    json!({
      "record_type":"calibration_metric.v1", "schema_version":"calibration-v1", "metric":"W1",
      "algorithm_version":"w1.v1", "endpoint":"discharge", "strata":{"z":[2,1],"a":true},
      "window":{"start_ticks":"0","end_ticks":"340282366920938463463374607431768211455"}, "units":"ns",
      "reference_count":1, "simulation_count":1, "excluded_count":0, "censored_count":0, "missing_count":0,
      "unmatched_count":0, "failed_count":0, "infeasible_count":0, "value":null, "status":"insufficient_data",
      "uncertainty":null, "provenance":{"dataset_id":"d","run_id":"r","mapping_version":"m",
      "seed_schedule_id":"sched","seed_map_ref":"map","seed_contract_version":"kairoecs.seed-purpose.v1","parameter_hash":null}
    })
}
#[test]
fn c43_schemas_preserve_empty_shapes_and_output_types() {
    let r = output::schema("calibration_residual.v1").unwrap();
    let m = output::schema("calibration_metric.v1").unwrap();
    assert_eq!(r.fields().len(), 30);
    assert_eq!(
        r.metadata().get("format").unwrap(),
        "kairoecs.calibration.output"
    );
    assert_eq!(r.field(27).data_type(), &arrow_schema::DataType::Utf8);
    assert!(!r.field(27).is_nullable());
    assert_eq!(
        r.field(11).metadata().get("encoding").unwrap(),
        "unsigned_u128_le"
    );
    assert_eq!(m.fields().len(), 27);
    assert_eq!(
        output::encode("calibration_residual.v1", &[])
            .unwrap()
            .num_rows(),
        0
    );
    assert_eq!(
        output::encode("calibration_metric.v1", &[])
            .unwrap()
            .num_rows(),
        0
    );
}
#[test]
fn c43_roundtrips_full_width_ticks_nulls_and_canonical_strata() {
    for (kind, row) in [
        ("calibration_residual.v1", residual()),
        ("calibration_metric.v1", metric()),
    ] {
        let batch = output::encode(kind, std::slice::from_ref(&row)).unwrap();
        let decoded = output::decode(kind, &batch).unwrap();
        assert_eq!(decoded, vec![row]);
    }
}
#[test]
fn c43_rejects_inconsistent_residual_unknown_fields_and_duplicate_records() {
    let mut bad = residual();
    bad["residual_magnitude"] = json!("1");
    assert!(output::encode("calibration_residual.v1", &[bad]).is_err());
    let mut extra = residual();
    extra["surprise"] = json!(true);
    assert!(output::encode("calibration_residual.v1", &[extra]).is_err());
    assert!(output::encode("calibration_residual.v1", &[residual(), residual()]).is_err());
    let mut bad_metric = metric();
    bad_metric["reference_count"] = json!(true);
    assert!(output::encode("calibration_metric.v1", &[bad_metric]).is_err());
    let mut bad_metric = metric();
    bad_metric["uncertainty"] = json!({"method":"unknown"});
    assert!(output::encode("calibration_metric.v1", &[bad_metric]).is_err());
}
#[test]
fn c43_sort_uses_full_residual_identity_and_numeric_occurrence() {
    let mut a = residual();
    a["occurrence"] = json!(10);
    let mut b = residual();
    b["occurrence"] = json!(2);
    let mut c = b.clone();
    c["dataset_id"] = json!("other-dataset");
    let batch = output::encode("calibration_residual.v1", &[a, b, c]).unwrap();
    let decoded = output::decode("calibration_residual.v1", &batch).unwrap();
    assert_eq!(decoded[0]["occurrence"], json!(2));
    assert_eq!(decoded[0]["dataset_id"], json!("d"));
    assert_eq!(decoded[1]["occurrence"], json!(10));
    assert_eq!(decoded[2]["dataset_id"], json!("other-dataset"));
}
#[test]
fn c43_validates_frozen_keys_metric_semantics_and_decode_rows() {
    let mut bad = residual();
    bad["seed_purpose"] = json!("other");
    assert!(output::encode("calibration_residual.v1", &[bad]).is_err());

    let mut bad = metric();
    bad["window"]["extra"] = json!(0);
    assert!(output::encode("calibration_metric.v1", &[bad]).is_err());
    let mut bad = metric();
    bad["provenance"]["extra"] = json!(0);
    assert!(output::encode("calibration_metric.v1", &[bad]).is_err());
    let mut bad = metric();
    bad["status"] = json!("computed");
    assert!(output::encode("calibration_metric.v1", &[bad]).is_err());
    let mut bad = metric();
    bad["metric"] = json!("KS_D");
    bad["units"] = json!("dimensionless");
    bad["status"] = json!("computed");
    bad["value"] = json!(1.1);
    assert!(output::encode("calibration_metric.v1", &[bad]).is_err());
    let mut bad = metric();
    bad["provenance"]["seed_schedule_id"] = Value::Null;
    bad["provenance"]["seed_map_ref"] = Value::Null;
    assert!(output::encode("calibration_metric.v1", &[bad]).is_err());

    let batch = output::encode("calibration_metric.v1", &[metric()]).unwrap();
    let mut columns = batch.columns().to_vec();
    columns[5] = std::sync::Arc::new(arrow_array::StringArray::from(vec!["{ \"a\":1}"]));
    let malformed = arrow_array::RecordBatch::try_new(batch.schema(), columns).unwrap();
    assert!(output::decode("calibration_metric.v1", &malformed).is_err());
}
#[cfg(all(feature = "ipc", feature = "parquet"))]
#[test]
fn c43_actual_ipc_file_stream_and_parquet_roundtrip() {
    use kairo_ecs_arrow_io::{
        IoLimits, read_ipc_file, read_ipc_stream, read_parquet, write_ipc_file, write_ipc_stream,
        write_parquet,
    };
    use std::sync::Arc;
    let limits = IoLimits::default();
    for (kind, row) in [
        ("calibration_residual.v1", residual()),
        ("calibration_metric.v1", metric()),
    ] {
        let schema = output::schema(kind).unwrap();
        let batch = output::encode(kind, std::slice::from_ref(&row)).unwrap();
        let file =
            write_ipc_file(Arc::clone(&schema), std::slice::from_ref(&batch), limits).unwrap();
        let stream =
            write_ipc_stream(Arc::clone(&schema), std::slice::from_ref(&batch), limits).unwrap();
        let parquet =
            write_parquet(Arc::clone(&schema), std::slice::from_ref(&batch), limits).unwrap();
        for read in [
            read_ipc_file(&file, Arc::clone(&schema), limits).unwrap(),
            read_ipc_stream(&stream, Arc::clone(&schema), limits).unwrap(),
            read_parquet(&parquet, Arc::clone(&schema), limits).unwrap(),
        ] {
            assert_eq!(output::decode(kind, &read[0]).unwrap(), vec![row.clone()]);
        }
        if let Some(dir) = std::env::var_os("C43_OUTPUT_DIR") {
            std::fs::create_dir_all(std::path::PathBuf::from(&dir).join("codec")).unwrap();
            let p = std::path::PathBuf::from(dir).join("codec");
            let name = if kind == "calibration_residual.v1" {
                "residual"
            } else {
                "metric"
            };
            std::fs::write(p.join(format!("{name}.ipc_file")), file).unwrap();
            std::fs::write(p.join(format!("{name}.ipc_stream")), stream).unwrap();
            std::fs::write(p.join(format!("{name}.parquet")), parquet).unwrap();
            std::fs::write(
                p.join(format!("{name}.json")),
                serde_json::to_vec(&output::decode(kind, &batch).unwrap()).unwrap(),
            )
            .unwrap();
        }
    }
}
