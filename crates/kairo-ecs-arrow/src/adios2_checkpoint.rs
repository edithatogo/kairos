use super::{
    checkpoint_data::{decode_records, digest, encode_records, BlockRow, EventRow},
    publication_parent, ArrowError, CheckpointFormat, CheckpointManifest, ContiguousBlock,
};
use kairo_ecs_adios2::{Reader, Session, Values};
use rustix::fs::{renameat_with, RenameFlags};
use std::{
    ffi::CStr,
    fs::{self, File, Metadata, Permissions},
    io::{self, Read},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

const SCHEMA: &[u8] = b"kairo-ecs-adios2-bp5-checkpoint-v1";
const ENGINE_SCHEMA: &[u8] = b"kairo-ecs-adios2-bp5-checkpoint-v2";
const FILES: [&str; 4] = ["data.0", "md.0", "md.idx", "mmd.0"];

macro_rules! table {
    ($encode:ident, $decode:ident, $row:ident; $($field:ident: $ty:ty => $name:literal),+ $(,)?) => {
        fn $encode(rows: &[$row], columns: &mut Vec<(&'static CStr, Values)>) {
            $(columns.push(($name, rows.iter().map(|row| row.$field).collect::<Vec<$ty>>().into()));)+
        }

        fn $decode(reader: &Reader<'_>, count: usize) -> Result<Vec<$row>, ArrowError> {
            $(let $field = reader.array::<$ty>($name, count)?;)+
            Ok((0..count).map(|row| $row { $($field: $field[row]),+ }).collect())
        }
    };
}

table!(write_events, read_events, EventRow;
    event_id_index: u64 => c"event_id_index",
    entity_id_index: u64 => c"entity_id_index",
    time_ticks_hi: u64 => c"time_ticks_hi",
    time_ticks_lo: u64 => c"time_ticks_lo",
    sequence: u64 => c"sequence",
    kind_bytes: u64 => c"kind_bytes",
    payload_bytes: u64 => c"payload_bytes",
    event_id_generation: u32 => c"event_id_generation",
    entity_id_generation: u32 => c"entity_id_generation",
    priority: i32 => c"priority",
    entity_present: u8 => c"entity_present",
    payload_present: u8 => c"payload_present",
    status: u8 => c"status",
);
table!(write_blocks, read_blocks, BlockRow;
    offset_bytes: u64 => c"block_offset_bytes",
    byte_len: u64 => c"block_byte_len",
    row_count: u64 => c"block_row_count",
);

pub(super) fn write(
    path: &Path,
    manifest: &CheckpointManifest,
    engine: Option<&[u8]>,
) -> Result<(), ArrowError> {
    let session = Session::acquire()?;
    manifest.validate_for_restore()?;
    let normalized = checked_path(path)?;
    let path = normalized.as_path();
    let existing = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            read_bp5(&session, path, engine.is_some())?;
            Some(metadata)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    let parent = publication_parent(path);
    let parent_file = File::open(parent)?;
    let staging = create_staging_directory(parent)?;
    let staged_path = staging.path().to_path_buf();
    let (rows, text) = encode_records(&manifest.records);
    let blocks: Vec<_> = manifest.blocks.iter().map(BlockRow::from).collect();
    let schema = if engine.is_some() {
        ENGINE_SCHEMA
    } else {
        SCHEMA
    };
    let mut counts = vec![
        rows.len() as u64,
        blocks.len() as u64,
        text.len() as u64,
        manifest.checkpoint_id.len() as u64,
        manifest.run_id.len() as u64,
        manifest.schema_fingerprint.len() as u64,
    ];
    if let Some(state) = engine {
        counts.push(state.len() as u64);
    }
    let mut columns = vec![
        (c"schema", schema.to_vec().into()),
        (
            c"checkpoint_id",
            manifest.checkpoint_id.as_bytes().to_vec().into(),
        ),
        (c"run_id", manifest.run_id.as_bytes().to_vec().into()),
        (
            c"schema_fingerprint",
            manifest.schema_fingerprint.as_bytes().to_vec().into(),
        ),
        (c"counts", counts.into()),
        (c"text", text.into()),
        (c"legacy_checksum", vec![manifest.checksum].into()),
        (c"sha256", digest(schema, manifest, engine).to_vec().into()),
    ];
    if let Some(state) = engine {
        columns.push((c"engine_state", state.to_vec().into()));
    }
    write_events(&rows, &mut columns);
    write_blocks(&blocks, &mut columns);
    session.write(&staged_path, &columns)?;
    let (stored_manifest, stored_engine) = read_bp5(&session, &staged_path, engine.is_some())?;
    if stored_manifest != *manifest || stored_engine.as_deref() != engine {
        return Err(ArrowError::InvalidArgument(
            "staged BP5 checkpoint differs from input",
        ));
    }

    let staged_directory = File::open(&staged_path)?;
    for entry in fs::read_dir(&staged_path)? {
        let entry = entry?;
        let staged_file = File::open(entry.path())?;
        let previous = match existing {
            Some(_) => match fs::symlink_metadata(path.join(entry.file_name())) {
                Ok(metadata) if metadata.is_file() => Some(metadata),
                Ok(_) => {
                    return Err(ArrowError::InvalidArgument(
                        "BP5 member changed while staging",
                    ))
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            },
            None => None,
        };
        sync_with_mode(&staged_file, previous.as_ref(), 0o600)?;
    }
    if let Some(expected) = &existing {
        let actual = fs::symlink_metadata(path)?;
        if (actual.dev(), actual.ino()) != (expected.dev(), expected.ino()) {
            return Err(ArrowError::InvalidArgument(
                "BP5 target changed while staging",
            ));
        }
    }

    let name = path
        .file_name()
        .ok_or(ArrowError::InvalidArgument("BP5 target has no filename"))?;
    let staged_name = staged_path.file_name().ok_or(ArrowError::InvalidArgument(
        "BP5 staging path has no filename",
    ))?;
    let replace = existing.is_some();
    let published = sync_with_mode(&staged_directory, existing.as_ref(), 0o700).and_then(|()| {
        renameat_with(
            &parent_file,
            staged_name,
            &parent_file,
            name,
            if replace {
                RenameFlags::EXCHANGE
            } else {
                RenameFlags::NOREPLACE
            },
        )
        .map_err(io::Error::from)?;
        Ok(())
    });
    if let Err(error) = published {
        if let Err(cleanup) = staged_directory
            .set_permissions(Permissions::from_mode(0o700))
            .and_then(|()| staging.close())
        {
            return Err(ArrowError::Io(format!(
                "BP5 publication failed: {error}. Cleanup of {} failed: {cleanup}",
                staged_path.display()
            )));
        }
        return Err(error);
    }

    if let Err(error) = parent_file.sync_all() {
        let retained = staging.keep();
        return Err(ArrowError::Io(format!(
            "BP5 publication sync failed: {error}. Check destination {} and staging path {}",
            path.display(),
            retained.display()
        )));
    }
    if !replace {
        let _ = staging.keep();
        return Ok(());
    }
    staging.close().map_err(|error| {
        ArrowError::Io(format!(
            "BP5 checkpoint was published but cleanup of {} failed: {error}",
            staged_path.display()
        ))
    })
}

fn create_staging_directory(parent: &Path) -> io::Result<tempfile::TempDir> {
    tempfile::Builder::new()
        .permissions(Permissions::from_mode(0o700))
        .tempdir_in(parent)
}

pub(super) fn read(
    path: &Path,
    require_engine: bool,
) -> Result<(CheckpointManifest, Option<Vec<u8>>), ArrowError> {
    read_bp5(&Session::acquire()?, path, require_engine)
}

fn read_bp5(
    session: &Session,
    path: &Path,
    require_engine: bool,
) -> Result<(CheckpointManifest, Option<Vec<u8>>), ArrowError> {
    let normalized = checked_path(path)?;
    let path = normalized.as_path();
    session.read(path, validate_directory(path)?, |reader| {
        let schema = if require_engine {
            ENGINE_SCHEMA
        } else {
            SCHEMA
        };
        if reader.array::<u8>(c"schema", schema.len())? != schema {
            return Err(ArrowError::UnexpectedStreamHeader);
        }
        let counts = reader.array::<u64>(c"counts", if require_engine { 7 } else { 6 })?;
        let count = |index: usize| {
            usize::try_from(counts[index]).map_err(|_| ArrowError::InvalidNumber("BP5 count"))
        };
        let rows = read_events(reader, count(0)?)?;
        let text = reader.array::<u8>(c"text", count(2)?)?;
        let records = decode_records(rows, &text)?;
        let blocks = read_blocks(reader, count(1)?)?
            .into_iter()
            .map(ContiguousBlock::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let string = |name, index| {
            String::from_utf8(reader.array::<u8>(name, count(index)?)?)
                .map_err(|_| ArrowError::InvalidUtf8)
        };
        let manifest = CheckpointManifest {
            checkpoint_id: string(c"checkpoint_id", 3)?,
            format: CheckpointFormat::Adios2,
            run_id: string(c"run_id", 4)?,
            schema_fingerprint: string(c"schema_fingerprint", 5)?,
            row_count: records.len(),
            blocks,
            records,
            checksum: reader.array::<u64>(c"legacy_checksum", 1)?[0],
        };
        let engine = require_engine
            .then(|| {
                reader
                    .array::<u8>(c"engine_state", count(6)?)
                    .map_err(ArrowError::from)
            })
            .transpose()?;
        if reader.array::<u8>(c"sha256", 32)? != digest(schema, &manifest, engine.as_deref()) {
            return Err(ArrowError::InvalidNumber("checkpoint sha256"));
        }
        manifest.validate_for_restore()?;
        Ok((manifest, engine))
    })
}

fn checked_path(path: &Path) -> Result<PathBuf, ArrowError> {
    let path: PathBuf = path.components().collect();
    if path.file_name().is_none() || path.to_str().is_none_or(|name| name.contains('\0')) {
        return Err(ArrowError::InvalidArgument(
            "ADIOS2 checkpoint paths require UTF-8 without NUL",
        ));
    }
    Ok(path)
}

fn validate_directory(path: &Path) -> Result<u64, ArrowError> {
    let invalid = |actual: &'static str| ArrowError::InvalidCheckpointFormat {
        expected: "native ADIOS2 BP5 directory",
        actual: actual.into(),
    };
    if !fs::symlink_metadata(path)?.is_dir() {
        return Err(invalid("checkpoint target is not a directory"));
    }
    let mut members = [false; FILES.len()];
    let mut data_bytes = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.is_file() {
            return Err(invalid("BP5 members must be direct regular files"));
        }
        let name = entry.file_name();
        if let Some(index) = FILES.iter().position(|file| name == *file) {
            members[index] = true;
            if index == 0 {
                data_bytes = metadata.len();
            }
        } else if name != "profiling.json" {
            return Err(invalid("unexpected file in serial BP5 directory"));
        }
    }
    if members.contains(&false) {
        return Err(invalid("serial BP5 directory is incomplete"));
    }
    let mut magic = [0; 9];
    File::open(path.join("md.idx"))?.read_exact(&mut magic)?;
    if &magic != b"ADIOS-BP " {
        return Err(invalid("unrecognized BP5 index signature"));
    }
    Ok(data_bytes)
}

fn sync_with_mode(file: &File, previous: Option<&Metadata>, mode: u32) -> Result<(), ArrowError> {
    let mode = if let Some(original) = previous {
        let staged = file.metadata()?;
        if (staged.uid(), staged.gid()) != (original.uid(), original.gid()) {
            return Err(ArrowError::InvalidArgument(
                "BP5 owner or group would change",
            ));
        }
        original.mode() & 0o777
    } else {
        mode
    };
    file.set_permissions(Permissions::from_mode(mode))?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_starts_with_owner_only_permissions() {
        let scratch = tempfile::tempdir().unwrap();
        let staging = create_staging_directory(scratch.path()).unwrap();
        let mode = fs::symlink_metadata(staging.path()).unwrap().mode();
        assert_eq!(mode & 0o077, 0);
    }

    #[test]
    fn native_empty_generation_preserves_empty_arrays_and_metadata() {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("empty.bp");
        let mut manifest = CheckpointManifest {
            checkpoint_id: String::new(),
            format: CheckpointFormat::Adios2,
            run_id: String::new(),
            schema_fingerprint: crate::event_log_schema_fingerprint(),
            row_count: 0,
            blocks: Vec::new(),
            records: Vec::new(),
            checksum: 0,
        };
        manifest.checksum = manifest.calculate_checksum();
        write(&path, &manifest, None).unwrap();
        let (actual, engine) = read(&path, false).unwrap();
        assert_eq!(actual, manifest);
        assert!(engine.is_none());

        manifest.records.push(crate::CheckpointRecord {
            event_id: kairo_ecs_types::EventId::new(1, 0),
            entity_id: None,
            time_ticks: 1,
            priority: 0,
            sequence: 0,
            event_kind: "custom:1".into(),
            status: crate::EventStatus::Dispatched,
            payload_ref: Some(String::new()),
        });
        manifest.blocks.push(ContiguousBlock {
            offset_bytes: 0,
            byte_len: 4,
            row_count: 1,
        });
        manifest.row_count = 1;
        manifest.checksum = manifest.calculate_checksum();
        write(&path, &manifest, None).unwrap();
        assert_eq!(read(&path, false).unwrap(), (manifest, None));
    }
}
