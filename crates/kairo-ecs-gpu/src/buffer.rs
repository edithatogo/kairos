use core::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferUsage {
    Storage,
    StagingUpload,
    StagingDownload,
    Uniform,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GpuBufferError {
    EmptyBuffer,
    SizeOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuBuffer {
    pub label: String,
    pub usage: BufferUsage,
    pub len_bytes: usize,
}

impl GpuBuffer {
    pub fn new(
        label: impl Into<String>,
        usage: BufferUsage,
        len_bytes: usize,
    ) -> Result<Self, GpuBufferError> {
        if len_bytes == 0 {
            return Err(GpuBufferError::EmptyBuffer);
        }

        Ok(Self {
            label: label.into(),
            usage,
            len_bytes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedGpuBuffer<T> {
    raw: GpuBuffer,
    len_items: usize,
    _marker: PhantomData<T>,
}

impl<T> TypedGpuBuffer<T> {
    pub fn new(
        label: impl Into<String>,
        usage: BufferUsage,
        len_items: usize,
    ) -> Result<Self, GpuBufferError> {
        let item_size = core::mem::size_of::<T>();
        let len_bytes = len_items
            .checked_mul(item_size)
            .ok_or(GpuBufferError::SizeOverflow)?;

        Ok(Self {
            raw: GpuBuffer::new(label, usage, len_bytes)?,
            len_items,
            _marker: PhantomData,
        })
    }

    pub fn raw(&self) -> &GpuBuffer {
        &self.raw
    }

    pub fn len_items(&self) -> usize {
        self.len_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_buffer_tracks_item_and_byte_length() {
        let buffer = TypedGpuBuffer::<u32>::new("entities", BufferUsage::Storage, 16).unwrap();

        assert_eq!(buffer.len_items(), 16);
        assert_eq!(buffer.raw().len_bytes, 64);
    }

    #[test]
    fn empty_buffer_is_rejected() {
        assert_eq!(
            TypedGpuBuffer::<u32>::new("empty", BufferUsage::Storage, 0).unwrap_err(),
            GpuBufferError::EmptyBuffer
        );
    }
}
