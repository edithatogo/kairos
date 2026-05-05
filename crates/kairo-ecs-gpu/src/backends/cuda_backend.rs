use crate::compute::{
    CpuFallbackCompute, DesEvent, GpuCompute, GpuComputeError, GpuState, GpuStepStats,
};

/// Placeholder CUDA backend. It is feature-gated separately from the default
/// build so CUDA dependencies can be added without leaking into CPU users.
#[derive(Clone, Debug, Default)]
pub struct CudaBackend {
    fallback: CpuFallbackCompute,
}

impl CudaBackend {
    pub fn new_without_context_for_tests() -> Self {
        Self::default()
    }
}

impl GpuCompute for CudaBackend {
    fn backend_name(&self) -> &'static str {
        "cuda-placeholder"
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        self.fallback.upload_state(state)
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        self.fallback.run_abm_step(dt, seed)
    }

    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        self.fallback.run_des_step(events)
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        self.fallback.download_state()
    }
}
