//! Track 70 Phase 2: owned Arrow C Data Interface export for Schema v2 event logs.
//!
//! Replaces descriptor-only/fabricated capsule exports with populated,
//! ownership-carrying Arrow C Data arrays and C stream export built on the
//! pinned Polars Arrow facilities first (spec "Data and interface contract":
//! "Use Polars Arrow facilities first and direct Apache Arrow only for
//! demonstrably necessary ABI/interchange gaps").
//!
//! # Safety boundaries
//!
//! This module contains no `unsafe` code: this crate forbids `unsafe_code`
//! (workspace lint) and stays the generic, unsafe-free data boundary. The
//! Arrow C Data Interface *conversion* functions used here
//! (`export_array_to_c`, `export_field_to_c`, `export_iterator`) are safe in
//! the pinned polars-arrow version: they produce ABI structs with owned
//! buffers behind the interface release callback. ABI-level *imports* and
//! struct handoffs live in `crates/kairo-ecs-ffi` (which carries the audited
//! `allow(unsafe_code)` boundary) and consume this module's exports plus the
//! [`crate::arrow_c_data::arrow_ffi`] re-export; see reviewer notes in the Track 70 handoff.
//!
//! # Ownership and copies (documented; no universal zero-copy claim)
//!
//! - Exported arrays/streams own their buffers through the C Data Interface
//!   release callback. After handoff the buffers are immutable and outlive
//!   source [`polars::prelude::DataFrame`] destruction and any producing engine lifetime. The
//!   consumer must call `release` exactly once and null the holder first.
//! - Primitive columns convert their existing Polars chunk buffers without
//!   per-value copies; sliced stream windows share the same chunks. Logical
//!   conversions inside `Series::to_arrow` may materialise new buffers
//!   (documented per-column, not a universal zero-copy claim).
//! - String columns deliberately use `CompatLevel::oldest()` so Arrow fields
//!   and buffers map to `LargeUtf8` (`U` format), maximising
//!   independent-reader portability across bindings; primitive types are
//!   identical at every compatibility level. Revisit only with a reviewed
//!   Polars pin advance.
//!
//! # Schema negotiation
//!
//! Passing a `requested` field that does not deep-equal the canonical v2
//! field rejects explicitly (`UnsupportedSchemaNegotiation`) — there is no
//! silent schema-coercion fallback. DataFrames that do not match the
//! contract's column count, order, Arrow types or required-nullability
//! reject explicitly as well.
//!
//! # Multi-batch contract
//!
//! [`crate::arrow_c_data::export_stream`] and
//! [`crate::arrow_c_data::build_v2_stream_arrays`] window the table into
//! bounded record batches of at most `max_batch_rows` rows. Empty tables
//! produce a schema-correct zero-batch stream; batch export of an empty
//! table produces a length-0 struct array that still carries all 14 typed
//! children, so independent readers stay schema-valid.

use crate::{ArrowError, EVENT_LOG_FIELDS_V2, EVENT_LOG_STREAM_V2};
use polars::prelude::*;
pub use polars_arrow::array::StructArray;
pub use polars_arrow::array::{Array, PrimitiveArray, Utf8Array};
pub use polars_arrow::datatypes::{ArrowDataType, Field as ArrowField};
pub use polars_arrow::ffi::{
    export_array_to_c, export_field_to_c, export_iterator, ArrowArray, ArrowArrayStream,
    ArrowSchema,
};

/// Deliberate public re-export of the pinned polars-arrow C Data Interface
/// primitives, so downstream ABI surfaces (`kairo-ecs-ffi`, native consumers)
/// import through this crate's pinned version instead of guessing versions.
///
/// The import/parse side of the interface (`import_array_from_c`,
/// `import_field_from_c`, `ArrowArrayStreamReader`) requires `unsafe` blocks
/// and therefore does NOT live in this crate; ABI consumers use it from
/// `kairo-ecs-ffi`. This re-export only carries the safe export constructors
/// plus the ABI struct types downstream tests need for `unsafe`-allowing
/// crates.
pub mod arrow_ffi {
    pub use polars_arrow::ffi::{
        export_array_to_c, export_field_to_c, export_iterator, import_array_from_c,
        import_field_from_c, ArrowArray, ArrowArrayStream, ArrowArrayStreamReader, ArrowSchema,
    };
}

/// Compat level chosen for exported buffers/fields; see module docs.
const EXPORT_COMPAT_LEVEL: CompatLevel = CompatLevel::oldest();

