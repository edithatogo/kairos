#![allow(unsafe_code)]

//! CUDA execution bridge for KairoECS.
//!
//! Encapsulates low-level CUDA driver calls, memory transfers, and PTX kernel dispatch.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CudaParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

#[cfg(feature = "cuda")]
use cudarc::driver::PushKernelArg;

#[cfg(feature = "cuda")]
unsafe impl cudarc::driver::DeviceRepr for CudaParticle {}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CudaEntityDelta {
    pub entity: u32,
    pub delta: i32,
}

#[cfg(feature = "cuda")]
unsafe impl cudarc::driver::DeviceRepr for CudaEntityDelta {}

pub const ABM_STEP_PTX: &str = r#"
.version 7.0
.target sm_75
.address_size 64

.visible .entry abm_step_kernel(
    .param .u64 particles_ptr,
    .param .u32 particle_count,
    .param .f32 dt,
    .param .u64 seed
) {
    .reg .pred %p0;
    .reg .b32 %r0, %r1, %r2, %r3, %r_count;
    .reg .b32 %r_seed, %r_state, %r_shift, %r_mix, %r_tmp;
    .reg .b64 %rd1, %rd_ptr, %rd_offset;
    .reg .f32 %f_dt, %f_x, %f_y, %f_vx, %f_vy, %f_nx, %f_ny, %f_jitter;

    mov.u32 %r0, %ctaid.x;
    mov.u32 %r1, %ntid.x;
    mov.u32 %r2, %tid.x;
    mad.lo.u32 %r3, %r0, %r1, %r2;

    ld.param.u32 %r_count, [particle_count];
    setp.ge.u32 %p0, %r3, %r_count;
    @%p0 ret;

    ld.param.u64 %rd_ptr, [particles_ptr];
    ld.param.f32 %f_dt, [dt];

    // Each particle is 4 f32s (16 bytes): x, y, vx, vy
    mul.wide.u32 %rd_offset, %r3, 16;
    add.u64 %rd1, %rd_ptr, %rd_offset;

    // Load x, y, vx, vy
    ld.global.f32 %f_x, [%rd1];
    ld.global.f32 %f_y, [%rd1+4];
    ld.global.f32 %f_vx, [%rd1+8];
    ld.global.f32 %f_vy, [%rd1+12];

    // Match the CPU seed and particle index hash, using the low 32 seed bits.
    ld.param.u32 %r_seed, [seed];
    mul.lo.u32 %r_state, %r3, 747796405;
    xor.b32 %r_state, %r_state, %r_seed;
    shr.u32 %r_shift, %r_state, 28;
    add.u32 %r_shift, %r_shift, 4;
    shr.u32 %r_mix, %r_state, %r_shift;
    xor.b32 %r_mix, %r_mix, %r_state;
    mul.lo.u32 %r_mix, %r_mix, 277803737;
    shr.u32 %r_tmp, %r_mix, 22;
    xor.b32 %r_mix, %r_mix, %r_tmp;
    and.b32 %r_mix, %r_mix, 65535;
    cvt.rn.f32.u32 %f_jitter, %r_mix;
    div.rn.f32 %f_jitter, %f_jitter, 0f477FFF00;
    sub.rn.f32 %f_jitter, %f_jitter, 0f3F000000;
    mul.rn.f32 %f_jitter, %f_jitter, 0f3A83126F;
    add.rn.f32 %f_vx, %f_vx, %f_jitter;
    sub.rn.f32 %f_vy, %f_vy, %f_jitter;
    fma.rn.f32 %f_nx, %f_vx, %f_dt, %f_x;
    fma.rn.f32 %f_ny, %f_vy, %f_dt, %f_y;

    // Store back
    st.global.f32 [%rd1], %f_nx;
    st.global.f32 [%rd1+4], %f_ny;

    ret;
}

.visible .entry des_add_kernel(
    .param .u64 values_ptr,
    .param .u64 events_ptr,
    .param .u32 count
) {
    .reg .pred %outside;
    .reg .b32 %block, %width, %thread, %i, %count, %entity, %delta, %prior;
    .reg .b64 %values, %events, %offset, %event, %value;
    mov.u32 %block, %ctaid.x;
    mov.u32 %width, %ntid.x;
    mov.u32 %thread, %tid.x;
    mad.lo.u32 %i, %block, %width, %thread;
    ld.param.u32 %count, [count];
    setp.ge.u32 %outside, %i, %count;
    @%outside ret;

    ld.param.u64 %events, [events_ptr];
    ld.param.u64 %values, [values_ptr];
    mul.wide.u32 %offset, %i, 8;
    add.u64 %event, %events, %offset;
    ld.global.u32 %entity, [%event];
    ld.global.s32 %delta, [%event+4];
    mul.wide.u32 %offset, %entity, 4;
    add.u64 %value, %values, %offset;
    atom.global.add.s32 %prior, [%value], %delta;
    ret;
}
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CudaDeviceInfo {
    pub device_name: String,
    pub total_memory_bytes: usize,
    pub compute_capability: (i32, i32),
}

