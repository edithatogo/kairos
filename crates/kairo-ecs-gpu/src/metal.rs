#![forbid(unsafe_code)]

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
use crate::compute::AgentParticle;
use crate::compute::{
    DesEvent, GpuBackendAvailability, GpuBackendCapabilities, GpuCompute, GpuComputeError,
    GpuState, GpuStepStats, PersistentGpuSession, DEFAULT_WORKGROUP_SIZE,
};

pub const METAL_BACKEND_NAME: &str = "apple-metal-unified-compute";

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
const METAL_KERNEL_WIDTH: u32 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetalAdapterInfo {
    pub adapter_name: String,
    pub unified_memory: bool,
    pub max_threads_per_threadgroup: u32,
    pub worker_threads: usize,
    pub metal_toolchain_available: bool,
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
struct NativeMetalDeviceContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    abm_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    adapter_name: String,
    particles: NativeParticleState,
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
enum NativeParticleState {
    Empty,
    Resident(NativeParticles),
    Indeterminate,
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
fn metal_error(reason: &'static str) -> GpuComputeError {
    GpuComputeError::DeviceUnavailable {
        backend: METAL_BACKEND_NAME,
        reason,
    }
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
fn device_operation<T>(
    device: &wgpu::Device,
    reason: &'static str,
    run: impl FnOnce() -> Result<T, GpuComputeError>,
) -> Result<T, GpuComputeError> {
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = run();
    let errors = [
        pollster::block_on(validation.pop()),
        pollster::block_on(memory.pop()),
        pollster::block_on(internal.pop()),
    ];
    if errors.iter().any(Option::is_some) {
        return Err(metal_error(reason));
    }
    result
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
fn upload_buffer(
    device: &wgpu::Device,
    label: &'static str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> Result<wgpu::Buffer, GpuComputeError> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage,
        mapped_at_creation: true,
    });
    let mut view = buffer
        .get_mapped_range_mut(..)
        .map_err(|_| metal_error("could not map the Metal upload buffer"))?;
    view.copy_from_slice(bytes);
    drop(view);
    buffer.unmap();
    Ok(buffer)
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
struct NativeParticles {
    buffer: wgpu::Buffer,
    count: u32,
    workgroups: u32,
}

#[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
impl NativeMetalDeviceContext {
    fn replace_particles(&mut self, particles: &[AgentParticle]) -> Result<usize, GpuComputeError> {
        let bytes = std::mem::size_of_val(particles);
        let count =
            u32::try_from(particles.len()).map_err(|_| GpuComputeError::DispatchSizeOverflow)?;
        let workgroups = count.div_ceil(METAL_KERNEL_WIDTH);
        let limits = self.device.limits();
        if workgroups > limits.max_compute_workgroups_per_dimension {
            return Err(GpuComputeError::DispatchSizeOverflow);
        }
        if bytes as u64
            > limits
                .max_buffer_size
                .min(limits.max_storage_buffer_binding_size)
        {
            return Err(GpuComputeError::MemorySizeOverflow);
        }

        self.particles = if count == 0 {
            NativeParticleState::Empty
        } else {
            device_operation(&self.device, "Metal particle upload failed", || {
                Ok(NativeParticleState::Resident(NativeParticles {
                    buffer: upload_buffer(
                        &self.device,
                        "metal_persistent_particles",
                        bytemuck::cast_slice(particles),
                        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    )?,
                    count,
                    workgroups,
                }))
            })?
        };
        Ok(bytes)
    }

    fn wait_for_submission(
        &self,
        submission: wgpu::SubmissionIndex,
    ) -> Result<(), GpuComputeError> {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .map_err(|_| metal_error("Metal submission did not complete"))?;
        Ok(())
    }

    fn read_particles(
        &self,
        particles: &NativeParticles,
    ) -> Result<Vec<AgentParticle>, GpuComputeError> {
        device_operation(&self.device, "Metal particle readback failed", || {
            const FAILURE: GpuComputeError = GpuComputeError::DeviceUnavailable {
                backend: METAL_BACKEND_NAME,
                reason: "Metal particle readback failed",
            };
            let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("metal_particle_readback"),
                size: particles.buffer.size(),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("metal_particle_readback"),
                });
            encoder.copy_buffer_to_buffer(
                &particles.buffer,
                0,
                &staging,
                0,
                particles.buffer.size(),
            );
            let submitted = self.queue.submit(Some(encoder.finish()));
            let slice = staging.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            self.wait_for_submission(submitted)?;
            receiver
                .recv_timeout(std::time::Duration::from_secs(30))
                .map_err(|_| FAILURE)?
                .map_err(|_| FAILURE)?;
            let view = slice.get_mapped_range().map_err(|_| FAILURE)?;
            let particles = bytemuck::try_cast_slice::<_, AgentParticle>(&view)
                .map_err(|_| FAILURE)?
                .to_vec();
            drop(view);
            staging.unmap();
            Ok(particles)
        })
    }

    fn dispatch_particles(
        &self,
        particles: &NativeParticles,
        dt: f32,
        seed: u64,
    ) -> Result<GpuStepStats, GpuComputeError> {
        device_operation(&self.device, "Metal ABM dispatch failed", || {
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
                count: particles.count,
            };
            let uniform_buffer = upload_buffer(
                &self.device,
                "metal_uniforms",
                bytemuck::bytes_of(&uniforms),
                wgpu::BufferUsages::UNIFORM,
            )?;
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("metal_abm_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particles.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("metal_abm_encoder"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("metal_abm_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.abm_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(particles.workgroups, 1, 1);
            }
            let submitted = self.queue.submit(Some(encoder.finish()));
            self.wait_for_submission(submitted)?;
            Ok(GpuStepStats {
                uploaded_bytes: std::mem::size_of::<Uniforms>(),
                downloaded_bytes: 0,
                dispatched_workgroups: particles.workgroups,
            })
        })
    }
}