/// The canonical, contract-driven Arrow field for the Schema v2 event log: a
/// non-nullable struct named `kairo_ecs.event_log.v2` whose 14 children carry
/// the exact names, Arrow types and nullability declared by
/// [`EVENT_LOG_FIELDS_V2`] (spec: "Required fields, units, nullability ...
/// are defined once").
pub fn event_log_v2_arrow_field() -> ArrowField {
    ArrowField {
        name: EVENT_LOG_STREAM_V2.into(),
        dtype: ArrowDataType::Struct(field_children()),
        is_nullable: false,
        metadata: None,
    }
}

/// The per-column Arrow children of the v2 event log, mapped through the
/// pinned Polars dtype→Arrow mapping so exported format strings always match
/// the converted buffers exactly.
fn field_children() -> Vec<ArrowField> {
    EVENT_LOG_FIELDS_V2
        .iter()
        .map(|field| ArrowField {
            name: field.name.into(),
            dtype: contract_arrow_dtype(field.data_type),
            is_nullable: field.nullable,
            metadata: None,
        })
        .collect()
}

/// Map a contract data-type string to its Arrow physical type via the pinned
/// Polars mapping. The contract strings are a closed static set; an unknown
/// string is a contract defect and fails loudly rather than mis-exporting a
/// column.
fn contract_arrow_dtype(contract: &str) -> ArrowDataType {
    let polars_dtype = match contract {
        "UInt16" => DataType::UInt16,
        "UInt64" => DataType::UInt64,
        "UInt32" => DataType::UInt32,
        "Int32" => DataType::Int32,
        "Utf8" => DataType::String,
        other => panic!("EVENT_LOG_FIELDS_V2 declares unknown data type: {other}"),
    };
    polars_dtype.to_arrow(EXPORT_COMPAT_LEVEL)
}

/// Validate that `df` is exactly the Schema v2 event-log table: same column
/// count, same names in contract order, same Arrow physical types, and no
/// nulls in contract-required columns. Unknown shapes, wrong order and
/// required-null violations reject explicitly.
pub fn validate_v2_dataframe(df: &DataFrame) -> Result<(), ArrowError> {
    let columns = df.get_columns();
    if columns.len() != EVENT_LOG_FIELDS_V2.len() {
        return Err(ArrowError::UnsupportedSchemaNegotiation(format!(
            "event log must carry {} columns, got {}",
            EVENT_LOG_FIELDS_V2.len(),
            columns.len()
        )));
    }
    for (index, (column, field)) in columns.iter().zip(EVENT_LOG_FIELDS_V2.iter()).enumerate() {
        let series = column.as_materialized_series();
        if series.name().as_str() != field.name {
            return Err(ArrowError::UnsupportedSchemaNegotiation(format!(
                "column at position {index} is {}, contract requires {}",
                series.name(),
                field.name
            )));
        }
        let actual = series.dtype().to_arrow(EXPORT_COMPAT_LEVEL);
        let expected = contract_arrow_dtype(field.data_type);
        if actual != expected {
            return Err(ArrowError::UnsupportedSchemaNegotiation(format!(
                "column {} has Arrow type {actual:?}, contract requires {expected:?}",
                field.name
            )));
        }
        if !field.nullable && series.null_count() != 0 {
            return Err(ArrowError::UnsupportedSchemaNegotiation(format!(
                "column {} is contract-required but carries {} nulls",
                field.name,
                series.null_count()
            )));
        }
    }
    Ok(())
}

fn ensure_requested_schema(
    requested: Option<&ArrowField>,
    contract: &ArrowField,
) -> Result<(), ArrowError> {
    match requested {
        None => Ok(()),
        Some(requested) => {
            if requested != contract {
                Err(ArrowError::UnsupportedSchemaNegotiation(format!(
                    "requested schema does not deep-equal the canonical v2 event log field ({EVENT_LOG_STREAM_V2})"
                )))
            } else {
                Ok(())
            }
        }
    }
}

/// Rechunk to a single chunk per column before Arrow conversion.
fn rechunked_clone(df: &DataFrame) -> DataFrame {
    let mut df = df.clone();
    df.rechunk_mut();
    df
}

/// Convert an already rechunked dataframe into per-column base chunk arrays.
fn base_child_arrays(rechunked: &DataFrame) -> Vec<Box<dyn polars_arrow::array::Array>> {
    rechunked
        .get_columns()
        .iter()
        .map(|column| {
            column
                .as_materialized_series()
                .to_arrow(0, EXPORT_COMPAT_LEVEL)
        })
        .collect()
}

