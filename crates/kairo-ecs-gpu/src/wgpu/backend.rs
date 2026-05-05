use crate::compute::{
    DesEvent, GpuBackendAvailability, GpuCompute, GpuComputeError, GpuState, GpuStepStats,
    WGPU_BACKEND_NOT_CONFIGURED,
};

/// Feature-gated wgpu backend contract.
///
/// The crate does not include the real `wgpu` dependency yet, so this type
/// reports an explicit unavailable status instead of falling back to CPU work.
#[derive(Clone, Debug, Default)]
pub struct WgpuBackend;

impl WgpuBackend {
    pub fn new_without_device_for_tests() -> Self {
        Self::default()
    }

    pub fn availability(&self) -> GpuBackendAvailability {
        GpuBackendAvailability::BackendNotConfigured(WGPU_BACKEND_NOT_CONFIGURED)
    }
}

impl GpuCompute for WgpuBackend {
    fn backend_name(&self) -> &'static str {
        WGPU_BACKEND_NOT_CONFIGURED
    }

    fn upload_state(&mut self, _state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            WGPU_BACKEND_NOT_CONFIGURED,
        ))
    }

    fn run_abm_step(&mut self, _dt: f32, _seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            WGPU_BACKEND_NOT_CONFIGURED,
        ))
    }

    fn run_des_step(&mut self, _events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            WGPU_BACKEND_NOT_CONFIGURED,
        ))
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            WGPU_BACKEND_NOT_CONFIGURED,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgpu_backend_reports_not_configured_without_device_dependencies() {
        let mut backend = WgpuBackend::new_without_device_for_tests();

        assert_eq!(
            backend.availability(),
            GpuBackendAvailability::BackendNotConfigured(WGPU_BACKEND_NOT_CONFIGURED)
        );
        assert_eq!(backend.backend_name(), WGPU_BACKEND_NOT_CONFIGURED);
        assert_eq!(
            backend.upload_state(&GpuState::default()),
            Err(GpuComputeError::UnsupportedBackend(
                WGPU_BACKEND_NOT_CONFIGURED
            ))
        );
    }
}
