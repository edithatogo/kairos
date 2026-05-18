#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferDescriptor {
    pub offset_bytes: usize,
    pub len_bytes: usize,
    pub stride_bytes: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BufferBridgeError {
    MisalignedOffset {
        offset_bytes: usize,
        alignment: usize,
    },
    ZeroStride,
    MisalignedStride {
        stride_bytes: usize,
        alignment: usize,
    },
    EmptyBuffer,
    InvalidLengthForStride {
        len_bytes: usize,
        stride_bytes: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BufferBridge {
    descriptor: BufferDescriptor,
}

impl BufferBridge {
    pub fn new(descriptor: BufferDescriptor) -> Result<Self, BufferBridgeError> {
        const WGSL_STORAGE_ALIGNMENT: usize = 16;

        if descriptor.len_bytes == 0 {
            return Err(BufferBridgeError::EmptyBuffer);
        }
        if descriptor.offset_bytes % WGSL_STORAGE_ALIGNMENT != 0 {
            return Err(BufferBridgeError::MisalignedOffset {
                offset_bytes: descriptor.offset_bytes,
                alignment: WGSL_STORAGE_ALIGNMENT,
            });
        }
        if descriptor.stride_bytes == 0 {
            return Err(BufferBridgeError::ZeroStride);
        }
        if descriptor.stride_bytes % WGSL_STORAGE_ALIGNMENT != 0 {
            return Err(BufferBridgeError::MisalignedStride {
                stride_bytes: descriptor.stride_bytes,
                alignment: WGSL_STORAGE_ALIGNMENT,
            });
        }
        if descriptor.len_bytes < descriptor.stride_bytes
            || descriptor.len_bytes % descriptor.stride_bytes != 0
        {
            return Err(BufferBridgeError::InvalidLengthForStride {
                len_bytes: descriptor.len_bytes,
                stride_bytes: descriptor.stride_bytes,
            });
        }

        Ok(Self { descriptor })
    }

    pub fn descriptor(&self) -> BufferDescriptor {
        self.descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_webgpu_aligned_buffer_descriptor() {
        let bridge = BufferBridge::new(BufferDescriptor {
            offset_bytes: 32,
            len_bytes: 1024,
            stride_bytes: 16,
        })
        .unwrap();

        assert_eq!(bridge.descriptor().stride_bytes, 16);
    }

    #[test]
    fn rejects_misaligned_wasm_offset() {
        assert_eq!(
            BufferBridge::new(BufferDescriptor {
                offset_bytes: 4,
                len_bytes: 1024,
                stride_bytes: 16,
            })
            .unwrap_err(),
            BufferBridgeError::MisalignedOffset {
                offset_bytes: 4,
                alignment: 16
            }
        );
    }

    #[test]
    fn rejects_partial_elements() {
        assert_eq!(
            BufferBridge::new(BufferDescriptor {
                offset_bytes: 0,
                len_bytes: 24,
                stride_bytes: 16,
            })
            .unwrap_err(),
            BufferBridgeError::InvalidLengthForStride {
                len_bytes: 24,
                stride_bytes: 16,
            }
        );
    }

    #[test]
    fn rejects_zero_stride_without_panicking() {
        assert_eq!(
            BufferBridge::new(BufferDescriptor {
                offset_bytes: 0,
                len_bytes: 16,
                stride_bytes: 0,
            })
            .unwrap_err(),
            BufferBridgeError::ZeroStride
        );
    }
}
