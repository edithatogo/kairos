/// Agent particle state used by the first GPU parity harness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

/// Minimal DES event representation for deterministic dispatch scaffolding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DesEvent {
    pub timestamp_ns: u64,
    pub entity_id: u64,
    pub delta: i32,
}

/// Flat state buffers that can be uploaded to a GPU backend.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuState {
    pub particles: Vec<AgentParticle>,
    pub entity_values: Vec<i32>,
}

/// Runtime metrics reported by a compute backend.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuStepStats {
    pub uploaded_bytes: usize,
    pub downloaded_bytes: usize,
    pub dispatched_workgroups: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GpuComputeError {
    UnsupportedBackend(&'static str),
    BufferShapeMismatch { expected: usize, actual: usize },
    EntityOutOfRange { entity_id: u64, entity_count: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuBackendAvailability {
    CpuFallback,
    BackendNotConfigured(&'static str),
}

/// Backend-independent GPU compute contract.
pub trait GpuCompute {
    fn backend_name(&self) -> &'static str;
    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError>;
    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError>;
    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError>;
    fn download_state(&self) -> Result<GpuState, GpuComputeError>;
}

pub const WGPU_BACKEND_NOT_CONFIGURED: &str = "wgpu-backend-not-configured";
pub const CUDA_BACKEND_NOT_CONFIGURED: &str = "cuda-backend-not-configured";

/// Deterministic fallback used by CI and by parity tests on machines without GPU hardware.
#[derive(Clone, Debug, Default)]
pub struct CpuFallbackCompute {
    state: GpuState,
}

impl CpuFallbackCompute {
    pub fn new() -> Self {
        Self::default()
    }

    fn jitter(seed: u64, index: usize) -> f32 {
        let mut value = seed ^ ((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        value ^= value >> 30;
        value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value ^= value >> 27;
        value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^= value >> 31;
        ((value & 0xffff) as f32 / 65_535.0) - 0.5
    }
}

impl GpuCompute for CpuFallbackCompute {
    fn backend_name(&self) -> &'static str {
        "cpu-fallback"
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        self.state = state.clone();
        Ok(GpuStepStats {
            uploaded_bytes: state.particles.len() * core::mem::size_of::<AgentParticle>()
                + state.entity_values.len() * core::mem::size_of::<i32>(),
            downloaded_bytes: 0,
            dispatched_workgroups: 0,
        })
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        for (index, particle) in self.state.particles.iter_mut().enumerate() {
            let jitter = Self::jitter(seed, index) * 0.001;
            particle.x += (particle.vx + jitter) * dt;
            particle.y += (particle.vy - jitter) * dt;
        }

        Ok(GpuStepStats {
            uploaded_bytes: 0,
            downloaded_bytes: 0,
            dispatched_workgroups: ((self.state.particles.len() as u32).saturating_add(255)) / 256,
        })
    }

    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        let mut ordered = events.to_vec();
        ordered.sort();
        let entity_count = self.state.entity_values.len();

        for event in ordered {
            let index = event.entity_id as usize;
            let Some(value) = self.state.entity_values.get_mut(index) else {
                return Err(GpuComputeError::EntityOutOfRange {
                    entity_id: event.entity_id,
                    entity_count,
                });
            };
            *value += event.delta;
        }

        Ok(GpuStepStats {
            uploaded_bytes: events.len() * core::mem::size_of::<DesEvent>(),
            downloaded_bytes: 0,
            dispatched_workgroups: ((events.len() as u32).saturating_add(255)) / 256,
        })
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        Ok(self.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abm_step_is_seed_reproducible() {
        let state = GpuState {
            particles: vec![AgentParticle {
                x: 1.0,
                y: 2.0,
                vx: 0.5,
                vy: -0.25,
            }],
            entity_values: vec![],
        };
        let mut a = CpuFallbackCompute::new();
        let mut b = CpuFallbackCompute::new();

        a.upload_state(&state).unwrap();
        b.upload_state(&state).unwrap();
        a.run_abm_step(0.25, 42).unwrap();
        b.run_abm_step(0.25, 42).unwrap();

        assert_eq!(a.download_state().unwrap(), b.download_state().unwrap());
    }
}
