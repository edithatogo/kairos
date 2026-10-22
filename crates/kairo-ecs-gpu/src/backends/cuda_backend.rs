use crate::compute::{
    AgentParticle, DesEvent, GpuBackendAvailability, GpuBackendCapabilities, GpuCompute,
    GpuComputeError, GpuState, GpuStepStats, PersistentGpuSession, DEFAULT_WORKGROUP_SIZE,
};
pub use kairo_ecs_cuda::{CudaDeviceInfo, CudaParticle};

pub const CUDA_BACKEND_NAME: &str = "nvidia-cuda-driver-compute";

pub const CUDA_DEVICE_UNAVAILABLE_REASON: &str =
    "real CUDA context initialization is not linked in this build";

/// Feature-gated CUDA backend contract and driver wrapper.
pub struct CudaBackend {
    session: PersistentGpuSession,
    #[cfg(feature = "cuda-backend")]
    device_session: Option<kairo_ecs_cuda::CudaDeviceSession>,
    #[cfg(not(feature = "cuda-backend"))]
    device_session: (),
}

impl core::fmt::Debug for CudaBackend {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CudaBackend")
            .field("session", &self.session)
            .finish()
    }
}

impl Default for CudaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CudaBackend {
    pub fn new() -> Self {
        #[cfg(feature = "cuda-backend")]
        let device_session = kairo_ecs_cuda::CudaDeviceSession::new(0).ok();
        #[cfg(not(feature = "cuda-backend"))]
        let device_session = ();

        Self {
            session: PersistentGpuSession::new(CUDA_BACKEND_NAME),
            device_session,
        }
    }

    pub fn new_without_context_for_tests() -> Self {
        #[cfg(feature = "cuda-backend")]
        let device_session = None;
        #[cfg(not(feature = "cuda-backend"))]
        let device_session = ();

        Self {
            session: PersistentGpuSession::new(CUDA_BACKEND_NAME),
            device_session,
        }
    }

    pub fn initialize_required_context() -> Result<Self, GpuComputeError> {
        #[cfg(feature = "cuda-backend")]
        {
            let session = kairo_ecs_cuda::CudaDeviceSession::new(0).map_err(|_e| {
                GpuComputeError::DeviceUnavailable {
                    backend: CUDA_BACKEND_NAME,
                    reason: CUDA_DEVICE_UNAVAILABLE_REASON,
                }
            })?;
            Ok(Self {
                session: PersistentGpuSession::new(CUDA_BACKEND_NAME),
                device_session: Some(session),
            })
        }
        #[cfg(not(feature = "cuda-backend"))]
        {
            Err(GpuComputeError::DeviceUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: CUDA_DEVICE_UNAVAILABLE_REASON,
            })
        }
    }

    pub fn initialization_report() -> crate::compute::NativeGpuInitializationReport {
        #[cfg(feature = "cuda-backend")]
        {
            match kairo_ecs_cuda::CudaDeviceSession::new(0) {
                Ok(_s) => crate::compute::NativeGpuInitializationReport {
                    backend: crate::compute::NativeGpuBackendKind::Cuda,
                    attempted_real_device: true,
                    available: true,
                    reason: "device initialized",
                },
                Err(_) => crate::compute::NativeGpuInitializationReport::unavailable(
                    crate::compute::NativeGpuBackendKind::Cuda,
                    CUDA_DEVICE_UNAVAILABLE_REASON,
                ),
            }
        }
        #[cfg(not(feature = "cuda-backend"))]
        {
            crate::compute::NativeGpuInitializationReport::unavailable(
                crate::compute::NativeGpuBackendKind::Cuda,
                CUDA_DEVICE_UNAVAILABLE_REASON,
            )
        }
    }

    pub fn session(&self) -> &PersistentGpuSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut PersistentGpuSession {
        &mut self.session
    }

    pub fn availability(&self) -> GpuBackendAvailability {
        #[cfg(feature = "cuda-backend")]
        {
            if self.device_session.is_some() {
                GpuBackendAvailability::Available
            } else {
                GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NAME)
            }
        }
        #[cfg(not(feature = "cuda-backend"))]
        {
            GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NAME)
        }
    }
}

impl GpuCompute for CudaBackend {
    fn backend_name(&self) -> &'static str {
        CUDA_BACKEND_NAME
    }

    fn capabilities(&self) -> GpuBackendCapabilities {
        GpuBackendCapabilities {
            backend_name: CUDA_BACKEND_NAME,
            max_workgroup_size: DEFAULT_WORKGROUP_SIZE,
            supports_unified_memory: false,
            supports_zero_copy_borrow: false,
            availability: self.availability(),
        }
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        #[cfg(feature = "cuda-backend")]
        {
            if let Some(session) = &mut self.device_session {
                let cuda_particles: Vec<CudaParticle> = state
                    .particles
                    .iter()
                    .map(|p| CudaParticle {
                        x: p.x,
                        y: p.y,
                        vx: p.vx,
                        vy: p.vy,
                    })
                    .collect();
                let bytes = session.upload_particles(&cuda_particles).map_err(|_| {
                    GpuComputeError::UnsupportedBackend("cuda particle upload failed")
                })?;
                let _ = self.session.upload_once(state);
                return Ok(GpuStepStats {
                    uploaded_bytes: bytes,
                    downloaded_bytes: 0,
                    dispatched_workgroups: 1,
                });
            }
        }
        self.session.upload_once(state)
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        #[cfg(feature = "cuda-backend")]
        {
            if let Some(session) = &mut self.device_session {
                let blocks = session
                    .step_abm(dt, seed)
                    .map_err(|_| GpuComputeError::UnsupportedBackend("cuda abm step failed"))?;
                let downloaded = session
                    .download_particles()
                    .map_err(|_| GpuComputeError::UnsupportedBackend("cuda download failed"))?;
                if let Ok(mut cur_state) = self.session.download_state() {
                    cur_state.particles = downloaded
                        .into_iter()
                        .map(|p| AgentParticle {
                            x: p.x,
                            y: p.y,
                            vx: p.vx,
                            vy: p.vy,
                        })
                        .collect();
                    let _ = self.session.upload_once(&cur_state);
                }
                return Ok(GpuStepStats {
                    uploaded_bytes: 0,
                    downloaded_bytes: 0,
                    dispatched_workgroups: blocks,
                });
            }
        }
        self.session.run_abm_tick_parallel(dt, seed, 4)
    }

    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        self.session.run_des_tick(events)
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        let mut session_clone = self.session.clone();
        session_clone.download_state()
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
            GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NAME)
        );
        assert_eq!(backend.backend_name(), CUDA_BACKEND_NAME);
        assert_eq!(
            backend.capabilities(),
            GpuBackendCapabilities {
                backend_name: CUDA_BACKEND_NAME,
                max_workgroup_size: DEFAULT_WORKGROUP_SIZE,
                supports_unified_memory: false,
                supports_zero_copy_borrow: false,
                availability: GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NAME),
            }
        );
        assert_eq!(
            backend.upload_state(&GpuState::default()),
            Ok(GpuStepStats {
                uploaded_bytes: 0,
                downloaded_bytes: 0,
                dispatched_workgroups: 0,
            })
        );
    }
}
