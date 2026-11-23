use crate::compute::{
    AgentParticle, DesEvent, GpuBackendAvailability, GpuBackendCapabilities, GpuCompute,
    GpuComputeError, GpuState, GpuStepStats, NativeGpuBackendKind, NativeGpuInitializationReport,
    PersistentGpuSession, DEFAULT_WORKGROUP_SIZE,
};
pub use kairo_ecs_cuda::{CudaDeviceInfo, CudaParticle};
use kairo_ecs_cuda::{CudaDeviceSession, CudaEntityDelta};

pub const CUDA_BACKEND_NAME: &str = "nvidia-cuda-driver-compute";
pub const CUDA_DEVICE_UNAVAILABLE_REASON: &str =
    "real CUDA context initialization is not linked in this build";
const CUDA_DRIVER_UNAVAILABLE_REASON: &str = "CUDA driver or device initialization failed";

/// Feature-gated CUDA backend contract and driver wrapper.
pub struct CudaBackend {
    execution: CudaExecution<CudaDeviceSession>,
}

impl core::fmt::Debug for CudaBackend {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CudaBackend")
            .field("session", self.session())
            .field(
                "hardware_only",
                &matches!(self.execution.mode, Mode::Native(_)),
            )
            .finish()
    }
}

impl Default for CudaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CudaBackend {
    fn with_mode(mode: Mode<CudaDeviceSession>) -> Self {
        Self {
            execution: CudaExecution::new(mode),
        }
    }

    pub fn new() -> Self {
        match CudaDeviceSession::new(0) {
            Ok(device) => Self::with_mode(Mode::Compatibility(device)),
            Err(_) => Self::with_mode(Mode::Host),
        }
    }

    pub fn new_without_context_for_tests() -> Self {
        Self::with_mode(Mode::Host)
    }

    pub fn initialize_required_context() -> Result<Self, GpuComputeError> {
        required_device().map(|device| Self::with_mode(Mode::Compatibility(device)))
    }

    /// Require CUDA for both particle and additive DES steps with explicit readback.
    pub fn initialize_hardware_only() -> Result<Self, GpuComputeError> {
        required_device().map(|device| Self::with_mode(Mode::Native(device)))
    }

    pub fn initialization_report() -> NativeGpuInitializationReport {
        match CudaDeviceSession::new(0) {
            Ok(_) => NativeGpuInitializationReport {
                backend: NativeGpuBackendKind::Cuda,
                attempted_real_device: true,
                available: true,
                reason: "device initialized",
            },
            Err(_) => NativeGpuInitializationReport::unavailable(
                NativeGpuBackendKind::Cuda,
                CUDA_DRIVER_UNAVAILABLE_REASON,
            ),
        }
    }

    /// Host state for the compatibility constructor.
    pub fn session(&self) -> &PersistentGpuSession {
        &self.execution.host
    }

    /// Mutable host state for the compatibility constructor.
    pub fn session_mut(&mut self) -> &mut PersistentGpuSession {
        &mut self.execution.host
    }

    pub fn availability(&self) -> GpuBackendAvailability {
        self.execution.availability()
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
        self.execution.upload_state(state)
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        self.execution.run_abm_step(dt, seed)
    }

    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        self.execution.run_des_step(events)
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        self.execution.download_state()
    }
}

fn required_device() -> Result<CudaDeviceSession, GpuComputeError> {
    CudaDeviceSession::new(0).map_err(|_| unavailable(CUDA_DRIVER_UNAVAILABLE_REASON))
}

fn unavailable(reason: &'static str) -> GpuComputeError {
    GpuComputeError::DeviceUnavailable {
        backend: CUDA_BACKEND_NAME,
        reason,
    }
}

enum Mode<D> {
    Host,
    Compatibility(D),
    Native(D),
}

