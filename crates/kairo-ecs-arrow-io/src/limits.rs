use std::io::{self, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::IoError;

/// Caller-selected bounds for one complete transport operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_batch_rows: usize,
    pub max_total_rows: usize,
    pub max_batches: usize,
    pub max_columns: usize,
}

impl Default for IoLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
            max_batch_rows: 100_000,
            max_total_rows: 1_000_000,
            max_batches: 4096,
            max_columns: 4096,
        }
    }
}

impl IoLimits {
    pub fn validate(self) -> Result<Self, IoError> {
        if self.max_input_bytes == 0
            || self.max_output_bytes == 0
            || self.max_batch_rows == 0
            || self.max_total_rows == 0
            || self.max_batches == 0
            || self.max_columns == 0
        {
            return Err(IoError::InvalidLimits);
        }
        Ok(self)
    }

    pub(crate) fn check_input_len(self, len: usize) -> Result<(), IoError> {
        self.validate()?;
        if len > self.max_input_bytes {
            return Err(IoError::LimitExceeded("max_input_bytes"));
        }
        Ok(())
    }

    pub(crate) fn check_columns(self, columns: usize) -> Result<(), IoError> {
        if columns > self.max_columns {
            return Err(IoError::LimitExceeded("max_columns"));
        }
        Ok(())
    }

    pub(crate) fn next_batch_count(self, count: usize) -> Result<usize, IoError> {
        let next = count
            .checked_add(1)
            .ok_or(IoError::LimitExceeded("max_batches"))?;
        if next > self.max_batches {
            return Err(IoError::LimitExceeded("max_batches"));
        }
        Ok(next)
    }

    pub(crate) fn next_row_count(self, count: usize, rows: usize) -> Result<usize, IoError> {
        if rows > self.max_batch_rows {
            return Err(IoError::LimitExceeded("max_batch_rows"));
        }
        let next = count
            .checked_add(rows)
            .ok_or(IoError::LimitExceeded("max_total_rows"))?;
        if next > self.max_total_rows {
            return Err(IoError::LimitExceeded("max_total_rows"));
        }
        Ok(next)
    }

    #[cfg(any(feature = "ipc", feature = "parquet"))]
    pub(crate) fn check_batch(
        self,
        batch: &arrow_array::RecordBatch,
        expected: &arrow_schema::Schema,
    ) -> Result<(), IoError> {
        self.check_columns(batch.num_columns())?;
        self.check_columns(expected.fields().len())?;
        if batch.schema().as_ref() != expected {
            return Err(IoError::SchemaMismatch);
        }
        if batch.num_rows() > self.max_batch_rows {
            return Err(IoError::LimitExceeded("max_batch_rows"));
        }
        Ok(())
    }

    #[cfg(any(feature = "ipc", feature = "parquet"))]
    pub(crate) fn check_batches(
        self,
        batches: &[arrow_array::RecordBatch],
        expected: &arrow_schema::Schema,
    ) -> Result<usize, IoError> {
        self.validate()?;
        self.check_columns(expected.fields().len())?;
        let mut rows = 0usize;
        for (index, batch) in batches.iter().enumerate() {
            if index >= self.max_batches {
                return Err(IoError::LimitExceeded("max_batches"));
            }
            self.check_batch(batch, expected)?;
            rows = self.next_row_count(rows, batch.num_rows())?;
        }
        Ok(rows)
    }
}

/// A Vec-backed sink that never appends bytes beyond its configured limit.
pub(crate) struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: Arc<AtomicBool>,
}

impl BoundedBuffer {
    pub(crate) fn new(limit: usize, exceeded: Arc<AtomicBool>) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded,
        }
    }

    pub(crate) fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    fn limit_error(&self) -> io::Error {
        self.exceeded.store(true, Ordering::Relaxed);
        io::Error::new(io::ErrorKind::WriteZero, "maximum output bytes exceeded")
    }
}

impl Write for BoundedBuffer {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let new_len = self
            .bytes
            .len()
            .checked_add(buffer.len())
            .ok_or_else(|| self.limit_error())?;
        if new_len > self.limit {
            return Err(self.limit_error());
        }
        self.bytes
            .try_reserve_exact(buffer.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn output_limit_hit(exceeded: &AtomicBool) -> bool {
    exceeded.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::{atomic::AtomicBool, Arc},
    };

    use super::{BoundedBuffer, IoLimits};
    use crate::IoError;

    #[test]
    fn every_zero_cap_is_invalid() {
        let defaults = IoLimits::default();
        let zeroes = [
            IoLimits {
                max_input_bytes: 0,
                ..defaults
            },
            IoLimits {
                max_output_bytes: 0,
                ..defaults
            },
            IoLimits {
                max_batch_rows: 0,
                ..defaults
            },
            IoLimits {
                max_total_rows: 0,
                ..defaults
            },
            IoLimits {
                max_batches: 0,
                ..defaults
            },
            IoLimits {
                max_columns: 0,
                ..defaults
            },
        ];
        for limits in zeroes {
            assert!(matches!(limits.validate(), Err(IoError::InvalidLimits)));
        }
    }

    #[test]
    fn independent_input_column_batch_and_total_row_caps_are_enforced() {
        let limits = IoLimits {
            max_input_bytes: 2,
            max_output_bytes: 8,
            max_batch_rows: 2,
            max_total_rows: 3,
            max_batches: 2,
            max_columns: 1,
        };
        assert!(limits.check_input_len(2).is_ok());
        assert!(matches!(
            limits.check_input_len(3),
            Err(IoError::LimitExceeded("max_input_bytes"))
        ));
        assert!(matches!(
            limits.check_columns(2),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        assert!(matches!(limits.next_row_count(0, 2), Ok(2)));
        assert!(matches!(
            limits.next_row_count(0, 3),
            Err(IoError::LimitExceeded("max_batch_rows"))
        ));
        assert!(matches!(
            limits.next_row_count(2, 2),
            Err(IoError::LimitExceeded("max_total_rows"))
        ));
        assert!(matches!(limits.next_batch_count(1), Ok(2)));
        assert!(matches!(
            limits.next_batch_count(2),
            Err(IoError::LimitExceeded("max_batches"))
        ));
    }

    #[test]
    fn checked_count_overflow_fails_closed() {
        let limits = IoLimits::default();
        assert!(matches!(
            limits.next_row_count(usize::MAX, 1),
            Err(IoError::LimitExceeded("max_total_rows"))
        ));
        assert!(matches!(
            limits.next_batch_count(usize::MAX),
            Err(IoError::LimitExceeded("max_batches"))
        ));
    }

    #[test]
    fn bounded_buffer_accepts_exact_limit_and_rejects_before_appending_past_it() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut sink = BoundedBuffer::new(3, flag.clone());
        sink.write_all(&[1, 2, 3]).unwrap();
        assert!(sink.write_all(&[4]).is_err());
        assert!(super::output_limit_hit(&flag));
        assert_eq!(sink.into_inner(), vec![1, 2, 3]);
    }
}
