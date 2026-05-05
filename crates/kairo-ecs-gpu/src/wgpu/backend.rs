use crate::compute::{
    CpuFallbackCompute, DesEvent, GpuCompute, GpuComputeError, GpuState, GpuStepStats,
};

/// Placeholder wgpu backend that preserves the final facade shape while shader
/// compilation and device setup are wired in later phases.
#[derive(Clone, Debug, Default)]
pub struct WgpuBackend {
    fallback: CpuFallbackCompute,
}

impl WgpuBackend {
    pub fn new_without_device_for_tests() -> Self {
        Self::default()
    }
}

impl GpuCompute for WgpuBackend {
    fn backend_name(&self) -> &'static str {
        "wgpu-placeholder"
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
