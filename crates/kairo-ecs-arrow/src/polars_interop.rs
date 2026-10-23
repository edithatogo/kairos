//! Track 70 Phase 1: Polars-native tabular processing and standard interchange.
//!
//! Provides conversion between KairoECS `EventLogRecord` / `EventLogBatch` and
//! upstream Polars `DataFrame`, alongside true Parquet/Zstd and Arrow IPC writers and readers.

use crate::{ArrowError, EventLogRecord, SCHEMA_VERSION_V2};
use polars::prelude::*;
use std::io::{Read, Seek, Write};

/// Convert a slice of [`EventLogRecord`]s into an official Polars [`DataFrame`] adhering to Schema v2.
///
/// In Task 1.1, this is a behavioral characterization stub that panics pending Task 1.2 implementation.
pub fn records_to_dataframe(records: &[EventLogRecord]) -> Result<DataFrame, ArrowError> {
    if records
        .iter()
        .any(|r| r.schema_version != SCHEMA_VERSION_V2)
    {
        return Err(ArrowError::UnsupportedSchemaVersion(
            records.first().map(|r| r.schema_version).unwrap_or(0),
        ));
    }
    panic!("Track 70 Phase 1: records_to_dataframe not yet implemented");
}

/// Write a [`DataFrame`] to Parquet with Zstandard compression.
///
/// In Task 1.1, this is a behavioral characterization stub that panics pending Task 1.2 implementation.
pub fn write_parquet_zstd<W: Write>(_df: &mut DataFrame, _writer: W) -> Result<(), ArrowError> {
    panic!("Track 70 Phase 1: write_parquet_zstd not yet implemented");
}

/// Read a [`DataFrame`] from a Parquet reader.
///
/// In Task 1.1, this is a behavioral characterization stub that panics pending Task 1.2 implementation.
pub fn read_parquet<R: Read + Seek>(_reader: R) -> Result<DataFrame, ArrowError> {
    panic!("Track 70 Phase 1: read_parquet not yet implemented");
}

/// Write a [`DataFrame`] to an Arrow IPC stream.
///
/// In Task 1.1, this is a behavioral characterization stub that panics pending Task 1.2 implementation.
pub fn write_ipc_stream<W: Write>(_df: &mut DataFrame, _writer: W) -> Result<(), ArrowError> {
    panic!("Track 70 Phase 1: write_ipc_stream not yet implemented");
}

/// Read a [`DataFrame`] from an Arrow IPC stream reader.
///
/// In Task 1.1, this is a behavioral characterization stub that panics pending Task 1.2 implementation.
pub fn read_ipc_stream<R: Read + Seek>(_reader: R) -> Result<DataFrame, ArrowError> {
    panic!("Track 70 Phase 1: read_ipc_stream not yet implemented");
}