#[cfg(feature = "cuda")]
enum ParticleState {
    AwaitingUpload,
    Resident(u32),
    Indeterminate,
}

#[cfg(feature = "cuda")]
pub struct CudaDeviceSession {
    info: CudaDeviceInfo,
    stream: std::sync::Arc<cudarc::driver::safe::CudaStream>,
    func: cudarc::driver::safe::CudaFunction,
    des_func: cudarc::driver::safe::CudaFunction,
    entity_buffer: Option<cudarc::driver::safe::CudaSlice<i32>>,
    entity_count: Option<u32>,
    particle_buffer: Option<cudarc::driver::safe::CudaSlice<CudaParticle>>,
    particle_state: ParticleState,
}

#[cfg(feature = "cuda")]
impl CudaDeviceSession {
    pub fn new(device_ordinal: usize) -> Result<Self, String> {
        let result =
            std::panic::catch_unwind(|| cudarc::driver::safe::CudaContext::new(device_ordinal));
        let ctx = match result {
            Ok(Ok(ctx)) => ctx,
            Ok(Err(e)) => {
                return Err(format!(
                    "failed to initialize CUDA context on device {device_ordinal}: {e:?}"
                ))
            }
            Err(_) => {
                return Err(
                    "CUDA driver dynamic library unavailable or panicked during initialization"
                        .to_string(),
                )
            }
        };
        let info = CudaDeviceInfo {
            device_name: ctx
                .name()
                .map_err(|error| format!("CUDA device name query failed: {error:?}"))?,
            total_memory_bytes: ctx
                .total_mem()
                .map_err(|error| format!("CUDA device memory query failed: {error:?}"))?,
            compute_capability: ctx
                .compute_capability()
                .map_err(|error| format!("CUDA capability query failed: {error:?}"))?,
        };
        let stream = ctx.default_stream();
        let module = ctx
            .load_module(cudarc::nvrtc::Ptx::from_src(ABM_STEP_PTX))
            .map_err(|e| format!("failed to compile/load PTX module: {e:?}"))?;
        let func = module
            .load_function("abm_step_kernel")
            .map_err(|e| format!("failed to load abm_step_kernel function: {e:?}"))?;

        let des_func = module
            .load_function("des_add_kernel")
            .map_err(|error| format!("failed to load the CUDA DES kernel: {error:?}"))?;
        Ok(Self {
            info,
            stream,
            func,
            des_func,
            entity_buffer: None,
            entity_count: None,
            particle_buffer: None,
            particle_state: ParticleState::AwaitingUpload,
        })
    }

    pub fn device_info(&self) -> CudaDeviceInfo {
        self.info.clone()
    }

    pub fn upload_particles(&mut self, particles: &[CudaParticle]) -> Result<usize, String> {
        let count = u32::try_from(particles.len()).map_err(|_| "too many CUDA particles")?;
        let buffer = if particles.is_empty() {
            None
        } else {
            Some(
                self.stream
                    .clone_htod(particles)
                    .map_err(|error| format!("CUDA particle upload failed: {error:?}"))?,
            )
        };
        self.stream
            .synchronize()
            .map_err(|error| format!("CUDA particle upload sync failed: {error:?}"))?;
        self.particle_buffer = buffer;
        self.particle_state = ParticleState::Resident(count);
        Ok(std::mem::size_of_val(particles))
    }

    pub fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String> {
        let count = match self.particle_state {
            ParticleState::AwaitingUpload | ParticleState::Resident(0) => return Ok(0),
            ParticleState::Resident(count) => count,
            ParticleState::Indeterminate => {
                return Err("upload particle state after a failed CUDA submission".into())
            }
        };

        let d_buf = self
            .particle_buffer
            .as_mut()
            .ok_or("no particles uploaded")?;
        let mut launcher = self.stream.launch_builder(&self.func);
        launcher.arg(d_buf);
        launcher.arg(&count);
        launcher.arg(&dt);
        launcher.arg(&seed);

        let workgroup_size = 256u32;
        let blocks = count.div_ceil(workgroup_size);
        let cfg = cudarc::driver::safe::LaunchConfig {
            grid_dim: (blocks, 1, 1),
            block_dim: (workgroup_size, 1, 1),
            shared_mem_bytes: 0,
        };

        // The validated particle count and buffer stay valid through synchronization.
        let result = unsafe { launcher.launch(cfg) }.and_then(|_| self.stream.synchronize());
        if let Err(error) = result {
            self.particle_state = ParticleState::Indeterminate;
            self.entity_count = None;
            return Err(format!("CUDA particle kernel failed: {error:?}"));
        }

        Ok(blocks)
    }

