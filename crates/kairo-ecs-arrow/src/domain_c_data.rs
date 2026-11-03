//! Declared primitive domain tables over the pinned Arrow C Data interface.
//!
//! Domain owners supply versioned schema, units and identity/limb semantics.
//! This module validates physical shape and requiredness, never domain meaning.
//! Primitive buffers can share storage; String conversion/rechunking may copy.
//! Whole-frame batch export can allocate; streaming windows are lazy but retain
//! the rechunked source buffers. No universal zero-copy or RSS bound is claimed.
use super::{base_child_arrays, rechunked_clone, EXPORT_COMPAT_LEVEL};
use super::{export_array_to_c, export_field_to_c, export_iterator};
use super::{ArrowArray, ArrowArrayStream, ArrowDataType, ArrowField, ArrowSchema, StructArray};
use crate::ArrowError;
pub use polars::prelude::DataFrame;
use std::collections::HashSet;

/// A validated owned batch. Private holders cannot be forged or mutated through
/// safe Rust. No Clone or from_raw constructor duplicates release obligations.
pub struct OwnedBatch {
    schema: ArrowSchema,
    array: ArrowArray,
}
impl OwnedBatch {
    /// Transfer both standard C Data holders to a foreign consumer. The consumer
    /// must obey Arrow's unsafe pointer, lifetime and single-release contract.
    /// Dropping the returned Rust holders without transfer releases them normally.
    pub fn into_raw(self) -> (ArrowSchema, ArrowArray) {
        (self.schema, self.array)
    }
}
/// An owned trusted stream created only by the validated producer below.
pub struct OwnedStream {
    stream: ArrowArrayStream,
}
impl OwnedStream {
    /// Consume this trusted wrapper and transfer the standard C Stream holder.
    /// Foreign consumers assume the standard unsafe Arrow consumer obligations.
    pub fn into_raw(self) -> ArrowArrayStream {
        self.stream
    }
}
fn unsupported(message: impl Into<String>) -> ArrowError {
    ArrowError::UnsupportedSchemaNegotiation(message.into())
}
fn supported(dtype: &ArrowDataType) -> bool {
    matches!(
        dtype,
        ArrowDataType::Boolean
            | ArrowDataType::UInt8
            | ArrowDataType::UInt16
            | ArrowDataType::UInt32
            | ArrowDataType::UInt64
            | ArrowDataType::Int8
            | ArrowDataType::Int16
            | ArrowDataType::Int32
            | ArrowDataType::Int64
            | ArrowDataType::Float32
            | ArrowDataType::Float64
            | ArrowDataType::LargeUtf8
    )
}
fn validate(
    df: &DataFrame,
    contract: &ArrowField,
    requested: Option<&ArrowField>,
) -> Result<(), ArrowError> {
    if contract.name.is_empty() || contract.name.as_str().contains('\0') || contract.is_nullable {
        return Err(unsupported(
            "domain root must be named, nonnullable and NUL-free",
        ));
    }
    if requested.is_some_and(|requested| requested != contract) {
        return Err(unsupported(
            "requested domain schema differs from declared contract",
        ));
    }
    let ArrowDataType::Struct(fields) = &contract.dtype else {
        return Err(unsupported("domain contract must be a Struct"));
    };
    if fields.is_empty() || fields.len() != df.width() || df.height() > u32::MAX as usize {
        return Err(unsupported("domain shape or row-index capacity"));
    }
    let mut names = HashSet::new();
    for (field, column) in fields.iter().zip(df.columns()) {
        if field.name.is_empty()
            || field.name.as_str().contains('\0')
            || !names.insert(field.name.clone())
            || column.name() != &field.name
            || !supported(&field.dtype)
            || column.dtype().to_arrow(EXPORT_COMPAT_LEVEL) != field.dtype
            || (!field.is_nullable && column.null_count() != 0)
        {
            return Err(unsupported(format!(
                "unsupported or mismatched domain field {}",
                field.name
            )));
        }
    }
    Ok(())
}
/// Export a native domain frame as a populated owned struct array. Negotiation
/// is exact, including field order, nullability and declared metadata. Unsupported
/// physical types reject without coercion. Empty typed frames remain typed.
pub fn export_batch(
    df: &DataFrame,
    contract: &ArrowField,
    requested: Option<&ArrowField>,
) -> Result<OwnedBatch, ArrowError> {
    validate(df, contract, requested)?;
    let frame = rechunked_clone(df);
    let array = StructArray::try_new(
        contract.dtype.clone(),
        frame.height(),
        base_child_arrays(&frame),
        None,
    )
    .map_err(|error| unsupported(error.to_string()))?;
    // All fallible validation/construction precedes ABI-holder creation. There
    // is no fallible partial-holder path or manual release shim here.
    Ok(OwnedBatch {
        schema: export_field_to_c(contract),
        array: export_array_to_c(Box::new(array)),
    })
}
/// Export bounded windows on demand. The stream owns its buffers after source
/// mutation/drop; no vector of all batch arrays is collected. Empty frames yield
/// a schema-correct zero-batch stream. max_batch_rows must be positive.
pub fn export_stream(
    df: &DataFrame,
    contract: &ArrowField,
    max_batch_rows: usize,
    requested: Option<&ArrowField>,
) -> Result<OwnedStream, ArrowError> {
    validate(df, contract, requested)?;
    if max_batch_rows == 0 {
        return Err(ArrowError::InvalidArgument(
            "max_batch_rows must be positive",
        ));
    }
    let frame = rechunked_clone(df);
    let total = frame.height();
    let bases = base_child_arrays(&frame);
    let dtype = contract.dtype.clone();
    let windows = (0..total).step_by(max_batch_rows).map(move |offset| {
        let len = max_batch_rows.min(total - offset);
        let children = bases
            .iter()
            .map(|child| child.sliced(offset, len))
            .collect();
        StructArray::try_new(dtype.clone(), len, children, None)
            .map(|array| Box::new(array) as Box<dyn super::Array>)
    });
    Ok(OwnedStream {
        stream: export_iterator(Box::new(windows), contract.clone()),
    })
}
