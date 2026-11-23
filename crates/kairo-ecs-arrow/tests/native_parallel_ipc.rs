#![cfg(all(feature = "parallel-io", feature = "polars-native"))]

use std::fs;
use std::io::{self, Cursor, Write};
use std::path::PathBuf;

use kairo_ecs_arrow::native_ipc::NativeIpcBatch;
use kairo_ecs_arrow::{
    compute_sha256_hex, ArrowError, CheckpointFormat, CheckpointManifest, ContiguousBlock,
    EventLogRecord, EventStatus, ParallelIoRecordBatch,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};
use polars_arrow::io::ipc::read::{read_stream_metadata, StreamReader};
use polars_arrow::io::ipc::write::{default_ipc_fields, schema_to_bytes};

#[test]
fn named_outputs_round_trip_and_match_physical_receipts() -> Result<(), Box<dyn std::error::Error>>
{
    let make_event = |id, tick, sequence, entity| {
        EventLogRecord::dispatched(
            "run-箭",
            DispatchedEvent::new(
                EventId::new(id, 7),
                SimTime::from_ticks(tick),
                -3,
                sequence,
                entity,
                EventKind::custom(17),
            ),
        )
    };
    let first = make_event(1, 5, 31, None);
    let mut second = make_event(2, (1 << 127) + 9, 32, Some(EntityId::new(40, u32::MAX)));
    second.status = EventStatus::Cancelled;
    second.payload_ref = Some("payload/é".into());
    let third = make_event(u64::MAX, u128::MAX - 2, 33, Some(EntityId::new(50, 2)));
    let events = ParallelIoRecordBatch::from_records(vec![first, second, third])?;
    let mut checkpoint = CheckpointManifest::from_batch("cp-1", CheckpointFormat::Hdf5, &events)?;
    let fixture_dir = tempfile::tempdir()?;
    let path = std::env::var_os("KAIROS_NATIVE_IPC_DIR")
        .map_or_else(|| fixture_dir.path().to_path_buf(), PathBuf::from);
    checkpoint.write_contract_file(path.join("checkpoint.contract"))?;
    let total_bytes = checkpoint.blocks[0].byte_len;
    checkpoint.blocks = vec![
        ContiguousBlock {
            offset_bytes: 0,
            byte_len: 128,
            row_count: 1,
        },
        ContiguousBlock {
            offset_bytes: 128,
            byte_len: total_bytes - 128,
            row_count: 2,
        },
    ];

    let outputs = [
        (
            "events.arrows",
            "kairo_ecs.event_log.v2",
            NativeIpcBatch::from_event_log(&events)?,
        ),
        (
            "world.arrows",
            "kairo_ecs.world_snapshot.v1",
            NativeIpcBatch::from_world_snapshot(&[(u64::MAX, u32::MAX, true), (7, 3, false)])?,
        ),
        (
            "checkpoint.arrows",
            "kairo_ecs.checkpoint_metadata.v1",
            NativeIpcBatch::from_checkpoint_metadata(&checkpoint)?,
        ),
    ];
    for (name, stream, native) in outputs {
        let file = path.join(name);
        let manifest = native.write_contiguous_to_path(&file)?;
        let bytes = fs::read(file)?;
        let mut cursor = Cursor::new(&bytes);
        let metadata = read_stream_metadata(&mut cursor)?;
        assert_eq!(metadata.schema, *native.record_batch().schema());
        assert_eq!(
            metadata.custom_schema_metadata.as_ref().unwrap()["kairo.stream"].as_str(),
            stream
        );
        let schema_bytes = schema_to_bytes(
            &metadata.schema,
            &default_ipc_fields(metadata.schema.iter_values()),
            metadata.custom_schema_metadata.as_ref(),
        );
        let schema_start = 8;
        assert_eq!(
            bytes[schema_start..schema_start + schema_bytes.len()],
            schema_bytes
        );
        assert_eq!(
            manifest.schema_fingerprint,
            compute_sha256_hex(&schema_bytes)
        );
        let mut reader = StreamReader::new(cursor, metadata, None);
        assert_eq!(&reader.next().unwrap()?.unwrap(), native.record_batch());
        assert!(reader.next().is_none());

        assert_eq!(manifest.stream, stream);
        assert_eq!(manifest.row_count, native.record_batch().len());
        assert_eq!(manifest.encoded_len, bytes.len());
        assert_eq!(
            manifest.blocks,
            [ContiguousBlock {
                byte_len: bytes.len() as u64,
                offset_bytes: 0,
                row_count: manifest.row_count,
            }]
        );
        let checksum = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        });
        assert_eq!(manifest.checksum, checksum);

        let mut short_sink = ShortSink::default();
        assert_eq!(native.write_contiguous(&mut short_sink)?, manifest);
        assert_eq!(short_sink.bytes, bytes);
        short_sink.fail_flush = true;
        assert!(matches!(
            native.write_contiguous(short_sink),
            Err(ArrowError::Io(_))
        ));
    }

    checkpoint.row_count += 1;
    assert!(matches!(
        NativeIpcBatch::from_checkpoint_metadata(&checkpoint),
        Err(ArrowError::WrongCellCount { .. })
    ));
    Ok(())
}

#[test]
fn empty_event_stream_has_schema_without_row_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let events = ParallelIoRecordBatch::from_records(Vec::new())?;
    let native = NativeIpcBatch::from_event_log(&events)?;
    let mut bytes = Vec::new();
    let manifest = native.write_contiguous(&mut bytes)?;
    assert_eq!(manifest.row_count, 0);
    assert!(manifest.blocks.is_empty());
    assert_eq!(manifest.encoded_len, bytes.len());
    let mut cursor = Cursor::new(bytes);
    let metadata = read_stream_metadata(&mut cursor)?;
    assert_eq!(metadata.schema.len(), 14);
    assert_eq!(&metadata.schema, native.record_batch().schema());
    assert!(StreamReader::new(cursor, metadata, None).next().is_none());
    Ok(())
}

#[derive(Default)]
struct ShortSink {
    bytes: Vec<u8>,
    fail_flush: bool,
}

impl Write for ShortSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.write(&bytes[..bytes.len().min(7)])
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            return Err(io::Error::other("fixture flush failure"));
        }
        Ok(())
    }
}
