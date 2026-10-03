use thiserror::Error;

/// Errors returned by bounded Arrow IPC and Parquet transport.
#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O limits must all be greater than zero")]
    InvalidLimits,
    #[error("I/O limit exceeded: {0}")]
    LimitExceeded(&'static str),
    #[error("record batch schema does not match the declared schema")]
    SchemaMismatch,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(any(feature = "ipc", feature = "parquet"))]
    #[error(transparent)]
    Arrow(#[from] arrow_schema::ArrowError),
    #[cfg(feature = "parquet")]
    #[error(transparent)]
    Parquet(#[from] parquet::errors::ParquetError),
}

#[cfg(test)]
mod tests {
    use super::IoError;

    #[test]
    fn limit_error_keeps_the_named_limit() {
        let error = IoError::LimitExceeded("max_output_bytes");
        assert_eq!(error.to_string(), "I/O limit exceeded: max_output_bytes");
    }
}
