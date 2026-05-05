//! Optional GPU compute facade for KairoECS.
//!
//! The crate is intentionally dependency-free by default. Native GPU backend
//! modules are feature-gated so CPU-only builds do not pull GPU dependencies.

pub mod buffer;
pub mod compute;
pub mod transfer;

#[cfg(feature = "cuda-backend")]
pub mod backends {
    pub mod cuda_backend;
}

#[cfg(feature = "wgpu-backend")]
pub mod wgpu {
    pub mod backend;
}

pub use buffer::{BufferUsage, GpuBuffer, GpuBufferError, TypedGpuBuffer};
pub use compute::{
    AgentParticle, CpuFallbackCompute, DesEvent, GpuBackendAvailability, GpuCompute,
    GpuComputeError, GpuState, GpuStepStats, CUDA_BACKEND_NOT_CONFIGURED,
    WGPU_BACKEND_NOT_CONFIGURED,
};
pub use transfer::{TransferDirection, TransferPlan, TransferStep};
