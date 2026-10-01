use crate::compute::{
    CpuFallbackCompute, DesEvent, DispatchShape, GpuCompute, GpuComputeError, GpuState,
    GpuStepStats, NativeGpuBackendKind, PersistentGpuSession, ResidentBufferKind,
    CUDA_BACKEND_NOT_CONFIGURED, WGPU_BACKEND_NOT_CONFIGURED,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceBufferAccess {
    ReadOnly,
    ReadWrite,
    WriteOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentDeviceBuffer {
    pub kind: ResidentBufferKind,
    pub label: &'static str,
    pub len_bytes: usize,
    pub access: DeviceBufferAccess,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PersistentDeviceMemory {
    buffers: Vec<ResidentDeviceBuffer>,
    total_bytes: usize,
}

impl PersistentDeviceMemory {
    pub fn from_state(state: &GpuState) -> Result<Self, GpuComputeError> {
        let footprint = state.footprint()?;
        let mut buffers = Vec::new();
        if footprint.particles_bytes > 0 {
            buffers.push(ResidentDeviceBuffer {
                kind: ResidentBufferKind::Particles,
                label: "particles.resident",
                len_bytes: footprint.particles_bytes,
                access: DeviceBufferAccess::ReadWrite,
            });
        }
        if footprint.entity_values_bytes > 0 {
            buffers.push(ResidentDeviceBuffer {
                kind: ResidentBufferKind::EntityValues,
                label: "entity_values.resident",
                len_bytes: footprint.entity_values_bytes,
                access: DeviceBufferAccess::ReadWrite,
            });
        }
        Ok(Self {
            buffers,
            total_bytes: footprint.checked_total_bytes()?,
        })
    }

    pub fn buffers(&self) -> &[ResidentDeviceBuffer] {
        &self.buffers
    }

    pub fn buffer(&self, kind: ResidentBufferKind) -> Option<&ResidentDeviceBuffer> {
        self.buffers.iter().find(|buffer| buffer.kind == kind)
    }

    pub fn contains(&self, kind: ResidentBufferKind) -> bool {
        self.buffer(kind).is_some()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuFallbackPolicy {
    AllowCpuFallback,
    DisableCpuFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeGpuBackendRequest {
    pub backend: NativeGpuBackendKind,
    pub fallback_policy: GpuFallbackPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeBackendContractError {
    DeviceUnavailable {
        backend: &'static str,
        reason: &'static str,
    },
    FallbackDisabled {
        backend: &'static str,
        reason: &'static str,
    },
}

impl NativeGpuBackendRequest {
    pub const fn required(backend: NativeGpuBackendKind) -> Self {
        Self {
            backend,
            fallback_policy: GpuFallbackPolicy::DisableCpuFallback,
        }
    }

    pub const fn optional(backend: NativeGpuBackendKind) -> Self {
        Self {
            backend,
            fallback_policy: GpuFallbackPolicy::AllowCpuFallback,
        }
    }

    pub const fn backend_name(self) -> &'static str {
        match self.backend {
            NativeGpuBackendKind::Wgpu => WGPU_BACKEND_NOT_CONFIGURED,
            NativeGpuBackendKind::Cuda => CUDA_BACKEND_NOT_CONFIGURED,
        }
    }

    pub const fn unavailable_error(self, reason: &'static str) -> NativeBackendContractError {
        match self.fallback_policy {
            GpuFallbackPolicy::AllowCpuFallback => NativeBackendContractError::DeviceUnavailable {
                backend: self.backend_name(),
                reason,
            },
            GpuFallbackPolicy::DisableCpuFallback => NativeBackendContractError::FallbackDisabled {
                backend: self.backend_name(),
                reason,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuKernelContract {
    pub kernel_name: &'static str,
    pub dispatch_shape: DispatchShape,
    pub transient_upload_bytes: usize,
    pub requires_resident_state: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GpuBatchCommand {
    AbmStep { dt: f32, seed: u64 },
    DesStep { events: Vec<DesEvent> },
}

impl GpuBatchCommand {
    pub fn kernel_contract(
        &self,
        resident_state: &GpuState,
    ) -> Result<GpuKernelContract, GpuComputeError> {
        match self {
            Self::AbmStep { .. } => Ok(GpuKernelContract {
                kernel_name: "abm_step",
                dispatch_shape: DispatchShape::for_items(resident_state.particles.len())?,
                transient_upload_bytes: 0,
                requires_resident_state: true,
            }),
            Self::DesStep { events } => {
                validate_des_events(events, resident_state.entity_values.len())?;
                Ok(GpuKernelContract {
                    kernel_name: "des_dispatch",
                    dispatch_shape: DispatchShape::for_items(events.len())?,
                    transient_upload_bytes: core::mem::size_of_val(events.as_slice()),
                    requires_resident_state: true,
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GpuBatchStats {
    pub commands_dispatched: usize,
    pub uploaded_bytes: usize,
    pub downloaded_bytes: usize,
    pub dispatched_workgroups: u32,
}

impl GpuBatchStats {
    fn record(&mut self, stats: GpuStepStats) -> Result<(), GpuComputeError> {
        self.commands_dispatched = self
            .commands_dispatched
            .checked_add(1)
            .ok_or(GpuComputeError::DispatchSizeOverflow)?;
        self.uploaded_bytes = self
            .uploaded_bytes
            .checked_add(stats.uploaded_bytes)
            .ok_or(GpuComputeError::MemorySizeOverflow)?;
        self.downloaded_bytes = self
            .downloaded_bytes
            .checked_add(stats.downloaded_bytes)
            .ok_or(GpuComputeError::MemorySizeOverflow)?;
        self.dispatched_workgroups = self
            .dispatched_workgroups
            .checked_add(stats.dispatched_workgroups)
            .ok_or(GpuComputeError::DispatchSizeOverflow)?;
        Ok(())
    }
}

pub trait GpuBatchDispatch {
    fn dispatch_batch(
        &mut self,
        commands: &[GpuBatchCommand],
    ) -> Result<GpuBatchStats, GpuComputeError>;
}

impl GpuBatchDispatch for CpuFallbackCompute {
    fn dispatch_batch(
        &mut self,
        commands: &[GpuBatchCommand],
    ) -> Result<GpuBatchStats, GpuComputeError> {
        let mut batch = GpuBatchStats::default();
        for command in commands {
            let stats = match command {
                GpuBatchCommand::AbmStep { dt, seed } => self.run_abm_step(*dt, *seed)?,
                GpuBatchCommand::DesStep { events } => self.run_des_step(events)?,
            };
            batch.record(stats)?;
        }
        Ok(batch)
    }
}

impl GpuBatchDispatch for PersistentGpuSession {
    fn dispatch_batch(
        &mut self,
        commands: &[GpuBatchCommand],
    ) -> Result<GpuBatchStats, GpuComputeError> {
        let mut batch = GpuBatchStats::default();
        for command in commands {
            let stats = match command {
                GpuBatchCommand::AbmStep { dt, seed } => self.run_abm_tick(*dt, *seed)?,
                GpuBatchCommand::DesStep { events } => self.run_des_tick(events)?,
            };
            batch.record(stats)?;
        }
        Ok(batch)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuFallbackParityReport {
    pub commands_checked: usize,
    pub state_matches: bool,
    pub cpu_stats: GpuBatchStats,
    pub candidate_stats: GpuBatchStats,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpuFallbackParityContract {
    pub initial_state: GpuState,
    pub commands: Vec<GpuBatchCommand>,
}

impl CpuFallbackParityContract {
    pub fn new(initial_state: GpuState, commands: Vec<GpuBatchCommand>) -> Self {
        Self {
            initial_state,
            commands,
        }
    }

    pub fn evaluate_persistent_session(
        &self,
        session: &mut PersistentGpuSession,
    ) -> Result<CpuFallbackParityReport, GpuComputeError> {
        let mut cpu = CpuFallbackCompute::new();
        cpu.upload_state(&self.initial_state)?;
        session.upload_once(&self.initial_state)?;
        let cpu_stats = cpu.dispatch_batch(&self.commands)?;
        let candidate_stats = session.dispatch_batch(&self.commands)?;
        let cpu_state = cpu.download_state()?;
        let candidate_state = session.download_state()?;
        Ok(CpuFallbackParityReport {
            commands_checked: self.commands.len(),
            state_matches: cpu_state == candidate_state,
            cpu_stats,
            candidate_stats,
        })
    }
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
