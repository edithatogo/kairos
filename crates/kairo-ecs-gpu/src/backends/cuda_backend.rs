use crate::compute::{
    DesEvent, GpuBackendAvailability, GpuCompute, GpuComputeError, GpuState, GpuStepStats,
    CUDA_BACKEND_NOT_CONFIGURED,
};

/// Feature-gated CUDA backend contract.
///
/// The crate does not include CUDA bindings yet, so this type reports an
/// explicit unavailable status instead of falling back to CPU work.
#[derive(Clone, Debug, Default)]
pub struct CudaBackend;

impl CudaBackend {
    pub fn new_without_context_for_tests() -> Self {
        Self
    }

    pub fn availability(&self) -> GpuBackendAvailability {
        GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NOT_CONFIGURED)
    }
}

impl GpuCompute for CudaBackend {
    fn backend_name(&self) -> &'static str {
        CUDA_BACKEND_NOT_CONFIGURED
    }

    fn capabilities(&self) -> crate::compute::GpuBackendCapabilities {
        crate::compute::GpuBackendCapabilities::not_configured(CUDA_BACKEND_NOT_CONFIGURED)
    }

    fn upload_state(&mut self, _state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            CUDA_BACKEND_NOT_CONFIGURED,
        ))
    }

    fn run_abm_step(&mut self, _dt: f32, _seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            CUDA_BACKEND_NOT_CONFIGURED,
        ))
    }

    fn run_des_step(&mut self, _events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            CUDA_BACKEND_NOT_CONFIGURED,
        ))
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        Err(GpuComputeError::UnsupportedBackend(
            CUDA_BACKEND_NOT_CONFIGURED,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuda_backend_reports_not_configured_without_cuda_bindings() {
        let mut backend = CudaBackend::new_without_context_for_tests();

        assert_eq!(
            backend.availability(),
            GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NOT_CONFIGURED)
        );
        assert_eq!(backend.backend_name(), CUDA_BACKEND_NOT_CONFIGURED);
        assert_eq!(
            backend.capabilities(),
            crate::compute::GpuBackendCapabilities::not_configured(CUDA_BACKEND_NOT_CONFIGURED)
        );
        assert_eq!(
            backend.upload_state(&GpuState::default()),
            Err(GpuComputeError::UnsupportedBackend(
                CUDA_BACKEND_NOT_CONFIGURED
            ))
        );
    }
}
