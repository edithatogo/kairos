use crate::transfer::{TransferDirection, TransferPlan, TransferStep};

/// Agent particle state used by the first GPU parity harness.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

/// Minimal DES event representation for deterministic dispatch scaffolding.
#[repr(C)]
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

    pub fn transfer_plan(&self) -> Result<TransferPlan, GpuComputeError> {
        let footprint = self.footprint()?;
        let mut plan = TransferPlan::new();

        if footprint.particles_bytes > 0 {
            plan.push(TransferStep {
                label: "particles.upload".into(),
                direction: TransferDirection::HostToGpu,
                len_bytes: footprint.particles_bytes,
            });
            plan.push(TransferStep {
                label: "particles.download".into(),
                direction: TransferDirection::GpuToHost,
                len_bytes: footprint.particles_bytes,
            });
        }

        if footprint.entity_values_bytes > 0 {
            plan.push(TransferStep {
                label: "entity_values.upload".into(),
                direction: TransferDirection::HostToGpu,
                len_bytes: footprint.entity_values_bytes,
            });
            plan.push(TransferStep {
                label: "entity_values.download".into(),
                direction: TransferDirection::GpuToHost,
                len_bytes: footprint.entity_values_bytes,
            });
        }

        Ok(plan)
    }

    pub fn abm_execution_plan(
        &self,
        dt: f32,
        seed: u64,
    ) -> Result<GpuExecutionPlan, GpuComputeError> {
        let state_footprint = self.footprint()?;
        Ok(GpuExecutionPlan {
            workload: GpuWorkloadKind::AbmStep { dt, seed },
            state_footprint,
            dispatch_shape: DispatchShape::for_items(self.particles.len())?,
            transfer_plan: self.transfer_plan()?,
            memory_budget: TRACK32_TARGET_MEMORY_BUDGET,
        })
    }

    pub fn des_execution_plan(
        &self,
        events: &[DesEvent],
    ) -> Result<GpuExecutionPlan, GpuComputeError> {
        let state_footprint = self.footprint()?;
        let mut transfer_plan = self.transfer_plan()?;

        if !events.is_empty() {
            let event_bytes = core::mem::size_of_val(events);
            transfer_plan.push(TransferStep {
                label: "events.upload".into(),
                direction: TransferDirection::HostToGpu,
                len_bytes: event_bytes,
            });
            transfer_plan.push(TransferStep {
                label: "events.download".into(),
                direction: TransferDirection::GpuToHost,
                len_bytes: event_bytes,
            });
        }

        Ok(GpuExecutionPlan {
            workload: GpuWorkloadKind::DesStep {
                event_count: events.len(),
                event_bytes: core::mem::size_of_val(events),
            },
            state_footprint,
            dispatch_shape: DispatchShape::for_items(events.len())?,
            transfer_plan,
            memory_budget: TRACK32_TARGET_MEMORY_BUDGET,
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

#[derive(Clone, Debug, PartialEq)]
pub enum GpuWorkloadKind {
    AbmStep {
        dt: f32,
        seed: u64,
    },
    DesStep {
        event_count: usize,
        event_bytes: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuExecutionPlan {
    pub workload: GpuWorkloadKind,
    pub state_footprint: GpuStateFootprint,
    pub dispatch_shape: DispatchShape,
    pub transfer_plan: TransferPlan,
    pub memory_budget: GpuMemoryBudget,
}

impl GpuExecutionPlan {
    pub fn transfer_bytes_to_gpu(&self) -> usize {
        self.transfer_plan.total_host_to_gpu_bytes()
    }

    pub fn transfer_bytes_from_gpu(&self) -> usize {
        self.transfer_plan.total_gpu_to_host_bytes()
    }

    pub fn roundtrip_transfer_bytes(&self) -> usize {
        self.transfer_bytes_to_gpu()
            .saturating_add(self.transfer_bytes_from_gpu())
    }

    pub fn checked_device_bytes_required(&self) -> Result<usize, GpuComputeError> {
        let state_bytes = self.state_footprint.checked_total_bytes()?;
        match self.workload {
            GpuWorkloadKind::AbmStep { .. } => Ok(state_bytes),
            GpuWorkloadKind::DesStep { event_bytes, .. } => state_bytes
                .checked_add(event_bytes)
                .ok_or(GpuComputeError::MemorySizeOverflow),
        }
    }

    pub fn checked_staging_bytes_required(&self) -> Result<usize, GpuComputeError> {
        self.transfer_plan
            .checked_total_host_to_gpu_bytes()
            .and_then(|uploaded| {
                self.transfer_plan
                    .checked_total_gpu_to_host_bytes()
                    .and_then(|downloaded| uploaded.checked_add(downloaded))
            })
            .ok_or(GpuComputeError::MemorySizeOverflow)
    }

    pub fn fits_within_budget(&self) -> bool {
        let Ok(device_bytes) = self.checked_device_bytes_required() else {
            return false;
        };
        let Ok(staging_bytes) = self.checked_staging_bytes_required() else {
            return false;
        };

        device_bytes <= self.memory_budget.max_device_bytes
            && staging_bytes <= self.memory_budget.max_staging_bytes
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentBufferKind {
    Particles,
    EntityValues,
}

/// Host-observable residency counters for a persistent device-memory session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GpuResidencySnapshot {
    pub state_bytes_resident: usize,
    pub resident_ticks: u64,
    pub host_state_uploads: u64,
    pub host_state_downloads: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GpuComputeError {
    UnsupportedBackend(&'static str),
    DeviceUnavailable {
        backend: &'static str,
        reason: &'static str,
    },
    BufferShapeMismatch {
        expected: usize,
        actual: usize,
    },
    EntityOutOfRange {
        entity_id: u64,
        entity_count: usize,
    },
    MemorySizeOverflow,
    DispatchSizeOverflow,
    StateNotResident,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuBackendAvailability {
    CpuFallback,
    BackendNotConfigured(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeGpuBackendKind {
    Wgpu,
    Cuda,
}

impl NativeGpuBackendKind {
    pub const fn backend_name(self) -> &'static str {
        match self {
            Self::Wgpu => WGPU_BACKEND_NOT_CONFIGURED,
            Self::Cuda => CUDA_BACKEND_NOT_CONFIGURED,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeGpuInitializationReport {
    pub backend: NativeGpuBackendKind,
    pub attempted_real_device: bool,
    pub available: bool,
    pub reason: &'static str,
}

impl NativeGpuInitializationReport {
    pub const fn unavailable(backend: NativeGpuBackendKind, reason: &'static str) -> Self {
        Self {
            backend,
            attempted_real_device: true,
            available: false,
            reason,
        }
    }
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
}

fn deterministic_jitter(seed: u64, index: usize) -> f32 {
    let mut state = (seed as u32) ^ ((index as u32).wrapping_mul(747_796_405));
    state = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    state = (state >> 22) ^ state;
    ((state & 0xffff) as f32 / 65_535.0) - 0.5
}

fn validate_des_events(events: &[DesEvent], entity_count: usize) -> Result<(), GpuComputeError> {
    for event in events {
        if event.entity_id as usize >= entity_count {
            return Err(GpuComputeError::EntityOutOfRange {
                entity_id: event.entity_id,
                entity_count,
            });
        }
    }
    Ok(())
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
            let jitter = deterministic_jitter(seed, index) * 0.001;
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
        validate_des_events(&ordered, entity_count)?;

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

/// Backend-independent contract for keeping state buffers resident across ticks.
///
/// This is intentionally a deterministic contract implementation rather than a
/// native hardware backend. The wgpu/CUDA backends must satisfy this residency
/// surface before Track 52 can claim hardware execution.
#[derive(Clone, Debug)]
pub struct PersistentGpuSession {
    backend_name: &'static str,
    state: Option<GpuState>,
    footprint: GpuStateFootprint,
    resident_ticks: u64,
    host_state_uploads: u64,
    host_state_downloads: u64,
}

impl PersistentGpuSession {
    pub fn new(backend_name: &'static str) -> Self {
        Self {
            backend_name,
            state: None,
            footprint: GpuStateFootprint::default(),
            resident_ticks: 0,
            host_state_uploads: 0,
            host_state_downloads: 0,
        }
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend_name
    }

    pub fn upload_once(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        let footprint = state.footprint()?;
        self.host_state_uploads += 1;
        self.footprint = footprint;
        self.state = Some(state.clone());
        Ok(GpuStepStats {
            uploaded_bytes: footprint.checked_total_bytes()?,
            downloaded_bytes: 0,
            dispatched_workgroups: 0,
        })
    }

    pub fn is_resident(&self, kind: ResidentBufferKind) -> bool {
        let Some(state) = &self.state else {
            return false;
        };
        match kind {
            ResidentBufferKind::Particles => !state.particles.is_empty(),
            ResidentBufferKind::EntityValues => !state.entity_values.is_empty(),
        }
    }

    pub fn run_abm_tick(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        let state = self
            .state
            .as_mut()
            .ok_or(GpuComputeError::StateNotResident)?;
        for (index, particle) in state.particles.iter_mut().enumerate() {
            let jitter = deterministic_jitter(seed, index) * 0.001;
            particle.x += (particle.vx + jitter) * dt;
            particle.y += (particle.vy - jitter) * dt;
        }
        self.resident_ticks += 1;

        Ok(GpuStepStats {
            uploaded_bytes: 0,
            downloaded_bytes: 0,
            dispatched_workgroups: DispatchShape::for_items(state.particles.len())?.workgroup_count,
        })
    }

    pub fn run_des_tick(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        let state = self
            .state
            .as_mut()
            .ok_or(GpuComputeError::StateNotResident)?;
        let mut ordered = events.to_vec();
        ordered.sort();
        let entity_count = state.entity_values.len();
        validate_des_events(&ordered, entity_count)?;

        for event in ordered {
            let index = event.entity_id as usize;
            let Some(value) = state.entity_values.get_mut(index) else {
                return Err(GpuComputeError::EntityOutOfRange {
                    entity_id: event.entity_id,
                    entity_count,
                });
            };
            *value += event.delta;
        }
        self.resident_ticks += 1;

        Ok(GpuStepStats {
            uploaded_bytes: core::mem::size_of_val(events),
            downloaded_bytes: 0,
            dispatched_workgroups: DispatchShape::for_items(events.len())?.workgroup_count,
        })
    }

    pub fn download_state(&mut self) -> Result<GpuState, GpuComputeError> {
        let state = self
            .state
            .as_ref()
            .ok_or(GpuComputeError::StateNotResident)?;
        self.host_state_downloads += 1;
        Ok(state.clone())
    }

    pub fn residency_snapshot(&self) -> GpuResidencySnapshot {
        GpuResidencySnapshot {
            state_bytes_resident: self.footprint.total_bytes(),
            resident_ticks: self.resident_ticks,
            host_state_uploads: self.host_state_uploads,
            host_state_downloads: self.host_state_downloads,
        }
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
        assert_eq!(core::mem::size_of::<AgentParticle>(), 16);
        assert_eq!(core::mem::align_of::<AgentParticle>(), 4);
        assert_eq!(core::mem::size_of::<DesEvent>(), 24);
        assert_eq!(core::mem::align_of::<DesEvent>(), 8);

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

    #[test]
    fn abm_execution_plan_tracks_state_and_transfers() {
        let state = GpuState {
            particles: vec![
                AgentParticle {
                    x: 1.0,
                    y: 2.0,
                    vx: 0.5,
                    vy: -0.25,
                };
                4
            ],
            entity_values: vec![7, 11],
        };

        let plan = state.abm_execution_plan(0.25, 99).unwrap();
        let footprint = state.footprint().unwrap();

        assert_eq!(
            plan.workload,
            GpuWorkloadKind::AbmStep { dt: 0.25, seed: 99 }
        );
        assert_eq!(plan.dispatch_shape.invocation_count, 4);
        assert_eq!(plan.transfer_plan.len(), 4);
        assert_eq!(plan.transfer_bytes_to_gpu(), footprint.total_bytes());
        assert_eq!(plan.transfer_bytes_from_gpu(), footprint.total_bytes());
        assert!(plan.fits_within_budget());
    }

    #[test]
    fn des_execution_plan_accounts_for_event_roundtrip() {
        let state = GpuState {
            particles: vec![],
            entity_values: vec![0, 0, 0],
        };
        let events = vec![
            DesEvent {
                timestamp_ns: 20,
                entity_id: 1,
                delta: 7,
            },
            DesEvent {
                timestamp_ns: 10,
                entity_id: 1,
                delta: -2,
            },
            DesEvent {
                timestamp_ns: 10,
                entity_id: 2,
                delta: 4,
            },
        ];

        let plan = state.des_execution_plan(&events).unwrap();
        let footprint = state.footprint().unwrap();
        let event_bytes = core::mem::size_of_val(&events[..]);

        assert_eq!(
            plan.workload,
            GpuWorkloadKind::DesStep {
                event_count: events.len(),
                event_bytes,
            }
        );
        assert_eq!(plan.dispatch_shape.invocation_count, events.len() as u32);
        assert_eq!(plan.transfer_plan.len(), 4);
        assert_eq!(
            plan.checked_device_bytes_required().unwrap(),
            footprint.total_bytes() + event_bytes
        );
        assert_eq!(
            plan.checked_staging_bytes_required().unwrap(),
            2 * (footprint.total_bytes() + event_bytes)
        );
        assert_eq!(
            plan.transfer_bytes_to_gpu(),
            footprint.total_bytes() + event_bytes
        );
        assert_eq!(
            plan.transfer_bytes_from_gpu(),
            footprint.total_bytes() + event_bytes
        );
        assert!(plan.fits_within_budget());
    }

    #[test]
    fn des_execution_plan_budget_checks_include_event_transfer_pressure() {
        let state = GpuState {
            particles: vec![],
            entity_values: vec![0],
        };
        let events = vec![
            DesEvent {
                timestamp_ns: 1,
                entity_id: 0,
                delta: 1,
            },
            DesEvent {
                timestamp_ns: 2,
                entity_id: 0,
                delta: -1,
            },
        ];

        let mut plan = state.des_execution_plan(&events).unwrap();
        let device_bytes = plan.checked_device_bytes_required().unwrap();
        let staging_bytes = plan.checked_staging_bytes_required().unwrap();

        plan.memory_budget = GpuMemoryBudget::new(device_bytes, staging_bytes - 1);
        assert!(!plan.fits_within_budget());

        plan.memory_budget = GpuMemoryBudget::new(device_bytes - 1, staging_bytes);
        assert!(!plan.fits_within_budget());

        plan.memory_budget = GpuMemoryBudget::new(device_bytes, staging_bytes);
        assert!(plan.fits_within_budget());
    }
}