/// Optional Apple Metal particle backend with a CPU compatibility constructor.
///
/// Cloning takes an independent CPU snapshot of the current state. A clone
/// panics if the required native readback fails. Use `GpuCompute::download_state`
/// to obtain a snapshot with a typed error on readback failure.
pub struct MetalBackend {
    worker_threads: usize,
    hardware_only: bool,
    session: PersistentGpuSession,
    adapter_info: MetalAdapterInfo,
    #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
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
        #[cfg_attr(
            not(all(target_os = "macos", feature = "wgpu-backend")),
            allow(unused_mut)
        )]
        let mut session = self.session.clone();
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        if self
            .native_ctx
            .as_ref()
            .is_some_and(|ctx| !matches!(ctx.particles, NativeParticleState::Empty))
        {
            let state = self
                .download_state()
                .expect("could not snapshot Metal particles for clone");
            session
                .upload_once(&state)
                .expect("Metal snapshot fits in host state");
        }
        Self {
            worker_threads: self.worker_threads,
            hardware_only: false,
            session,
            adapter_info: MetalAdapterInfo {
                adapter_name: "CPU fallback".into(),
                unified_memory: false,
                max_threads_per_threadgroup: DEFAULT_WORKGROUP_SIZE,
                ..self.adapter_info.clone()
            },
            #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
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
        let worker_threads = worker_threads.max(1);
        let adapter_info = MetalAdapterInfo {
            adapter_name: "CPU fallback".into(),
            unified_memory: false,
            max_threads_per_threadgroup: DEFAULT_WORKGROUP_SIZE,
            worker_threads,
            metal_toolchain_available: cfg!(all(target_os = "macos", feature = "wgpu-backend")),
        };
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        let native_ctx = Self::try_init_native_metal();
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        let adapter_info = match &native_ctx {
            Some(ctx) => MetalAdapterInfo {
                adapter_name: ctx.adapter_name.clone(),
                unified_memory: true,
                max_threads_per_threadgroup: ctx
                    .device
                    .limits()
                    .max_compute_invocations_per_workgroup,
                ..adapter_info
            },
            None => adapter_info,
        };

        Self {
            worker_threads,
            hardware_only: false,
            session: PersistentGpuSession::new(METAL_BACKEND_NAME),
            adapter_info,
            #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
            native_ctx,
        }
    }

    #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
    fn try_init_native_metal() -> Option<NativeMetalDeviceContext> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .ok()?;

        let info = adapter.get_info();
        if info.backend != wgpu::Backend::Metal
            || info.device_type != wgpu::DeviceType::IntegratedGpu
            || !info.name.starts_with("Apple ")
        {
            return None;
        }
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

        let (abm_pipeline, bind_group_layout) =
            device_operation(&device, "Metal ABM pipeline setup failed", || {
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

                let bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

                let pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("abm_pipeline_layout"),
                        bind_group_layouts: &[Some(&bind_group_layout)],
                        immediate_size: 0,
                    });

                let abm_pipeline =
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("abm_compute_pipeline"),
                        layout: Some(&pipeline_layout),
                        module: &shader,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    });

                Ok((abm_pipeline, bind_group_layout))
            })
            .ok()?;

        Some(NativeMetalDeviceContext {
            device,
            queue,
            abm_pipeline,
            bind_group_layout,
            adapter_name: info.name,
            particles: NativeParticleState::Empty,
        })
    }

    /// Return metadata from the selected device or the CPU fallback.
    pub fn probe_host_adapter() -> MetalAdapterInfo {
        Self::new().adapter_info
    }

    /// Select native Apple Metal or return a typed device error.
    ///
    /// This constructor provides particle ABM on the device. DES is
    /// unavailable in hardware-only sessions.
    pub fn initialize_macos_hardware_only() -> Result<Self, GpuComputeError> {
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        {
            let mut backend = Self::new();
            if backend.native_ctx.is_some() {
                backend.hardware_only = true;
                return Ok(backend);
            }
        }
        Err(GpuComputeError::DeviceUnavailable {
            backend: METAL_BACKEND_NAME,
            reason: "no native Apple Metal GPU is available in this build",
        })
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
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        let availability = if self.native_ctx.is_some() {
            GpuBackendAvailability::Available
        } else {
            GpuBackendAvailability::CpuFallback
        };
        #[cfg(not(all(target_os = "macos", feature = "wgpu-backend")))]
        let availability = GpuBackendAvailability::CpuFallback;

        GpuBackendCapabilities {
            backend_name: METAL_BACKEND_NAME,
            availability,
            max_workgroup_size: self.adapter_info.max_threads_per_threadgroup,
            supports_zero_copy_borrow: false,
            supports_unified_memory: availability == GpuBackendAvailability::Available,
        }
    }

    fn upload_state(&mut self, state: &GpuState) -> Result<GpuStepStats, GpuComputeError> {
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        if let Some(ctx) = &mut self.native_ctx {
            let uploaded_bytes = ctx.replace_particles(&state.particles)?;
            self.session.upload_once(&GpuState {
                particles: Vec::new(),
                entity_values: state.entity_values.clone(),
            })?;
            return Ok(GpuStepStats {
                uploaded_bytes,
                ..GpuStepStats::default()
            });
        }
        self.session.upload_once(state)
    }

    fn run_abm_step(&mut self, dt: f32, seed: u64) -> Result<GpuStepStats, GpuComputeError> {
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        if let Some(ctx) = &mut self.native_ctx {
            match &ctx.particles {
                NativeParticleState::Resident(particles) => {
                    let result = ctx.dispatch_particles(particles, dt, seed);
                    if result.is_err() {
                        // The device may have executed the step. Require an explicit upload.
                        ctx.particles = NativeParticleState::Indeterminate;
                    }
                    return result;
                }
                NativeParticleState::Indeterminate => {
                    return Err(metal_error(
                        "upload particle state after a failed Metal submission",
                    ));
                }
                NativeParticleState::Empty => {}
            }
        }

        self.session
            .run_abm_tick_parallel(dt, seed, self.worker_threads)
    }

    fn run_des_step(&mut self, events: &[DesEvent]) -> Result<GpuStepStats, GpuComputeError> {
        if self.hardware_only {
            return Err(GpuComputeError::UnsupportedBackend(METAL_BACKEND_NAME));
        }
        self.session.run_des_tick(events)?;
        Ok(GpuStepStats::default())
    }

    fn download_state(&self) -> Result<GpuState, GpuComputeError> {
        let mut session = self.session.clone();
        #[cfg_attr(
            not(all(target_os = "macos", feature = "wgpu-backend")),
            allow(unused_mut)
        )]
        let mut state = session.download_state()?;
        #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
        if let Some(ctx) = &self.native_ctx {
            match &ctx.particles {
                NativeParticleState::Resident(particles) => {
                    state.particles = ctx.read_particles(particles)?
                }
                NativeParticleState::Indeterminate => {
                    return Err(metal_error(
                        "upload particle state after a failed Metal submission",
                    ));
                }
                NativeParticleState::Empty => {}
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::{AgentParticle, CpuFallbackCompute};

    #[cfg(all(target_os = "macos", feature = "wgpu-backend"))]
    #[test]
    fn invalid_native_buffer_fences_particle_state_until_reupload() {
        let mut backend = MetalBackend::initialize_macos_hardware_only().unwrap();
        let state = GpuState {
            particles: vec![AgentParticle {
                x: 0.0,
                y: 0.0,
                vx: 1.0,
                vy: 0.0,
            }],
            entity_values: Vec::new(),
        };
        backend.upload_state(&state).unwrap();
        let ctx = backend.native_ctx.as_mut().unwrap();
        let NativeParticleState::Resident(particles) = &mut ctx.particles else {
            panic!("particle upload did not create a native buffer");
        };
        // This native buffer lacks STORAGE use. The device must reject the
        // dispatch under the error scope and a fresh upload must recover.
        particles.buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<AgentParticle>() as u64,
            usage: wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        assert!(matches!(
            backend.run_abm_step(0.25, 7),
            Err(GpuComputeError::DeviceUnavailable { .. })
        ));
        assert!(matches!(
            backend.download_state(),
            Err(GpuComputeError::DeviceUnavailable { .. })
        ));
        assert!(matches!(
            backend.run_abm_step(0.25, 7),
            Err(GpuComputeError::DeviceUnavailable { .. })
        ));
        backend.upload_state(&state).unwrap();
        assert_eq!(backend.download_state().unwrap(), state);
        assert_eq!(
            backend.run_abm_step(0.25, 7).unwrap().dispatched_workgroups,
            1
        );
    }

    #[test]
    fn test_metal_backend_capabilities() {
        let backend = MetalBackend::new();
        let caps = backend.capabilities();
        assert_eq!(caps.backend_name, METAL_BACKEND_NAME);
        assert_eq!(
            caps.supports_unified_memory,
            caps.availability == GpuBackendAvailability::Available
        );
        assert!(!caps.supports_zero_copy_borrow);
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