/// Export the whole v2 event-log table as one populated owned C Data
/// interface pair (schema + struct array). Empty tables yield a length-0
/// array with all 14 typed children present, reader-schema valid.
///
/// The returned export's buffers are independent of the source dataframe
/// lifetime; the consumer releases exactly once. The returned structs are
/// ABI-compatible representations with the `ArrowArray`/`ArrowSchema` the C
/// consumers hold.
pub fn export_batch(
    df: &DataFrame,
    requested: Option<&ArrowField>,
) -> Result<(ArrowSchema, ArrowArray), ArrowError> {
    let contract = event_log_v2_arrow_field();
    validate_v2_dataframe(df)?;
    ensure_requested_schema(requested, &contract)?;

    let rechunked = rechunked_clone(df);
    Ok((
        export_field_to_c(&contract),
        export_array_to_c(Box::new(populated_struct_array(&rechunked, df.height()))),
    ))
}

/// Export the v2 event-log table as an owned C Stream split into bounded
/// record batches of at most `max_batch_rows` rows. `max_batch_rows` must be
/// greater than zero. Empty tables yield a schema-correct zero-batch stream.
pub fn export_stream(
    df: &DataFrame,
    max_batch_rows: usize,
    requested: Option<&ArrowField>,
) -> Result<ArrowArrayStream, ArrowError> {
    let arrays = build_v2_stream_arrays(df, max_batch_rows, requested)?;
    let contract = event_log_v2_arrow_field();

    let iter = arrays.into_iter().map(|array| {
        Ok::<Box<dyn polars_arrow::array::Array>, polars::error::PolarsError>(
            Box::new(array) as Box<dyn polars_arrow::array::Array>
        )
    });

    Ok(export_iterator(Box::new(iter), contract))
}

/// Build the populated struct arrays (`StructArray` per batch window) for an
/// already rechunked, validated dataframe.
fn populated_struct_arrays(rechunked: &DataFrame, max_batch_rows: usize) -> Vec<StructArray> {
    let child_bases = base_child_arrays(rechunked);
    let total_rows = rechunked.height();
    let dtype = event_log_v2_arrow_field().dtype.clone();

    (0..total_rows)
        .step_by(max_batch_rows)
        .map(|offset| {
            let len = usize::min(max_batch_rows, total_rows - offset);
            (offset, len)
        })
        .filter(|(_, len)| *len > 0)
        .map(move |(offset, len)| {
            let children: Vec<Box<dyn polars_arrow::array::Array>> = child_bases
                .iter()
                .map(|child| child.sliced(offset, len))
                .collect();
            StructArray::new(dtype.clone(), len, children, None)
        })
        .collect()
}

/// Build the populated struct array from an already rechunked, validated
/// dataframe: `len == height` for the full export, or a shorter prefix when
/// slicing a batch window.
fn populated_struct_array(rechunked: &DataFrame, len: usize) -> StructArray {
    let child_bases = base_child_arrays(rechunked);
    let children: Vec<Box<dyn polars_arrow::array::Array>> = child_bases
        .iter()
        .map(|child| child.sliced(0, len))
        .collect();
    StructArray::new(
        event_log_v2_arrow_field().dtype.clone(),
        len,
        children,
        None,
    )
}

/// Build the bounded batch struct arrays for the v2 event log without placing
/// them in the C ABI: `StructArray` per window of at most `max_batch_rows`
/// rows. Empty tables yield no arrays. Useful for consumers (and tests) that
/// want the Arrow buffers while keeping this crate the only owner of the
/// Polars conversion path.
pub fn build_v2_batch_arrays(
    df: &DataFrame,
    requested: Option<&ArrowField>,
) -> Result<StructArray, ArrowError> {
    let contract = event_log_v2_arrow_field();
    validate_v2_dataframe(df)?;
    ensure_requested_schema(requested, &contract)?;
    let full = df.height();
    let rechunked = rechunked_clone(df);
    Ok(populated_struct_array(&rechunked, full))
}

