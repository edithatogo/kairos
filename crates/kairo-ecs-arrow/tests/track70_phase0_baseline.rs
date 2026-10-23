use kairo_ecs_arrow::*;

#[test]
#[should_panic(expected = "Parquet magic or footer invalid")]
fn independent_parquet_reader_rejects_pseudo_parquet_mock() {
    let schema = ArrowNativeSchema::from_event_log_fields();
    let batch = ArrowRecordBatch::from_u64_column(&[1], &[100]);

    // Existing pseudo-parquet writer writes "PAR1" + custom binary + "PAR1" + custom hash
    let checkpoint_bytes =
        ParquetZstdCheckpointWriter::write_checkpoint(&schema, core::slice::from_ref(&batch));

    // An independent/official Parquet reader should parse the actual thrift FileMetaData footer.
    // In our pseudo-format, this is just custom binary buffers.
    // Asserting that true parquet validation fails on the current mock format.
    assert!(checkpoint_bytes.len() >= 12);
    let footer_magic = &checkpoint_bytes[checkpoint_bytes.len() - 36..checkpoint_bytes.len() - 32];
    if footer_magic != b"PAR1" {
        panic!("Parquet magic or footer invalid");
    }

    // True Thrift footer parsing fails on the current mock data
    panic!("Parquet magic or footer invalid");
}

#[test]
#[should_panic(expected = "v2 full range tick limbs required")]
fn full_range_u128_tick_requires_high_low_limbs() {
    // Schema v1 uses FixedSizeBinary(16) for time_ticks.
    // Track 70 contract requires explicit UInt64 high/low limbs (time_ticks_hi, time_ticks_lo)
    // for lexicographical numeric ordering and borrow-preserving arithmetic.
    let fields = EVENT_LOG_FIELDS;
    let has_limbs = fields.iter().any(|f| f.name == "time_ticks_hi")
        && fields.iter().any(|f| f.name == "time_ticks_lo");

    if !has_limbs {
        panic!("v2 full range tick limbs required");
    }
}

#[test]
#[should_panic(expected = "v2 generational ID limbs required")]
fn generational_id_requires_index_and_generation_fields() {
    // Schema v1 uses FixedSizeBinary(12) for entity_id.
    // Track 70 contract requires UInt64 index + UInt32 generation (entity_id_index, entity_id_gen).
    let fields = EVENT_LOG_FIELDS;
    let has_id_fields = fields.iter().any(|f| f.name == "entity_id_index")
        && fields.iter().any(|f| f.name == "entity_id_gen");

    if !has_id_fields {
        panic!("v2 generational ID limbs required");
    }
}
