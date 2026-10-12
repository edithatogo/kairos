//! Private, bounded durable envelope for complete C2 checkpoint sections.
//!
//! The three identities are supplied by the current trusted model. Values read
//! from a checkpoint are only compared with those bindings; they never select
//! code, graphs, or owner decoders.

use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"KC2ENV01";
const SCHEMA_VERSION: u32 = 1;
const HEADER_BYTES: usize = 8 + 4 + 32 * 3 + 8 + 32;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Caller-trusted compatibility identities. The envelope carries opaque copies
/// and does not interpret them as authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CheckpointBindingV1 {
    pub(crate) model_code: [u8; 32],
    pub(crate) configuration: [u8; 32],
    pub(crate) owner_schemas: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CheckpointEnvelopeLimits {
    /// Maximum bytes in the complete envelope, including its fixed header.
    pub(crate) max_file_bytes: usize,
    /// Maximum bytes in the opaque composite owner body.
    pub(crate) max_body_bytes: usize,
}

#[derive(Debug, Error)]
pub(crate) enum CheckpointEnvelopeError {
    #[error("checkpoint envelope is malformed or truncated")]
    InvalidFormat,
    #[error("unsupported checkpoint envelope schema {0}")]
    UnsupportedSchema(u32),
    #[error("checkpoint envelope binding does not match trusted model")]
    BindingMismatch,
    #[error("checkpoint envelope exceeds configured size limits")]
    LimitExceeded,
    #[error("checkpoint envelope allocation failed")]
    AllocationFailed,
    #[error("checkpoint target already exists")]
    TargetExists,
    #[error("filesystem does not support atomic no-clobber publication")]
    UnsupportedPublication,
    #[error("checkpoint was published, but its temporary link could not be removed")]
    PublishedTemporaryCleanupFailed,
    #[error("checkpoint filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CheckpointSaveOutcome {
    /// False means the file was atomically published, but directory durability
    /// could not be confirmed on this filesystem.
    pub(crate) directory_synced: bool,
}

fn checked_total(
    body_len: usize,
    limits: CheckpointEnvelopeLimits,
) -> Result<usize, CheckpointEnvelopeError> {
    if body_len > limits.max_body_bytes {
        return Err(CheckpointEnvelopeError::LimitExceeded);
    }
    let total = HEADER_BYTES
        .checked_add(body_len)
        .ok_or(CheckpointEnvelopeError::LimitExceeded)?;
    if total > limits.max_file_bytes {
        return Err(CheckpointEnvelopeError::LimitExceeded);
    }
    Ok(total)
}

fn digest(body: &[u8]) -> [u8; 32] {
    Sha256::digest(body).into()
}

fn write_header(
    out: &mut Vec<u8>,
    binding: CheckpointBindingV1,
    body: &[u8],
) -> Result<(), CheckpointEnvelopeError> {
    let body_len = u64::try_from(body.len()).map_err(|_| CheckpointEnvelopeError::LimitExceeded)?;
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&SCHEMA_VERSION.to_le_bytes());
    out.extend_from_slice(&binding.model_code);
    out.extend_from_slice(&binding.configuration);
    out.extend_from_slice(&binding.owner_schemas);
    out.extend_from_slice(&body_len.to_le_bytes());
    out.extend_from_slice(&digest(body));
    Ok(())
}

/// Encodes an opaque complete composite body after checking the full size.
pub(crate) fn encode(
    body: &[u8],
    binding: CheckpointBindingV1,
    limits: CheckpointEnvelopeLimits,
) -> Result<Vec<u8>, CheckpointEnvelopeError> {
    let total = checked_total(body.len(), limits)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(total)
        .map_err(|_| CheckpointEnvelopeError::AllocationFailed)?;
    write_header(&mut bytes, binding, body)?;
    bytes.extend_from_slice(body);
    debug_assert_eq!(bytes.len(), total);
    Ok(bytes)
}

fn parse_header(
    header: &[u8; HEADER_BYTES],
    expected: CheckpointBindingV1,
    limits: CheckpointEnvelopeLimits,
    actual_file_len: Option<u64>,
) -> Result<usize, CheckpointEnvelopeError> {
    if &header[..8] != MAGIC {
        return Err(CheckpointEnvelopeError::InvalidFormat);
    }
    let schema = u32::from_le_bytes(header[8..12].try_into().expect("fixed schema slice"));
    if schema != SCHEMA_VERSION {
        return Err(CheckpointEnvelopeError::UnsupportedSchema(schema));
    }
    let model_end = 12 + 32;
    let config_end = model_end + 32;
    let owner_end = config_end + 32;
    if header[12..model_end] != expected.model_code
        || header[model_end..config_end] != expected.configuration
        || header[config_end..owner_end] != expected.owner_schemas
    {
        return Err(CheckpointEnvelopeError::BindingMismatch);
    }
    let body_len_u64 = u64::from_le_bytes(
        header[owner_end..owner_end + 8]
            .try_into()
            .expect("fixed length slice"),
    );
    let body_len =
        usize::try_from(body_len_u64).map_err(|_| CheckpointEnvelopeError::LimitExceeded)?;
    let total = checked_total(body_len, limits)?;
    if let Some(file_len) = actual_file_len {
        if u64::try_from(total).ok() != Some(file_len) {
            return Err(CheckpointEnvelopeError::InvalidFormat);
        }
    }
    Ok(body_len)
}

