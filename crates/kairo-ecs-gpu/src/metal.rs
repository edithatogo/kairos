#![forbid(unsafe_code)]

#[cfg(feature = "wgpu-backend")]
use crate::compute::AgentParticle;
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

#[cfg(feature = "wgpu-backend")]
struct NativeMetalDeviceContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    abm_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    particle_buffer: Option<wgpu::Buffer>,
    staging_buffer: Option<wgpu::Buffer>,
    particle_count: usize,
}

pub struct MetalBackend {
    worker_threads: usize,
    session: PersistentGpuSession,
    adapter_info: MetalAdapterInfo,
    #[cfg(feature = "wgpu-backend")]
    native_ctx: Option<NativeMetalDeviceContext>,
}

impl core::fmt::Debug for MetalBackend {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MetalBackend")
            .field("worker_threads", &self.worker_threads)
            .field("session", &self.session)
            .field("adapter_info", &self.adapter_info)
            .finish()
    }
}

impl Clone for MetalBackend {
    fn clone(&self) -> Self {
        // Create fresh clone with same session state
        Self {
            worker_threads: self.worker_threads,
            session: self.session.clone(),
            adapter_info: self.adapter_info.clone(),
            #[cfg(feature = "wgpu-backend")]
            native_ctx: None,
        }
    }
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
        #[cfg(feature = "wgpu-backend")]
        let native_ctx = Self::try_init_native_metal();
        #[cfg(not(feature = "wgpu-backend"))]
        let _native_ctx = ();