trait ModelDevice {
    fn upload(
        &mut self,
        particles: &[CudaParticle],
        entities: Option<&[i32]>,
    ) -> Result<usize, String>;
    fn resident_entity_count(&self) -> Option<usize>;
    fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String>;
    fn step_des(&mut self, events: &[CudaEntityDelta]) -> Result<u32, String>;
    fn download_entities(&self) -> Result<Vec<i32>, String>;
    fn download_particles(&self) -> Result<Vec<CudaParticle>, String>;
}

impl ModelDevice for CudaDeviceSession {
    fn upload(
        &mut self,
        particles: &[CudaParticle],
        entities: Option<&[i32]>,
    ) -> Result<usize, String> {
        match entities {
            Some(entities) => self.upload_model(particles, entities),
            None => self.upload_particles(particles),
        }
    }

    fn resident_entity_count(&self) -> Option<usize> {
        self.resident_entity_count()
    }

    fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String> {
        self.step_abm(dt, seed)
    }

    fn step_des(&mut self, events: &[CudaEntityDelta]) -> Result<u32, String> {
        self.step_des(events)
    }

    fn download_entities(&self) -> Result<Vec<i32>, String> {
        self.download_entities()
    }

    fn download_particles(&self) -> Result<Vec<CudaParticle>, String> {
        self.download_particles()
    }
}

struct CudaExecution<D> {
    mode: Mode<D>,
    host: PersistentGpuSession,
}

impl<D: ModelDevice> CudaExecution<D> {
    fn new(mode: Mode<D>) -> Self {
        Self {
            mode,
            host: PersistentGpuSession::new(CUDA_BACKEND_NAME),
        }
    }

    fn availability(&self) -> GpuBackendAvailability {
        match self.mode {
            Mode::Host => GpuBackendAvailability::BackendNotConfigured(CUDA_BACKEND_NAME),
            _ => GpuBackendAvailability::Available,
        }
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        let (device, entities, error) = match &mut self.mode {
            Mode::Host => return self.host.upload_once(state),
            Mode::Compatibility(device) => (
                device,
                None,
                GpuComputeError::UnsupportedBackend("cuda particle upload failed"),
            ),
            Mode::Native(device) => (
                device,
                Some(state.entity_values.as_slice()),
                unavailable("CUDA model upload failed"),
            ),
        };
        let particles: Vec<_> = state
            .particles
            .iter()
            .map(|p| CudaParticle {
                x: p.x,
                y: p.y,
                vx: p.vx,
                vy: p.vy,
            })
            .collect();
        let uploaded_bytes = device.upload(&particles, entities).map_err(|_| error)?;
        if entities.is_none() {
            let _ = self.host.upload_once(state);
        }
        Ok(GpuStepStats {
            uploaded_bytes,
            ..GpuStepStats::default()
        })
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        match &mut self.mode {
            Mode::Host => self.host.run_abm_tick_parallel(dt, seed, 4),
            Mode::Compatibility(device) => {
                let dispatched_workgroups = device
                    .step_abm(dt, seed)
                    .map_err(|_| GpuComputeError::UnsupportedBackend("cuda abm step failed"))?;
                let downloaded = device
                    .download_particles()
                    .map_err(|_| GpuComputeError::UnsupportedBackend("cuda download failed"))?;
                let downloaded_bytes = std::mem::size_of_val(downloaded.as_slice());
                if let Ok(mut state) = self.host.download_state() {
                    state.particles = decode_particles(downloaded);
                    let _ = self.host.upload_once(&state);
                }
                Ok(GpuStepStats {
                    uploaded_bytes: 0,
                    downloaded_bytes,
                    dispatched_workgroups,
                })
            }
            Mode::Native(device) => {
                device
                    .resident_entity_count()
                    .ok_or(GpuComputeError::StateNotResident)?;
                let dispatched_workgroups = device
                    .step_abm(dt, seed)
                    .map_err(|_| unavailable("CUDA particle dispatch failed"))?;
                Ok(GpuStepStats {
                    dispatched_workgroups,
                    ..GpuStepStats::default()
                })
            }
        }
    }

    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        let Mode::Native(device) = &mut self.mode else {
            return self.host.run_des_tick(events);
        };
        let entity_count = device
            .resident_entity_count()
            .ok_or(GpuComputeError::StateNotResident)?;
        let mut deltas = Vec::with_capacity(events.len());
        for event in events {
            if event.entity_id >= entity_count as u64 {
                return Err(GpuComputeError::EntityOutOfRange {
                    entity_id: event.entity_id,
                    entity_count,
                });
            }
            deltas.push(CudaEntityDelta {
                entity: event.entity_id as u32,
                delta: event.delta,
            });
        }
        let uploaded_bytes = std::mem::size_of_val(deltas.as_slice());
        let dispatched_workgroups = device
            .step_des(&deltas)
            .map_err(|_| unavailable("CUDA DES dispatch failed"))?;
        Ok(GpuStepStats {
            uploaded_bytes,
            dispatched_workgroups,
            ..GpuStepStats::default()
        })
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        let Mode::Native(device) = &self.mode else {
            return self.host.clone().download_state();
        };
        device
            .resident_entity_count()
            .ok_or(GpuComputeError::StateNotResident)?;
        let entity_values = device
            .download_entities()
            .map_err(|_| unavailable("CUDA entity readback failed"))?;
        let particles = device
            .download_particles()
            .map_err(|_| unavailable("CUDA particle readback failed"))?;
        Ok(GpuState {
            entity_values,
            particles: decode_particles(particles),
        })
    }
}

