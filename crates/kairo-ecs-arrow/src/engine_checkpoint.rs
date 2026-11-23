//! Native v2 checkpoints containing an executable scheduler and caller-owned model state.

use std::path::Path;

use kairo_ecs_core::Scheduler;

use crate::{ArrowError, CheckpointAdapterReceipt, CheckpointFormat, CheckpointManifest};

/// A verified native checkpoint at a single model transition boundary.
#[derive(Debug)]
pub struct RestoredEngineCheckpoint {
    pub manifest: CheckpointManifest,
    pub scheduler: Scheduler,
    pub model_state: Vec<u8>,
}

/// Publish the scheduler, model state and event records in one HDF5 or BP5 checkpoint.
///
/// Capture all inputs at the same event boundary. The caller owns the model schema
/// and includes its component, resource, orchestration and RNG state as needed.
/// Use a versioned `model_schema` to select the matching model decoder.
///
/// # Errors
///
/// Returns an error for an invalid schema, scheduler serialization failure,
/// unsupported backend, invalid event records or failed publication.
pub fn write_engine_checkpoint(
    path: impl AsRef<Path>,
    manifest: &CheckpointManifest,
    scheduler: &Scheduler,
    model_schema: &str,
    model_state: &[u8],
) -> Result<CheckpointAdapterReceipt, ArrowError> {
    validate_model_schema(model_schema)?;
    let scheduler = scheduler
        .checkpoint_json()
        .map_err(|_| ArrowError::InvalidArgument("scheduler cannot be checkpointed"))?;
    let mut state = Vec::new();
    state.extend_from_slice(&(model_schema.len() as u64).to_le_bytes());
    state.extend_from_slice(&(scheduler.len() as u64).to_le_bytes());
    state.extend_from_slice(model_schema.as_bytes());
    state.extend_from_slice(&scheduler);
    state.extend_from_slice(model_state);

    let path = path.as_ref();
    write_native(path, manifest, &state)?;
    Ok(CheckpointAdapterReceipt {
        format: manifest.format,
        path: path.display().to_string(),
        row_count: manifest.row_count,
        checksum: manifest.checksum,
        contract_only: false,
    })
}

/// Open a native v2 checkpoint, validate its complete digest and restore a new scheduler.
///
/// The model schema must match the caller's decoder exactly. Decode the returned
/// model bytes with that decoder before resuming dispatch. The digest detects
/// corruption and cannot authenticate the producer.
///
/// # Errors
///
/// Returns an error for an unavailable backend, incompatible schema, damaged
/// artifact or invalid scheduler state. A v1 telemetry checkpoint has no engine state.
pub fn restore_engine_checkpoint(
    path: impl AsRef<Path>,
    format: CheckpointFormat,
    model_schema: &str,
) -> Result<RestoredEngineCheckpoint, ArrowError> {
    validate_model_schema(model_schema)?;
    let (manifest, state) = read_native(path.as_ref(), format)?;
    manifest.validate_for_restore()?;
    let state = state.ok_or(ArrowError::UnexpectedStreamHeader)?;
    let invalid_state = ArrowError::InvalidArgument("invalid engine state byte lengths");
    let (header, bytes) = state
        .split_at_checked(16)
        .ok_or_else(|| invalid_state.clone())?;
    let length = |part: &[u8]| {
        usize::try_from(u64::from_le_bytes(
            part.try_into().expect("eight-byte length"),
        ))
        .map_err(|_| invalid_state.clone())
    };
    let (stored_schema, bytes) = bytes
        .split_at_checked(length(&header[..8])?)
        .ok_or_else(|| invalid_state.clone())?;
    if stored_schema != model_schema.as_bytes() {
        return Err(ArrowError::InvalidArgument(
            "engine model schema does not match",
        ));
    }
    let (scheduler, model) = bytes
        .split_at_checked(length(&header[8..])?)
        .ok_or(invalid_state)?;
    let scheduler = Scheduler::from_checkpoint_json(scheduler)
        .map_err(|_| ArrowError::InvalidArgument("invalid stored scheduler state"))?;
    Ok(RestoredEngineCheckpoint {
        manifest,
        scheduler,
        model_state: model.to_vec(),
    })
}

fn validate_model_schema(value: &str) -> Result<(), ArrowError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(ArrowError::InvalidArgument("invalid engine model schema"));
    }
    Ok(())
}

fn write_native(
    path: &Path,
    manifest: &CheckpointManifest,
    state: &[u8],
) -> Result<(), ArrowError> {
    match manifest.format {
        #[cfg(feature = "hdf5")]
        CheckpointFormat::Hdf5 => crate::hdf5_checkpoint::write(path, manifest, Some(state)),
        #[cfg(all(feature = "adios2", any(target_os = "linux", target_os = "macos")))]
        CheckpointFormat::Adios2 => crate::adios2_checkpoint::write(path, manifest, Some(state)),
        format => {
            let _ = (path, state);
            Err(ArrowError::AdapterDisabled(format.as_str()))
        }
    }
}

fn read_native(
    path: &Path,
    format: CheckpointFormat,
) -> Result<(CheckpointManifest, Option<Vec<u8>>), ArrowError> {
    match format {
        #[cfg(feature = "hdf5")]
        CheckpointFormat::Hdf5 => crate::hdf5_checkpoint::read(path, true),
        #[cfg(all(feature = "adios2", any(target_os = "linux", target_os = "macos")))]
        CheckpointFormat::Adios2 => crate::adios2_checkpoint::read(path, true),
        format => {
            let _ = path;
            Err(ArrowError::AdapterDisabled(format.as_str()))
        }
    }
}