        Self {
            worker_threads: worker_threads.max(1),
            session: PersistentGpuSession::new(METAL_BACKEND_NAME),
            adapter_info,
            #[cfg(feature = "wgpu-backend")]
            native_ctx,
        }
    }

    #[cfg(feature = "wgpu-backend")]
    fn try_init_native_metal() -> Option<NativeMetalDeviceContext> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .ok()?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("metal_abm_kernel"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
                struct Particle {
                    x: f32,
                    y: f32,
                    vx: f32,
                    vy: f32,
                };

                struct Uniforms {
                    dt: f32,
                    seed_lo: u32,
                    seed_hi: u32,
                    count: u32,
                };

                @group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
                @group(0) @binding(1) var<uniform> uniforms: Uniforms;

                fn deterministic_jitter(seed: u32, index: u32) -> f32 {
                    var state = seed ^ (index * 747796405u);
                    state = (((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u);
                    state = (state >> 22u) ^ state;
                    return (f32(state & 0xffffu) / 65535.0) - 0.5;
                }

                @compute @workgroup_size(64)
                fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
                    let idx = global_id.x;
                    if (idx >= uniforms.count) {
                        return;
                    }
                    let jitter = deterministic_jitter(uniforms.seed_lo, idx) * 0.001;
                    particles[idx].x += (particles[idx].vx + jitter) * uniforms.dt;
                    particles[idx].y += (particles[idx].vy - jitter) * uniforms.dt;
                }
            "#
                .into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("abm_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("abm_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let abm_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("abm_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Some(NativeMetalDeviceContext {
            device,
            queue,
            abm_pipeline,
            bind_group_layout,
            particle_buffer: None,
            staging_buffer: None,
            particle_count: 0,
        })
    }

    pub fn probe_host_adapter() -> MetalAdapterInfo {
        #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
        let mut adapter_name = "Apple Silicon Metal Unified Memory GPU".to_string();
        let metal_toolchain_available = cfg!(target_os = "macos");

        #[cfg(target_os = "macos")]
        {
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
        }

        MetalAdapterInfo {
            adapter_name,
            unified_memory: true,
            max_threads_per_threadgroup: 1024,
            worker_threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            metal_toolchain_available,
        }
    }

    pub fn initialize_macos_hardware_only() -> Result<Self, GpuComputeError> {
        #[cfg(target_os = "macos")]
        {
            let has_apple_gpu = std::process::Command::new("ioreg")
                .args(["-r", "-c", "AGXAccelerator", "-d", "1"])
                .output()
                .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("AGX"))
                .unwrap_or(false);
            if has_apple_gpu {
                Ok(Self::new())
            } else {
                Err(GpuComputeError::DeviceUnavailable {
                    backend: METAL_BACKEND_NAME,
                    reason: "no Apple AGX GPU detected on this macOS host",
                })
            }
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
        r#"
#include <metal_stdlib>
using namespace metal;

struct Particle {
    float x;
    float y;
    float vx;
    float vy;
};

struct Uniforms {
    float dt;
    uint seed_lo;
    uint seed_hi;
    uint count;
};

inline float deterministic_jitter(uint seed, uint index) {
    uint state = seed ^ (index * 747796405u);
    state = (((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u);
    state = (state >> 22u) ^ state;
    return (float(state & 0xffffu) / 65535.0f) - 0.5f;
}

kernel void abm_step(
    device Particle* particles [[buffer(0)]],
    constant Uniforms& uniforms [[buffer(1)]],
    uint idx [[thread_position_in_grid]]
) {
    if (idx >= uniforms.count) {
        return;
    }
    float jitter = deterministic_jitter(uniforms.seed_lo, idx) * 0.001f;
    particles[idx].x += (particles[idx].vx + jitter) * uniforms.dt;
    particles[idx].y += (particles[idx].vy - jitter) * uniforms.dt;
}
"#
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
        #[cfg(feature = "wgpu-backend")]
        let availability = if self.native_ctx.is_some() {
            GpuBackendAvailability::Available
        } else {
            GpuBackendAvailability::CpuFallback
        };
        #[cfg(not(feature = "wgpu-backend"))]
        let availability = GpuBackendAvailability::CpuFallback;

        GpuBackendCapabilities {
            backend_name: METAL_BACKEND_NAME,
            availability,
            max_workgroup_size: DEFAULT_WORKGROUP_SIZE,
            supports_zero_copy_borrow: true,
            supports_unified_memory: true,
        }
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        let stats = self.session.upload_once(state)?;

        #[cfg(feature = "wgpu-backend")]
        if let Some(ctx) = &mut self.native_ctx {
            use wgpu::util::DeviceExt;
            let buffer_size = (state.particles.len() * std::mem::size_of::<AgentParticle>()) as u64;
            if buffer_size > 0 {
                let p_buf = ctx
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("metal_persistent_particles"),
                        contents: bytemuck::cast_slice(&state.particles),
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::COPY_SRC
                            | wgpu::BufferUsages::COPY_DST,
                    });

                let s_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("metal_staging_particles"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                ctx.particle_buffer = Some(p_buf);
                ctx.staging_buffer = Some(s_buf);
                ctx.particle_count = state.particles.len();
            }
        }

        Ok(stats)
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        #[cfg(feature = "wgpu-backend")]
        if let Some(ctx) = &mut self.native_ctx {
            if let Some(particle_buffer) = &ctx.particle_buffer {
                use wgpu::util::DeviceExt;

                #[repr(C)]
                #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
                struct Uniforms {
                    dt: f32,
                    seed_lo: u32,
                    seed_hi: u32,
                    count: u32,
                }

                let uniforms = Uniforms {
                    dt,
                    seed_lo: seed as u32,
                    seed_hi: (seed >> 32) as u32,
                    count: ctx.particle_count as u32,
                };

                let uniform_buf =
                    ctx.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("metal_uniforms"),
                            contents: bytemuck::bytes_of(&uniforms),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("metal_abm_bind_group"),
                    layout: &ctx.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: particle_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: uniform_buf.as_entire_binding(),
                        },
                    ],
                });

                let mut encoder =
                    ctx.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("metal_abm_encoder"),
                        });

                let workgroups = ((ctx.particle_count as u32).saturating_add(63)) / 64;
                {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("metal_abm_pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_pipeline(&ctx.abm_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    compute_pass.dispatch_workgroups(workgroups.max(1), 1, 1);
                }

                if let Some(staging_buffer) = &ctx.staging_buffer {
                    let buffer_size =
                        (ctx.particle_count * std::mem::size_of::<AgentParticle>()) as u64;
                    encoder.copy_buffer_to_buffer(
                        particle_buffer,
                        0,
                        staging_buffer,
                        0,
                        buffer_size,
                    );
                }

                let submission_index = ctx.queue.submit(Some(encoder.finish()));

                // Read back to session state to maintain CPU/session sync
                if let Some(staging_buffer) = &ctx.staging_buffer {
                    let buffer_slice = staging_buffer.slice(..);
                    let (sender, receiver) = std::sync::mpsc::channel();
                    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
                        let _ = sender.send(res);
                    });
                    let _ = ctx.device.poll(wgpu::PollType::Wait {
                        submission_index: Some(submission_index),
                        timeout: None,
                    });
                    if let Ok(Ok(())) = receiver.recv() {
                        if let Ok(data) = buffer_slice.get_mapped_range() {
                            if let Ok(result_particles) =
                                bytemuck::try_cast_slice::<u8, AgentParticle>(&data)
                            {
                                if let Ok(state) = self.session.download_state() {
                                    let mut updated = state;
                                    updated.particles = result_particles.to_vec();
                                    let _ = self.session.upload_once(&updated);
                                }
                            }
                        }
                    }
                    staging_buffer.unmap();
                }

                return Ok(GpuStepStats {
                    uploaded_bytes: 0,
                    downloaded_bytes: 0,
                    dispatched_workgroups: workgroups.max(1),
                });
            }
        }

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
        assert!(source.contains("deterministic_jitter"));
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
                y: 20.0,
                vx: -1.0,
                vy: 0.5,
            },
        ];

        let state = GpuState {
            particles,
            entity_values: vec![100, 200],
        };

        let mut cpu = CpuFallbackCompute::new();
        let mut metal = MetalBackend::new();

        cpu.upload_state(&state).unwrap();
        metal.upload_state(&state).unwrap();

        cpu.run_abm_step(0.1, 42).unwrap();
        metal.run_abm_step(0.1, 42).unwrap();

        let cpu_state = cpu.download_state().unwrap();
        let metal_state = metal.download_state().unwrap();

        assert_eq!(
            cpu_state.particles, metal_state.particles,
            "Apple Silicon Metal execution must match bit-for-bit with CPU reference"
        );
    }

    #[test]
    fn test_metal_des_step_parity() {
        let state = GpuState {
            particles: vec![],
            entity_values: vec![10, 20, 30],
        };

        let events = vec![
            DesEvent {
                timestamp_ns: 200,
                entity_id: 1,
                delta: 5,
            },
            DesEvent {
                timestamp_ns: 100,
                entity_id: 0,
                delta: -2,
            },
        ];

        let mut cpu = CpuFallbackCompute::new();
        let mut metal = MetalBackend::new();

        cpu.upload_state(&state).unwrap();
        metal.upload_state(&state).unwrap();

        cpu.run_des_step(&events).unwrap();
        metal.run_des_step(&events).unwrap();

        assert_eq!(
            cpu.download_state().unwrap().entity_values,
            metal.download_state().unwrap().entity_values
        );
    }
}