fn decode_particles(particles: Vec<CudaParticle>) -> Vec<AgentParticle> {
    particles
        .into_iter()
        .map(|p| AgentParticle {
            x: p.x,
            y: p.y,
            vx: p.vx,
            vy: p.vy,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Operation {
        Upload,
        Abm,
        Des,
        ReadParticles,
        ReadEntities,
    }

    #[derive(Default)]
    struct FixtureDevice {
        particles: Vec<CudaParticle>,
        entities: Option<Vec<i32>>,
        failure: Option<Operation>,
    }

    impl FixtureDevice {
        fn check(&self, operation: Operation) -> Result<(), String> {
            if self.failure == Some(operation) {
                return Err("injected device failure".into());
            }
            Ok(())
        }
    }

    impl ModelDevice for FixtureDevice {
        fn upload(
            &mut self,
            particles: &[CudaParticle],
            entities: Option<&[i32]>,
        ) -> Result<usize, String> {
            self.check(Operation::Upload)?;
            self.particles = particles.to_vec();
            self.entities = entities.map(<[_]>::to_vec);
            Ok(std::mem::size_of_val(particles) + entities.map_or(0, std::mem::size_of_val))
        }

        fn resident_entity_count(&self) -> Option<usize> {
            self.entities.as_ref().map(Vec::len)
        }

        fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String> {
            self.check(Operation::Abm)?;
            assert_eq!((dt, seed), (0.25, 7));
            self.particles[0].x = 13.0;
            Ok(1)
        }

        fn step_des(&mut self, events: &[CudaEntityDelta]) -> Result<u32, String> {
            self.check(Operation::Des)?;
            let [event] = events else {
                panic!("expected one translated event");
            };
            assert_eq!((event.entity, event.delta), (0, -3));
            self.entities.as_mut().unwrap()[0] = 1;
            Ok(1)
        }

        fn download_particles(&self) -> Result<Vec<CudaParticle>, String> {
            self.check(Operation::ReadParticles)?;
            Ok(self.particles.clone())
        }

        fn download_entities(&self) -> Result<Vec<i32>, String> {
            self.check(Operation::ReadEntities)?;
            Ok(self.entities.as_ref().unwrap().clone())
        }
    }

    fn state() -> GpuState {
        GpuState {
            particles: vec![AgentParticle {
                x: 2.0,
                y: 3.0,
                vx: 0.5,
                vy: -0.25,
            }],
            entity_values: vec![4],
        }
    }

    const EVENT: DesEvent = DesEvent {
        timestamp_ns: 42,
        entity_id: 0,
        delta: -3,
    };

    fn fail_native(execution: &mut CudaExecution<FixtureDevice>, operation: Operation) {
        let Mode::Native(device) = &mut execution.mode else {
            panic!("expected native mode");
        };
        device.failure = Some(operation);
    }

    #[test]
    fn native_routing_translates_state_and_preserves_typed_failures() {
        let mut execution = CudaExecution::new(Mode::Native(FixtureDevice::default()));
        assert_eq!(execution.availability(), GpuBackendAvailability::Available);
        assert_eq!(
            execution.run_abm_step(0.25, 7),
            Err(GpuComputeError::StateNotResident)
        );
        assert_eq!(
            execution.run_des_step(&[EVENT]),
            Err(GpuComputeError::StateNotResident)
        );
        assert_eq!(
            execution.download_state(),
            Err(GpuComputeError::StateNotResident)
        );

        assert_eq!(execution.upload_state(&state()).unwrap().uploaded_bytes, 20);
        assert_eq!(
            execution.run_abm_step(0.25, 7),
            Ok(GpuStepStats {
                dispatched_workgroups: 1,
                ..GpuStepStats::default()
            })
        );
        let bad = DesEvent {
            entity_id: u64::MAX,
            ..EVENT
        };
        assert_eq!(
            execution.run_des_step(&[EVENT, bad]),
            Err(GpuComputeError::EntityOutOfRange {
                entity_id: u64::MAX,
                entity_count: 1,
            })
        );
        assert_eq!(execution.download_state().unwrap().entity_values, [4]);
        assert_eq!(
            execution.run_des_step(&[EVENT]),
            Ok(GpuStepStats {
                uploaded_bytes: 8,
                dispatched_workgroups: 1,
                ..GpuStepStats::default()
            })
        );
        let mut expected = state();
        expected.particles[0].x = 13.0;
        expected.entity_values[0] = 1;
        assert_eq!(execution.download_state().unwrap(), expected);
        assert_eq!(
            execution.host.download_state(),
            Err(GpuComputeError::StateNotResident)
        );

        fail_native(&mut execution, Operation::Abm);
        assert_eq!(
            execution.run_abm_step(0.25, 7),
            Err(unavailable("CUDA particle dispatch failed"))
        );
        fail_native(&mut execution, Operation::Des);
        assert_eq!(
            execution.run_des_step(&[EVENT]),
            Err(unavailable("CUDA DES dispatch failed"))
        );
        fail_native(&mut execution, Operation::ReadEntities);
        assert_eq!(
            execution.download_state(),
            Err(unavailable("CUDA entity readback failed"))
        );
        fail_native(&mut execution, Operation::ReadParticles);
        assert_eq!(
            execution.download_state(),
            Err(unavailable("CUDA particle readback failed"))
        );
        fail_native(&mut execution, Operation::Upload);
        assert_eq!(
            execution.upload_state(&state()),
            Err(unavailable("CUDA model upload failed"))
        );
    }

    #[test]
    fn compatibility_keeps_host_des_and_eager_particle_readback() {
        let mut execution = CudaExecution::new(Mode::Compatibility(FixtureDevice::default()));
        assert_eq!(execution.upload_state(&state()).unwrap().uploaded_bytes, 16);
        assert_eq!(
            execution.run_abm_step(0.25, 7),
            Ok(GpuStepStats {
                uploaded_bytes: 0,
                downloaded_bytes: 16,
                dispatched_workgroups: 1,
            })
        );
        execution.run_des_step(&[EVENT]).unwrap();
        let mut expected = state();
        expected.particles[0].x = 13.0;
        expected.entity_values[0] = 1;
        assert_eq!(execution.download_state().unwrap(), expected);
    }

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
        assert!(format!("{backend:?}").contains("hardware_only: false"));

        backend.upload_state(&state()).unwrap();
        backend.run_abm_step(0.25, 7).unwrap();
        backend.run_des_step(&[EVENT]).unwrap();
        let updated = backend.download_state().unwrap();
        assert_eq!(updated.entity_values, [1]);
        assert_ne!(updated.particles, state().particles);
        assert_eq!(backend.session_mut().download_state().unwrap(), updated);
    }
}
