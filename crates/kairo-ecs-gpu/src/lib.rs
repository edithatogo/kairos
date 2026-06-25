//! Optional GPU compute facade for KairoECS.
//!
//! The crate is intentionally dependency-free by default. Native GPU backend
//! modules are feature-gated so CPU-only builds do not pull GPU dependencies.

pub mod buffer;
pub mod compute;
pub mod contract;
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
    AgentParticle, CpuFallbackCompute, DesEvent, DispatchShape, GpuBackendAvailability,
    GpuBackendCapabilities, GpuCompute, GpuComputeError, GpuExecutionPlan, GpuMemoryBudget,
    GpuResidencySnapshot, GpuState, GpuStateFootprint, GpuStepStats, GpuWorkloadKind,
    NativeGpuBackendKind, NativeGpuInitializationReport, PersistentGpuSession, ResidentBufferKind,
    CUDA_BACKEND_NOT_CONFIGURED, DEFAULT_WORKGROUP_SIZE, TRACK32_TARGET_MEMORY_BUDGET,
    WGPU_BACKEND_NOT_CONFIGURED,
};
pub use contract::{
    CpuFallbackParityContract, CpuFallbackParityReport, DeviceBufferAccess, GpuBatchCommand,
    GpuBatchDispatch, GpuBatchStats, GpuFallbackPolicy, GpuKernelContract,
    NativeBackendContractError, NativeGpuBackendRequest, PersistentDeviceMemory,
    ResidentDeviceBuffer,
};
pub use transfer::{TransferDirection, TransferPlan, TransferStep};
