//! Optional RecordBatch transport; semantic mappings stay outside this crate.
#![forbid(unsafe_code)]

mod error;
mod limits;

pub use error::IoError;
pub use limits::IoLimits;

#[cfg(any(feature = "ipc", feature = "parquet"))]
pub use arrow_array::RecordBatch;
#[cfg(any(feature = "ipc", feature = "parquet"))]
pub use arrow_schema::{Schema, SchemaRef};

#[cfg(feature = "ipc")]
mod ipc;
#[cfg(feature = "ipc")]
pub use ipc::{read_ipc_file, read_ipc_stream, write_ipc_file, write_ipc_stream};

#[cfg(feature = "parquet")]
mod parquet;
#[cfg(feature = "parquet")]
pub use parquet::{read_parquet, write_parquet};