/// Build the bounded stream struct arrays without placing them in the C ABI,
/// mirroring [`crate::arrow_c_data::export_stream`]'s windowing exactly.
pub fn build_v2_stream_arrays(
    df: &DataFrame,
    max_batch_rows: usize,
    requested: Option<&ArrowField>,
) -> Result<Vec<StructArray>, ArrowError> {
    if max_batch_rows == 0 {
        return Err(ArrowError::InvalidArgument(
            "max_batch_rows must be greater than zero",
        ));
    }
    let contract = event_log_v2_arrow_field();
    validate_v2_dataframe(df)?;
    ensure_requested_schema(requested, &contract)?;
    let rechunked = rechunked_clone(df);
    Ok(populated_struct_arrays(&rechunked, max_batch_rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars_interop::records_to_dataframe;
    use crate::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
    use kairo_ecs_types::{EntityId, EventId};
    use polars_arrow::array::Utf8Array;

    fn sample_record(time_ticks: u128, entity: Option<EntityId>, sequence: u64) -> EventLogRecord {
        EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: "run-70-p2-cdata".to_string(),
            event_id: EventId::new(1, 1),
            entity_id: entity,
            time_ticks,
            time_scale: "ticks".to_string(),
            priority: -3,
            sequence,
            event_kind: "custom:5".to_string(),
            status: EventStatus::Dispatched,
            payload_ref: None,
        }
    }

    fn sample_df(records: &[EventLogRecord]) -> DataFrame {
        records_to_dataframe(records).expect("sample v2 records must convert to a DataFrame")
    }

    fn child_position(name: &str) -> usize {
        EVENT_LOG_FIELDS_V2
            .iter()
            .position(|f| f.name == name)
            .unwrap_or_else(|| panic!("contract must declare {name}"))
    }

    fn read_u64(
        child: &dyn polars_arrow::array::Array,
        name: &str,
    ) -> polars_arrow::array::PrimitiveArray<u64> {
        child
            .as_any()
            .downcast_ref::<polars_arrow::array::PrimitiveArray<u64>>()
            .unwrap_or_else(|| panic!("{name} must be a UInt64 array"))
            .clone()
    }

    fn read_u32(
        child: &dyn polars_arrow::array::Array,
        name: &str,
    ) -> polars_arrow::array::PrimitiveArray<u32> {
        child
            .as_any()
            .downcast_ref::<polars_arrow::array::PrimitiveArray<u32>>()
            .unwrap_or_else(|| panic!("{name} must be a UInt32 array"))
            .clone()
    }

    fn assert_batch_values(imported: &StructArray) {
        let children = imported.values();
        assert_eq!(children.len(), EVENT_LOG_FIELDS_V2.len());

        // Null-together generational entities: row 1 has no entity, rows 0/2 do.
        let entity_index = read_u64(
            children[child_position("entity_id_index")].as_ref(),
            "entity_id_index",
        );
        let entity_gen = read_u32(
            children[child_position("entity_id_gen")].as_ref(),
            "entity_id_gen",
        );
        assert_eq!(entity_index.null_count(), 1);
        assert_eq!(entity_gen.null_count(), 1);
        assert_eq!(entity_index.value(0), 7);
        assert_eq!(entity_gen.value(0), 2);
        assert_eq!(entity_index.value(2), 9);
        assert_eq!(entity_gen.value(2), 3);

        // Full-range tick limbs: row 2 is 2^64 + 5, row 0 is 100.
        let ticks_hi = read_u64(
            children[child_position("time_ticks_hi")].as_ref(),
            "time_ticks_hi",
        );
        let ticks_lo = read_u64(
            children[child_position("time_ticks_lo")].as_ref(),
            "time_ticks_lo",
        );
        assert_eq!((ticks_hi.value(0), ticks_lo.value(0)), (0, 100));
        assert_eq!((ticks_hi.value(2), ticks_lo.value(2)), (1, 5));

        // Populated run ids survive the conversion.
        let run_ids = children[child_position("run_id")]
            .as_any()
            .downcast_ref::<Utf8Array<i64>>()
            .expect("run_id must export as LargeUtf8");
        assert_eq!(run_ids.value(0), "run-70-p2-cdata");
        assert_eq!(run_ids.value(2), "run-70-p2-cdata");
    }

    #[test]
    fn contract_field_is_v2_struct_with_typed_children() {
        let field = event_log_v2_arrow_field();
        assert_eq!(field.name.as_str(), EVENT_LOG_STREAM_V2);
        assert!(!field.is_nullable);
        let children = match &field.dtype {
            ArrowDataType::Struct(children) => children,
            other => panic!("contract field must be a struct, got {other:?}"),
        };
        assert_eq!(children.len(), EVENT_LOG_FIELDS_V2.len());
        for (child, contract_field) in children.iter().zip(EVENT_LOG_FIELDS_V2.iter()) {
            assert_eq!(child.name.as_str(), contract_field.name);
            assert_eq!(
                child.is_nullable, contract_field.nullable,
                "nullability of {} must follow the v2 contract",
                contract_field.name
            );
        }
    }

    #[test]
    fn batch_arrays_are_populated_with_null_together_limbs() {
        let records = vec![
            sample_record(100, Some(EntityId::new(7, 2)), 1),
            sample_record(200, None, 2),
            sample_record((1u128 << 64) + 5, Some(EntityId::new(9, 3)), 3),
        ];
        let df = sample_df(&records);

        let array = build_v2_batch_arrays(&df, None).expect("populated batch build must succeed");
        assert_eq!(array.len(), 3);
        assert_batch_values(&array);
    }

    #[test]
    fn batch_array_lives_independently_of_source_dataframe() {
        let array = {
            let df = sample_df(&[
                sample_record(100, Some(EntityId::new(7, 2)), 1),
                sample_record(200, None, 2),
            ]);
            build_v2_batch_arrays(&df, None).expect("export must succeed")
            // df dropped: the struct array owns converted buffers.
        };
        assert_eq!(array.len(), 2);
        let ticks_lo = read_u64(
            array.values()[child_position("time_ticks_lo")].as_ref(),
            "time_ticks_lo",
        );
        assert_eq!(ticks_lo.value(0), 100);
        assert_eq!(ticks_lo.value(1), 200);
    }

    #[test]
    fn empty_output_stays_schema_valid() {
        let df = sample_df(&[]);

        let batch = build_v2_batch_arrays(&df, None).expect("empty build must succeed");
        assert_eq!(batch.len(), 0);
        assert_eq!(batch.values().len(), EVENT_LOG_FIELDS_V2.len());
        assert!(batch.values().iter().all(|child| child.is_empty()));

        let arrays = build_v2_stream_arrays(&df, 4, None).expect("empty stream build");
        assert!(arrays.is_empty(), "empty stream: zero batches");
    }

    #[test]
    fn stream_arrays_window_into_bounded_populated_batches() {
        let records: Vec<EventLogRecord> = (1..=3u128)
            .map(|i| {
                sample_record(
                    i * 100,
                    if i == 2 {
                        None
                    } else {
                        Some(EntityId::new(i as u64, 1))
                    },
                    i as u64,
                )
            })
            .collect();
        let df = sample_df(&records);

        let arrays = build_v2_stream_arrays(&df, 2, None).expect("stream build must succeed");
        assert_eq!(
            arrays.iter().map(|array| array.len()).collect::<Vec<_>>(),
            vec![2, 1],
            "3 rows at max_batch_rows=2"
        );
        for (batch_index, array) in arrays.iter().enumerate() {
            assert_eq!(array.values().len(), EVENT_LOG_FIELDS_V2.len());
            let sequence = read_u64(
                array.values()[child_position("sequence")].as_ref(),
                "sequence",
            );
            // Row order must be preserved across batch boundaries.
            let loaded: usize = arrays[..batch_index].iter().map(|a| a.len()).sum();
            for row in 0..array.len() {
                // Sample records use 1-based sequence values; row order must
                // be preserved across batch boundaries.
                assert_eq!(sequence.value(row), (loaded + row + 1) as u64);
            }
        }
    }

    #[test]
    fn export_rejects_bad_arguments_and_unsupported_negotiation() {
        let df = sample_df(&[sample_record(100, None, 1)]);

        assert!(matches!(
            build_v2_stream_arrays(&df, 0, None),
            Err(ArrowError::InvalidArgument(
                "max_batch_rows must be greater than zero"
            ))
        ));

        let mut mismatched = event_log_v2_arrow_field();
        if let ArrowDataType::Struct(children) = &mut mismatched.dtype {
            children[0].name = "schema_version_renamed".into();
        }
        assert!(matches!(
            build_v2_batch_arrays(&df, Some(&mismatched)),
            Err(ArrowError::UnsupportedSchemaNegotiation(_))
        ));

        build_v2_batch_arrays(&df, Some(&event_log_v2_arrow_field()))
            .expect("matching requested schema must be accepted");
    }

    #[test]
    fn validation_rejects_non_v2_dataframes_explicitly() {
        let df = sample_df(&[sample_record(100, None, 1)]);
        let truncated = DataFrame::new(df.get_columns().iter().take(3).cloned().collect())
            .expect("three v2 columns form a valid (non-contract) frame");
        assert!(matches!(
            validate_v2_dataframe(&truncated),
            Err(ArrowError::UnsupportedSchemaNegotiation(_))
        ));
    }
}