    /// Upload a complete model and retain the old buffers until both copies finish.
    pub fn upload_model(
        &mut self,
        particles: &[CudaParticle],
        entities: &[i32],
    ) -> Result<usize, String> {
        let particle_count =
            u32::try_from(particles.len()).map_err(|_| "too many CUDA particles")?;
        let entity_count = u32::try_from(entities.len()).map_err(|_| "too many CUDA entities")?;
        let bytes = std::mem::size_of_val(particles)
            .checked_add(std::mem::size_of_val(entities))
            .ok_or("CUDA upload byte size overflow")?;
        let staged = (|| {
            let particles = if particles.is_empty() {
                None
            } else {
                Some(
                    self.stream
                        .clone_htod(particles)
                        .map_err(|error| format!("CUDA particle upload failed: {error:?}"))?,
                )
            };
            let entities = if entities.is_empty() {
                None
            } else {
                Some(
                    self.stream
                        .clone_htod(entities)
                        .map_err(|error| format!("CUDA entity upload failed: {error:?}"))?,
                )
            };
            Ok::<_, String>((particles, entities))
        })();
        self.stream
            .synchronize()
            .map_err(|error| format!("CUDA model upload sync failed: {error:?}"))?;
        let (particle_buffer, entity_buffer) = staged?;
        self.particle_buffer = particle_buffer;
        self.particle_state = ParticleState::Resident(particle_count);
        self.entity_buffer = entity_buffer;
        self.entity_count = Some(entity_count);
        Ok(bytes)
    }

    pub fn resident_entity_count(&self) -> Option<usize> {
        self.entity_count.map(|count| count as usize)
    }

    pub fn step_des(&mut self, events: &[CudaEntityDelta]) -> Result<u32, String> {
        let entity_count = self
            .entity_count
            .ok_or("upload CUDA entity state before dispatch")?;
        if let Some(event) = events.iter().find(|event| event.entity >= entity_count) {
            return Err(format!(
                "CUDA entity index {} is out of range",
                event.entity
            ));
        }
        if events.is_empty() {
            return Ok(0);
        }
        let count = u32::try_from(events.len()).map_err(|_| "too many CUDA DES events")?;
        let events = self
            .stream
            .clone_htod(events)
            .map_err(|error| format!("CUDA DES event upload failed: {error:?}"))?;
        let grid = count.div_ceil(256);
        let config = cudarc::driver::safe::LaunchConfig {
            grid_dim: (grid, 1, 1),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        let values = self
            .entity_buffer
            .as_mut()
            .expect("validated resident entity buffer");
        let mut launch = self.stream.launch_builder(&self.des_func);
        launch.arg(values);
        launch.arg(&events);
        launch.arg(&count);
        // Both slices remain live until the synchronized launch finishes.
        let result = unsafe { launch.launch(config) }.and_then(|_| self.stream.synchronize());
        if let Err(error) = result {
            self.entity_count = None;
            return Err(format!("CUDA DES kernel failed: {error:?}"));
        }
        Ok(grid)
    }

    pub fn download_entities(&self) -> Result<Vec<i32>, String> {
        let count = self
            .entity_count
            .ok_or("CUDA entity state is unavailable")?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let buffer = self.entity_buffer.as_ref().expect("resident entity buffer");
        self.readback(buffer)
    }

    pub fn download_particles(&self) -> Result<Vec<CudaParticle>, String> {
        match self.particle_state {
            ParticleState::Resident(0) => return Ok(Vec::new()),
            ParticleState::Resident(_) => {}
            ParticleState::AwaitingUpload => return Err("no particles uploaded".into()),
            ParticleState::Indeterminate => {
                return Err("upload particle state after a failed CUDA submission".into())
            }
        }
        let buffer = self
            .particle_buffer
            .as_ref()
            .ok_or("no particles uploaded")?;
        self.readback(buffer)
    }

    fn readback<T: cudarc::driver::DeviceRepr>(
        &self,
        buffer: &cudarc::driver::safe::CudaSlice<T>,
    ) -> Result<Vec<T>, String> {
        let host = self
            .stream
            .clone_dtoh(buffer)
            .map_err(|error| format!("CUDA readback failed: {error:?}"))?;
        self.stream
            .synchronize()
            .map_err(|error| format!("CUDA readback sync failed: {error:?}"))?;
        Ok(host)
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native NVIDIA CUDA device"]
    fn indeterminate_particles_require_reupload() {
        let mut device = CudaDeviceSession::new(0).unwrap();
        let particle = CudaParticle {
            x: 0.0,
            y: 1.0,
            vx: 0.5,
            vy: 0.25,
        };
        device.upload_particles(&[particle]).unwrap();
        device.particle_state = ParticleState::Indeterminate;
        assert!(device.step_abm(0.25, 7).is_err());
        assert!(device.download_particles().is_err());
        device.upload_particles(&[particle]).unwrap();
        assert_eq!(device.step_abm(0.25, 7).unwrap(), 1);
        assert_eq!(device.download_particles().unwrap().len(), 1);
    }
}
