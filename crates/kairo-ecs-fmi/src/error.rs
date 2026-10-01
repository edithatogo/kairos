use std::fmt;
use std::path::PathBuf;

pub type FmiResult<T> = Result<T, FmiError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FmiError {
    NullComponent,
    UnsupportedArchiveExtraction {
        path: PathBuf,
    },
    UnsupportedArchiveCompression {
        path: PathBuf,
        method: u16,
    },
    UnsafeArchiveEntry {
        path: PathBuf,
        entry: String,
    },
    MissingModelDescription {
        root: PathBuf,
    },
    MissingBinary {
        platform: String,
        root: PathBuf,
    },
    Validation {
        artifact: &'static str,
        message: String,
    },
    FmiStatus {
        operation: &'static str,
        status: i32,
    },
    InvalidVariableCount {
        expected: usize,
        actual: usize,
    },
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for FmiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullComponent => write!(f, "FMI component pointer is null"),
            Self::UnsupportedArchiveExtraction { path } => write!(
                f,
                "FMU archive extraction is not wired yet for {}; unpack the FMU first",
                path.display()
            ),
            Self::UnsupportedArchiveCompression { path, method } => write!(
                f,
                "unsupported FMU archive compression method {} in {}",
                method,
                path.display()
            ),
            Self::UnsafeArchiveEntry { path, entry } => {
                write!(f, "unsafe archive entry {} in {}", entry, path.display())
            }
            Self::MissingModelDescription { root } => {
                write!(f, "missing modelDescription.xml under {}", root.display())
            }
            Self::MissingBinary { platform, root } => write!(
                f,
                "missing FMI shared library for platform {} under {}",
                platform,
                root.display()
            ),
            Self::Validation { artifact, message } => {
                write!(f, "{} validation failed: {}", artifact, message)
            }
            Self::FmiStatus { operation, status } => {
                write!(f, "{} returned FMI status {}", operation, status)
            }
            Self::InvalidVariableCount { expected, actual } => write!(
                f,
                "invalid variable count: expected {}, got {}",
                expected, actual
            ),
            Self::Io {
                operation,
                path,
                message,
            } => write!(
                f,
                "{} failed for {}: {}",
                operation,
                path.display(),
                message
            ),
        }
    }
}

impl std::error::Error for FmiError {}

#[allow(dead_code)]
pub(crate) fn io_error(operation: &'static str, path: PathBuf, error: std::io::Error) -> FmiError {
    FmiError::Io {
        operation,
        path,
        message: error.to_string(),
    }
}

#[allow(dead_code)]
pub(crate) fn validation_error(artifact: &'static str, message: impl Into<String>) -> FmiError {
    FmiError::Validation {
        artifact,
        message: message.into(),
    }
}
