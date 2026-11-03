//! Safe consumers for original trusted owned domain exports.
//!
//! This existing FFI crate is the unsafe import boundary. Arbitrary raw schemas,
//! arrays or streams cannot enter these safe APIs: only the producer's private,
//! unforgeable, ownership-carrying wrappers are accepted. Foreign imports retain
//! their ordinary unsafe obligations outside this API.
use kairo_ecs_arrow::arrow_c_data::{
    arrow_ffi as abi,
    domain::{OwnedBatch, OwnedStream},
    ArrowField, StructArray,
};
use kairo_ecs_arrow::ArrowError;
fn invalid(error: impl std::fmt::Display) -> ArrowError {
    ArrowError::UnsupportedSchemaNegotiation(error.to_string())
}
fn owned_struct(
    array: Box<dyn kairo_ecs_arrow::arrow_c_data::Array>,
) -> Result<StructArray, ArrowError> {
    array
        .as_any()
        .downcast_ref::<StructArray>()
        .cloned()
        .ok_or_else(|| invalid("trusted domain export was not a StructArray"))
}
/// Consume the original producer's owned batch through the real C Data import
/// path. Returned buffers own the imported callback lifetime; source and holder
/// destruction do not invalidate them. Schema metadata is returned unchanged.
pub fn import_batch(export: OwnedBatch) -> Result<(ArrowField, StructArray), ArrowError> {
    let (schema, array) = export.into_raw();
    // SAFETY: OwnedBatch has private fields and no raw constructor or mutable
    // access; only the validated safe Arrow producer can create this value.
    let field = unsafe { abi::import_field_from_c(&schema) }.map_err(invalid)?;
    // SAFETY: The original array and parsed field originate from the same
    // trusted export, have not been mutated and transfer ownership exactly once.
    let array = unsafe { abi::import_array_from_c(array, field.dtype.clone()) }.map_err(invalid)?;
    Ok((field, owned_struct(array)?))
}
/// Iterator owning a trusted native C Stream. Dropping it early releases the
/// stream; already yielded owned arrays retain their own buffer lifetimes.
pub struct TableStreamReader {
    reader: abi::ArrowArrayStreamReader<Box<abi::ArrowArrayStream>>,
    finished: bool,
}
impl TableStreamReader {
    /// The declared negotiated schema, including metadata and requiredness.
    pub fn schema(&self) -> &ArrowField {
        self.reader.field()
    }
}
impl Iterator for TableStreamReader {
    type Item = Result<StructArray, ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        // SAFETY: This reader is constructible only from OwnedStream. Its
        // private original producer iterator emits schema-validated arrays.
        let next = unsafe { self.reader.next() };
        match next {
            Some(Ok(array)) => {
                let result = owned_struct(array);
                if result.is_err() {
                    self.finished = true;
                }
                Some(result)
            }
            Some(Err(error)) => {
                self.finished = true;
                Some(Err(invalid(error)))
            }
            None => {
                self.finished = true;
                None
            }
        }
    }
}
/// Consume the original producer's stream without collecting its batches.
pub fn import_stream(export: OwnedStream) -> Result<TableStreamReader, ArrowError> {
    // SAFETY: OwnedStream cannot be forged from raw pointers through safe Rust;
    // into_raw consumes its sole trusted holder. Reader owns its release path.
    let reader = unsafe { abi::ArrowArrayStreamReader::try_new(Box::new(export.into_raw())) }
        .map_err(invalid)?;
    Ok(TableStreamReader {
        reader,
        finished: false,
    })
}
