//! Optional GPU compute facade for KairoECS.
//!
//! Provides hardware-accelerated and CPU-fallback compute backends for ABM simulation,
//! batched spatial transit evaluation, and candidate parameter scoring.
//! Queue arbitration and discrete event scheduling remain strictly CPU-owned.

#![forbid(unsafe_code)]

pub mod buffer;
pub mod compute;
pub mod numeric_batch;
pub mod transfer;
pub mod wgpu;

#[cfg(feature = "cuda-backend")]
pub mod backends {
    pub mod cuda_backend;
}

// Shader source constants
pub const BATCH_TRANSIT_WGSL: &str = include_str!("shaders/batch_transit.wgsl");
pub const CANDIDATE_SCORING_WGSL: &str = include_str!("shaders/candidate_scoring.wgsl");
pub const ABM_STEP_WGSL: &str = include_str!("shaders/abm_step.wgsl");
pub const DES_DISPATCH_WGSL: &str = include_str!("shaders/des_dispatch.wgsl");

pub use buffer::{BufferUsage, GpuBuffer, GpuBufferError, TypedGpuBuffer};
pub use compute::{
    AgentParticle, CpuFallbackCompute, DesEvent, DispatchShape, GpuBackendAvailability,
    GpuBackendCapabilities, GpuCompute, GpuComputeError, GpuExecutionPlan, GpuMemoryBudget,
    GpuState, GpuStateFootprint, GpuStepStats, GpuWorkloadKind, CUDA_BACKEND_NOT_CONFIGURED,
    DEFAULT_WORKGROUP_SIZE, TRACK32_TARGET_MEMORY_BUDGET, WGPU_BACKEND_NOT_CONFIGURED,
};
pub use numeric_batch::*;
pub use transfer::{TransferDirection, TransferPlan, TransferStep};
pub use wgpu::{
    BackendApi, CandidateScoringUniformParams, ComputePassEncoder, DeviceMetadata, DeviceStatus,
    DeviceType, StagingBuffer, TransitUniformParams, WgpuBackend, WgpuDeviceCompute,
    PINNED_NAGA_VERSION, PINNED_WGPU_VERSION,
};