fn verify_body(
    header: &[u8; HEADER_BYTES],
    body: Vec<u8>,
) -> Result<Vec<u8>, CheckpointEnvelopeError> {
    let digest_offset = HEADER_BYTES - 32;
    if header[digest_offset..] != digest(&body) {
        return Err(CheckpointEnvelopeError::InvalidFormat);
    }
    Ok(body)
}

/// Validates a complete in-memory envelope before returning its opaque body.
pub(crate) fn decode(
    bytes: &[u8],
    expected: CheckpointBindingV1,
    limits: CheckpointEnvelopeLimits,
) -> Result<Vec<u8>, CheckpointEnvelopeError> {
    if bytes.len() > limits.max_file_bytes || bytes.len() < HEADER_BYTES {
        return Err(if bytes.len() > limits.max_file_bytes {
            CheckpointEnvelopeError::LimitExceeded
        } else {
            CheckpointEnvelopeError::InvalidFormat
        });
    }
    let header: &[u8; HEADER_BYTES] = bytes[..HEADER_BYTES]
        .try_into()
        .map_err(|_| CheckpointEnvelopeError::InvalidFormat)?;
    let body_len = parse_header(
        header,
        expected,
        limits,
        Some(u64::try_from(bytes.len()).map_err(|_| CheckpointEnvelopeError::LimitExceeded)?),
    )?;
    let body_start = HEADER_BYTES;
    let body_end = body_start
        .checked_add(body_len)
        .ok_or(CheckpointEnvelopeError::LimitExceeded)?;
    let source_body = &bytes[body_start..body_end];
    let mut body = Vec::new();
    body.try_reserve_exact(body_len)
        .map_err(|_| CheckpointEnvelopeError::AllocationFailed)?;
    body.extend_from_slice(source_body);
    verify_body(header, body)
}

/// Reads and verifies a bounded envelope. No body buffer is allocated until
/// metadata, fixed header, expected bindings, and exact total length pass.
pub(crate) fn read_file(
    path: &Path,
    expected: CheckpointBindingV1,
    limits: CheckpointEnvelopeLimits,
) -> Result<Vec<u8>, CheckpointEnvelopeError> {
    let mut file = File::open(path)?;
    let initial_len = file.metadata()?.len();
    if initial_len > u64::try_from(limits.max_file_bytes).unwrap_or(u64::MAX) {
        return Err(CheckpointEnvelopeError::LimitExceeded);
    }
    if initial_len < HEADER_BYTES as u64 {
        return Err(CheckpointEnvelopeError::InvalidFormat);
    }
    let mut header = [0u8; HEADER_BYTES];
    file.read_exact(&mut header)?;
    let body_len = parse_header(&header, expected, limits, Some(initial_len))?;
    let mut body = Vec::new();
    body.try_reserve_exact(body_len)
        .map_err(|_| CheckpointEnvelopeError::AllocationFailed)?;
    body.resize(body_len, 0);
    file.read_exact(&mut body)?;
    let mut trailing = [0u8; 1];
    if file.read(&mut trailing)? != 0 {
        return Err(CheckpointEnvelopeError::InvalidFormat);
    }
    verify_body(&header, body)
}

struct TemporaryFile {
    path: PathBuf,
    armed: bool,
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn create_temporary(parent: &Path) -> Result<(File, TemporaryFile), CheckpointEnvelopeError> {
    for _ in 0..128 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(".kairos-c2-{}-{sequence}.tmp", std::process::id());
        let path = parent.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => {
                return Ok((file, TemporaryFile { path, armed: true }));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(CheckpointEnvelopeError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique checkpoint temporary file",
    )))
}

fn map_publish_error(error: std::io::Error) -> CheckpointEnvelopeError {
    match error.kind() {
        std::io::ErrorKind::AlreadyExists => CheckpointEnvelopeError::TargetExists,
        std::io::ErrorKind::Unsupported | std::io::ErrorKind::CrossesDevices => {
            CheckpointEnvelopeError::UnsupportedPublication
        }
        _ => CheckpointEnvelopeError::Io(error),
    }
}

/// Writes a complete bounded envelope and atomically publishes it without
/// replacing any existing target. Unsupported hard-link publication fails
/// explicitly; there is no copy or rename fallback.
pub(crate) fn write_file_no_clobber(
    target: &Path,
    body: &[u8],
    binding: CheckpointBindingV1,
    limits: CheckpointEnvelopeLimits,
) -> Result<CheckpointSaveOutcome, CheckpointEnvelopeError> {
    let total = checked_total(body.len(), limits)?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (mut file, mut temporary) = create_temporary(parent)?;
    let mut header = Vec::new();
    header
        .try_reserve_exact(HEADER_BYTES)
        .map_err(|_| CheckpointEnvelopeError::AllocationFailed)?;
    write_header(&mut header, binding, body)?;
    debug_assert_eq!(header.len(), HEADER_BYTES);
    file.write_all(&header)?;
    file.write_all(body)?;
    file.sync_all()?;
    drop(file);

    fs::hard_link(&temporary.path, target).map_err(map_publish_error)?;
    if fs::remove_file(&temporary.path).is_err() {
        return Err(CheckpointEnvelopeError::PublishedTemporaryCleanupFailed);
    }
    temporary.armed = false;

    let directory_synced = File::open(parent)
        .and_then(|directory| directory.sync_all())
        .is_ok();
    debug_assert!(total >= HEADER_BYTES);
    Ok(CheckpointSaveOutcome { directory_synced })
}
