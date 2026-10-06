//! Three calibration physical tables: external Python bytes -> Rust -> Python.
#![cfg(all(feature = "ipc", feature = "parquet"))]

use std::{fs, path::PathBuf, sync::Arc};

use arrow_array::RecordBatch;
use arrow_ipc::reader::FileReader;
use arrow_schema::{DataType, SchemaRef};
use kairo_ecs_arrow_io::{
    read_ipc_file, read_ipc_stream, read_parquet, write_ipc_file, write_ipc_stream, write_parquet,
    IoError, IoLimits,
};

fn fixture(name: &str, format: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/calibration_physical_v2")
            .join(format!("{name}.{format}")),
    )
    .expect("committed physical fixture")
}

fn frozen_schema(bytes: &[u8], name: &str) -> SchemaRef {
    let schema = FileReader::try_new(std::io::Cursor::new(bytes), None)
        .expect("external IPC schema")
        .schema();
    assert_eq!(
        schema.metadata().get("format").map(String::as_str),
        Some("careops.calibration.physical")
    );
    assert_eq!(
        schema
            .metadata()
            .get("logical_schema_sha256")
            .map(String::as_str),
        Some("8c46db62f691f243385a4ebdf8a7a3d3670e2655dd0c3f82cba4335ced2a3842")
    );
    assert_eq!(
        schema
            .metadata()
            .get("physical_version")
            .map(String::as_str),
        Some("2")
    );
    let expected = match name {
        "trace" => vec![
            "record_type",
            "schema_version",
            "dataset_id",
            "mapping_version",
            "case_key",
            "source_event_key",
            "occurrence",
            "event_kind",
            "relative_ticks",
            "source_order",
            "event_kind_rank",
            "occurrence_time",
            "source_recorded_time",
            "message_created_time",
            "time_lineage",
            "resource_key",
            "actor_key",
            "location_key",
            "quality_flags",
            "raw_event",
            "disposition",
            "knowledge_availability",
            "presence_fields",
        ],
        "exclusion" => vec![
            "record_type",
            "schema_version",
            "dataset_id",
            "mapping_version",
            "source_event_key",
            "raw_event",
            "raw_time_values",
            "exclusion_reason",
            "lineage",
            "detail",
            "presence_fields",
        ],
        "outcome" => vec![
            "record_type",
            "schema_version",
            "dataset_id",
            "case_key",
            "endpoint",
            "risk_start",
            "last_observed",
            "event_observed",
            "event_time",
            "event_cause",
            "censor_status",
            "censor_reason",
            "cluster_ids",
            "lineage",
            "presence_fields",
        ],
        _ => unreachable!(),
    };
    assert_eq!(
        schema
            .fields()
            .iter()
            .map(|f| f.name().as_str())
            .collect::<Vec<_>>(),
        expected
    );
    if name == "trace" {
        assert_eq!(
            schema
                .field_with_name("relative_ticks")
                .unwrap()
                .data_type(),
            &DataType::FixedSizeBinary(16)
        );
        assert_eq!(
            schema
                .field_with_name("event_kind_rank")
                .unwrap()
                .data_type(),
            &DataType::Utf8
        );
        assert_eq!(
            schema.field_with_name("source_order").unwrap().data_type(),
            &DataType::UInt64
        );
        let DataType::Struct(time) = schema
            .field_with_name("occurrence_time")
            .unwrap()
            .data_type()
        else {
            panic!("typed time struct")
        };
        let utc = time.iter().find(|f| f.name() == "utc_i128_le").unwrap();
        assert_eq!(utc.data_type(), &DataType::FixedSizeBinary(16));
        assert_eq!(
            utc.metadata().get("encoding").map(String::as_str),
            Some("signed_i128_le")
        );
        assert!(!utc.is_nullable());
    }
    schema
}

fn rows(batches: &[RecordBatch]) -> Vec<RecordBatch> {
    batches
        .iter()
        .flat_map(|batch| (0..batch.num_rows()).map(|i| batch.slice(i, 1)))
        .collect()
}

#[test]
fn actual_calibration_tables_roundtrip_with_schema_controls() {
    let output = std::env::var_os("KAIROS_C11_PHYSICAL_OUTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("kairos-c11-physical-{}", std::process::id()))
        });
    fs::create_dir(&output).expect("fresh physical readback directory");
    let limits = IoLimits {
        max_batch_rows: 2,
        max_total_rows: 10_000,
        max_batches: 10_000,
        ..IoLimits::default()
    };
    for name in ["trace", "exclusion", "outcome"] {
        let file = fixture(name, "ipc_file");
        let schema = frozen_schema(&file, name);
        let base = read_ipc_file(&file, Arc::clone(&schema), limits).unwrap();
        let stream = fixture(name, "ipc_stream");
        let pq = fixture(name, "parquet");
        let stream_batches = read_ipc_stream(&stream, Arc::clone(&schema), limits).unwrap();
        let pq_batches = read_parquet(&pq, Arc::clone(&schema), limits).unwrap();
        assert_eq!(rows(&base), rows(&stream_batches));
        assert_eq!(rows(&base), rows(&pq_batches));
        let mut changed = schema.metadata().clone();
        changed.insert("physical_version", "unknown");
        let bad = Arc::new(schema.as_ref().clone().with_metadata(changed));
        assert!(matches!(
            read_ipc_file(&file, Arc::clone(&bad), limits),
            Err(IoError::SchemaMismatch)
        ));
        assert!(matches!(
            read_ipc_stream(&stream, Arc::clone(&bad), limits),
            Err(IoError::SchemaMismatch)
        ));
        assert!(matches!(
            read_parquet(&pq, bad, limits),
            Err(IoError::SchemaMismatch)
        ));
        for (kind, bytes) in [
            (
                "ipc_file",
                write_ipc_file(Arc::clone(&schema), &base, limits).unwrap(),
            ),
            (
                "ipc_stream",
                write_ipc_stream(Arc::clone(&schema), &base, limits).unwrap(),
            ),
            (
                "parquet",
                write_parquet(Arc::clone(&schema), &base, limits).unwrap(),
            ),
        ] {
            fs::write(output.join(format!("{name}.{kind}")), bytes).unwrap();
        }
    }
}
