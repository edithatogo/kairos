// Compile against the candidate kairo-ecs-arrow with polars-native enabled.
use kairo_ecs_arrow::polars_interop::{records_to_dataframe, write_ipc_stream, write_parquet_zstd};
use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
use kairo_ecs_types::EventId;
use std::fs::File;
fn main() {
    let records = [0, u128::MAX].map(|tick| EventLogRecord {
        schema_version: SCHEMA_VERSION_V2,
        run_id: "track51".into(),
        event_id: EventId::new(if tick == 0 { 0 } else { 1 }, 1),
        entity_id: None,
        time_ticks: tick,
        time_scale: "ticks".into(),
        priority: 0,
        sequence: if tick == 0 { 0 } else { 1 },
        event_kind: "test".into(),
        status: EventStatus::Dispatched,
        payload_ref: None,
    });
    let mut frame = records_to_dataframe(&records).unwrap();
    write_ipc_stream(&mut frame, File::create_new("events.ipc").unwrap()).unwrap();
    write_parquet_zstd(&mut frame, File::create_new("events.parquet").unwrap()).unwrap();
}
