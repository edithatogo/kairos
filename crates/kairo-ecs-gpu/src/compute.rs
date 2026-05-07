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

impl GpuState {
    pub fn footprint(&self) -> Result<GpuStateFootprint, GpuComputeError> {
        Ok(GpuStateFootprint {
            particles_bytes: checked_slice_bytes::<AgentParticle>(self.particles.len())?,
            entity_values_bytes: checked_slice_bytes::<i32>(self.entity_values.len())?,
        })
    }
}

/// Host-visible footprint for a flat state upload/download cycle.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GpuStateFootprint {
    pub particles_bytes: usize,
    pub entity_values_bytes: usize,
}

impl GpuStateFootprint {
    pub fn total_bytes(self) -> usize {
        self.particles_bytes
            .saturating_add(self.entity_values_bytes)
    }

    pub fn checked_total_bytes(self) -> Result<usize, GpuComputeError> {
        self.particles_bytes
            .checked_add(self.entity_values_bytes)
            .ok_or(GpuComputeError::MemorySizeOverflow)
    }

    pub fn fits_within(self, budget: GpuMemoryBudget) -> bool {
        let Ok(total_bytes) = self.checked_total_bytes() else {
            return false;
        };

        total_bytes <= budget.max_device_bytes
            && total_bytes.saturating_mul(2) <= budget.max_staging_bytes
    }
}

/// Memory budget that can be evaluated without opening a GPU device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GpuMemoryBudget {
    pub max_device_bytes: usize,
    pub max_staging_bytes: usize,
}

impl GpuMemoryBudget {
    pub const fn new(max_device_bytes: usize, max_staging_bytes: usize) -> Self {
        Self {
            max_device_bytes,
            max_staging_bytes,
        }
    }
}

pub const TRACK32_TARGET_MEMORY_BUDGET: GpuMemoryBudget =
    GpuMemoryBudget::new(1_000_000_000, 2_000_000_000);
pub const DEFAULT_WORKGROUP_SIZE: u32 = 256;

/// Deterministic kernel dispatch shape shared by CPU fallback and backend stubs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DispatchShape {
    pub workgroup_size: u32,
    pub workgroup_count: u32,
    pub invocation_count: u32,
}

impl DispatchShape {
    pub fn for_items(item_count: usize) -> Result<Self, GpuComputeError> {
        let invocation_count =
            u32::try_from(item_count).map_err(|_| GpuComputeError::DispatchSizeOverflow)?;
        Ok(Self {
            workgroup_size: DEFAULT_WORKGROUP_SIZE,
            workgroup_count: invocation_count.saturating_add(DEFAULT_WORKGROUP_SIZE - 1)
                / DEFAULT_WORKGROUP_SIZE,
            invocation_count,
        })
    }
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
    MemorySizeOverflow,
    DispatchSizeOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuBackendAvailability {
    CpuFallback,
    BackendNotConfigured(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GpuBackendCapabilities {
    pub backend_name: &'static str,
    pub availability: GpuBackendAvailability,
    pub max_workgroup_size: u32,
    pub supports_zero_copy_borrow: bool,
    pub supports_unified_memory: bool,
}

impl GpuBackendCapabilities {
    pub const fn cpu_fallback() -> Self {
        Self {
            backend_name: "cpu-fallback",
            availability: GpuBackendAvailability::CpuFallback,
            max_workgroup_size: DEFAULT_WORKGROUP_SIZE,
            supports_zero_copy_borrow: false,
            supports_unified_memory: false,
        }
    }

    pub const fn not_configured(backend_name: &'static str) -> Self {
        Self {
            backend_name,
            availability: GpuBackendAvailability::BackendNotConfigured(backend_name),
            max_workgroup_size: DEFAULT_WORKGROUP_SIZE,
            supports_zero_copy_borrow: false,
            supports_unified_memory: false,
        }
    }
}

/// Backend-independent GPU compute contract.
pub trait GpuCompute {
    fn backend_name(&self) -> &'static str;
    fn capabilities(&self) -> GpuBackendCapabilities;
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

    fn capabilities(&self) -> GpuBackendCapabilities {
        GpuBackendCapabilities::cpu_fallback()
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        let footprint = state.footprint()?;
        self.state = state.clone();
        Ok(GpuStepStats {
            uploaded_bytes: footprint.checked_total_bytes()?,
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
            dispatched_workgroups: DispatchShape::for_items(self.state.particles.len())?
                .workgroup_count,
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
            uploaded_bytes: core::mem::size_of_val(events),
            downloaded_bytes: 0,
            dispatched_workgroups: DispatchShape::for_items(events.len())?.workgroup_count,
        })
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        Ok(self.state.clone())
    }
}

fn checked_slice_bytes<T>(len: usize) -> Result<usize, GpuComputeError> {
    len.checked_mul(core::mem::size_of::<T>())
        .ok_or(GpuComputeError::MemorySizeOverflow)
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

    #[test]
    fn state_footprint_tracks_flat_buffer_bytes() {
        let state = GpuState {
            particles: vec![
                AgentParticle {
                    x: 0.0,
                    y: 0.0,
                    vx: 1.0,
                    vy: 1.0,
                };
                2
            ],
            entity_values: vec![1, 2, 3],
        };

        let footprint = state.footprint().unwrap();

        assert_eq!(
            footprint.total_bytes(),
            2 * core::mem::size_of::<AgentParticle>() + 3 * core::mem::size_of::<i32>()
        );
        assert!(footprint.fits_within(TRACK32_TARGET_MEMORY_BUDGET));
    }

    #[test]
    fn dispatch_shape_uses_webgpu_safe_workgroups() {
        assert_eq!(
            DispatchShape::for_items(257).unwrap(),
            DispatchShape {
                workgroup_size: DEFAULT_WORKGROUP_SIZE,
                workgroup_count: 2,
                invocation_count: 257,
            }
        );
    }
}
