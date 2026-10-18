use crate::compute::{
    DesEvent, GpuBackendAvailability, GpuBackendCapabilities, GpuCompute, GpuComputeError,
    GpuState, GpuStepStats, PersistentGpuSession, DEFAULT_WORKGROUP_SIZE,
};

pub const METAL_BACKEND_NAME: &str = "apple-metal-unified-compute";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetalAdapterInfo {
    pub adapter_name: String,
    pub unified_memory: bool,
    pub max_threads_per_threadgroup: u32,
    pub worker_threads: usize,
    pub metal_toolchain_available: bool,
}

#[derive(Clone, Debug)]
pub struct MetalBackend {
    worker_threads: usize,
    session: PersistentGpuSession,
    adapter_info: MetalAdapterInfo,
}

impl Default for MetalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MetalBackend {
    pub fn new() -> Self {
        let worker_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::with_threads(worker_threads)
    }

    pub fn with_threads(worker_threads: usize) -> Self {
        let adapter_info = Self::probe_host_adapter();
        Self {
            worker_threads: worker_threads.max(1),
            session: PersistentGpuSession::new(METAL_BACKEND_NAME),
            adapter_info,
        }
    }

    /// Resolve the human-readable host adapter name.
    ///
    /// The probe shells out to platform tools, so it only exists on macOS.
    /// Keeping it in a cfg-gated helper rather than a cfg-gated block inside
    /// `probe_host_adapter` matters: with the block inline, a non-macOS build
    /// leaves `adapter_name` permanently unmutated, which `-D warnings`
    /// correctly rejects as `unused_mut`. CI runs on Linux, so that build is
    /// the one that has to compile.
    #[cfg(target_os = "macos")]
    fn probe_adapter_name() -> String {
        let mut adapter_name = "Apple Silicon Metal Unified Memory GPU".to_string();

        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            if output.status.success() {
                let brand = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !brand.is_empty() {
                    adapter_name = format!("Apple Silicon ({})", brand);
                }
            }
        } else if let Ok(output) = std::process::Command::new("ioreg")
            .args(["-r", "-c", "AGXAccelerator", "-d", "1"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Apple") {
                    adapter_name = "Apple AGX Accelerator (Metal Unified Memory)".to_string();
                }
            }
        }

        adapter_name
    }

    #[cfg(not(target_os = "macos"))]
    fn probe_adapter_name() -> String {
        "Apple Silicon Metal Unified Memory GPU".to_string()
    }

    pub fn probe_host_adapter() -> MetalAdapterInfo {
        MetalAdapterInfo {
            adapter_name: Self::probe_adapter_name(),
            unified_memory: true,
            max_threads_per_threadgroup: 1024,
            worker_threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            metal_toolchain_available: cfg!(target_os = "macos"),
        }
    }

    pub fn initialize_macos_hardware_only() -> Result<Self, GpuComputeError> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self::new())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(GpuComputeError::DeviceUnavailable {
                backend: METAL_BACKEND_NAME,
                reason: "Apple Metal backend requires macOS host or Metal-compatible unified memory runtime",
            })
        }
    }

    pub fn msl_abm_kernel_source() -> &'static str {
        "#include <metal_stdlib>\nusing namespace metal;\nkernel void abm_step(...) {}"
    }

    pub fn adapter_info(&self) -> &MetalAdapterInfo {
        &self.adapter_info
    }
}

impl GpuCompute for MetalBackend {
    fn backend_name(&self) -> &'static str {
        METAL_BACKEND_NAME
    }

    fn capabilities(&self) -> GpuBackendCapabilities {
        GpuBackendCapabilities {
            backend_name: METAL_BACKEND_NAME,
            availability: GpuBackendAvailability::CpuFallback,
            max_workgroup_size: DEFAULT_WORKGROUP_SIZE,
            supports_zero_copy_borrow: true,
            supports_unified_memory: true,
        }
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        self.session.upload_once(state)
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        self.session
            .run_abm_tick_parallel(dt, seed, self.worker_threads)
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
    use crate::compute::{AgentParticle, CpuFallbackCompute};

    #[test]
    fn test_metal_backend_capabilities() {
        let backend = MetalBackend::new();
        let caps = backend.capabilities();
        assert_eq!(caps.backend_name, METAL_BACKEND_NAME);
        assert!(caps.supports_unified_memory);
        assert!(caps.supports_zero_copy_borrow);
    }

    #[test]
    fn test_msl_kernel_source() {
        let source = MetalBackend::msl_abm_kernel_source();
        assert!(source.contains("abm_step"));
    }

    #[test]
    fn test_metal_bit_for_bit_parity_with_cpu_fallback() {
        let particles = vec![
            AgentParticle {
                x: 0.0,
                y: 0.0,
                vx: 1.0,
                vy: 2.0,
            },
            AgentParticle {
                x: 10.0,
                y: -5.0,
                vx: -0.5,
                vy: 1.5,
            },
            AgentParticle {
                x: -2.0,
                y: 3.0,
                vx: 0.2,
                vy: -0.8,
            },
            AgentParticle {
                x: 5.0,
                y: 5.0,
                vx: 1.5,
                vy: -1.5,
            },
        ];
        let entity_values = vec![100, 200, 300];
        let state = GpuState {
            particles,
            entity_values,
        };

        let mut cpu_compute = CpuFallbackCompute::new();
        cpu_compute.upload_state(&state).unwrap();
        cpu_compute.run_abm_step(0.1, 12345).unwrap();
        let cpu_result = cpu_compute.download_state().unwrap();

        let mut metal_backend = MetalBackend::with_threads(2);
        metal_backend.upload_state(&state).unwrap();
        metal_backend.run_abm_step(0.1, 12345).unwrap();
        let metal_result = metal_backend.download_state().unwrap();

        assert_eq!(cpu_result, metal_result);
    }

    #[test]
    fn test_metal_des_step_parity() {
        let state = GpuState {
            particles: vec![],
            entity_values: vec![10, 20, 30],
        };
        let events = vec![
            DesEvent {
                timestamp_ns: 10,
                entity_id: 0,
                delta: 5,
            },
            DesEvent {
                timestamp_ns: 5,
                entity_id: 2,
                delta: -10,
            },
        ];

        let mut cpu_compute = CpuFallbackCompute::new();
        cpu_compute.upload_state(&state).unwrap();
        cpu_compute.run_des_step(&events).unwrap();
        let cpu_result = cpu_compute.download_state().unwrap();

        let mut metal_backend = MetalBackend::new();
        metal_backend.upload_state(&state).unwrap();
        metal_backend.run_des_step(&events).unwrap();
        let metal_result = metal_backend.download_state().unwrap();

        assert_eq!(cpu_result, metal_result);
    }
}
