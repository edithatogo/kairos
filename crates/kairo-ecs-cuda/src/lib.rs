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
    .reg .b64 %rd0, %rd1, %rd_ptr, %rd_offset;
    .reg .f32 %f_dt, %f_x, %f_y, %f_vx, %f_vy, %f_nx, %f_ny;

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
    cvt.u64.u32 %rd0, %r3;
    mul.wide.u32 %rd_offset, %r3, 16;
    add.u64 %rd1, %rd_ptr, %rd_offset;

    // Load x, y, vx, vy
    ld.global.f32 %f_x, [%rd1];
    ld.global.f32 %f_y, [%rd1+4];
    ld.global.f32 %f_vx, [%rd1+8];
    ld.global.f32 %f_vy, [%rd1+12];

    // x += vx * dt; y += vy * dt
    fma.rn.f32 %f_nx, %f_vx, %f_dt, %f_x;
    fma.rn.f32 %f_ny, %f_vy, %f_dt, %f_y;

    // Store back
    st.global.f32 [%rd1], %f_nx;
    st.global.f32 [%rd1+4], %f_ny;

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
pub struct CudaDeviceSession {
    ctx: std::sync::Arc<cudarc::driver::safe::CudaContext>,
    stream: std::sync::Arc<cudarc::driver::safe::CudaStream>,
    func: cudarc::driver::safe::CudaFunction,
    particle_buffer: Option<cudarc::driver::safe::CudaSlice<CudaParticle>>,
    particle_count: usize,
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
        let stream = ctx.default_stream();
        let module = ctx
            .load_module(cudarc::nvrtc::Ptx::from_src(ABM_STEP_PTX))
            .map_err(|e| format!("failed to compile/load PTX module: {e:?}"))?;
        let func = module
            .load_function("abm_step_kernel")
            .map_err(|e| format!("failed to load abm_step_kernel function: {e:?}"))?;

        Ok(Self {
            ctx,
            stream,
            func,
            particle_buffer: None,
            particle_count: 0,
        })
    }

    pub fn device_info(&self) -> CudaDeviceInfo {
        let name = self.ctx.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());
        CudaDeviceInfo {
            device_name: name,
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            compute_capability: (7, 5),
        }
    }

    pub fn upload_particles(&mut self, particles: &[CudaParticle]) -> Result<usize, String> {
        let d_buf = self
            .stream
            .clone_htod(particles)
            .map_err(|e| format!("HTOD copy failed: {e:?}"))?;
        let count = particles.len();
        self.particle_buffer = Some(d_buf);
        self.particle_count = count;
        Ok(count * std::mem::size_of::<CudaParticle>())
    }

    pub fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String> {
        if self.particle_count == 0 {
            return Ok(0);
        }

        let d_buf = self
            .particle_buffer
            .as_mut()
            .ok_or("no particles uploaded")?;
        let count = self.particle_count as u32;

        let mut launcher = self.stream.launch_builder(&self.func);
        launcher.arg(d_buf);
        launcher.arg(&count);
        launcher.arg(&dt);
        launcher.arg(&seed);

        let workgroup_size = 256u32;
        let blocks = (count + workgroup_size - 1) / workgroup_size;
        let cfg = cudarc::driver::safe::LaunchConfig {
            grid_dim: (blocks, 1, 1),
            block_dim: (workgroup_size, 1, 1),
            shared_mem_bytes: 0,
        };

        unsafe {
            launcher
                .launch(cfg)
                .map_err(|e| format!("kernel launch failed: {e:?}"))?;
        }
        self.stream
            .synchronize()
            .map_err(|e| format!("stream sync failed: {e:?}"))?;

        Ok(blocks)
    }

    pub fn download_particles(&self) -> Result<Vec<CudaParticle>, String> {
        let d_buf = self
            .particle_buffer
            .as_ref()
            .ok_or("no particles uploaded")?;
        self.stream
            .clone_dtoh(d_buf)
            .map_err(|e| format!("DTOH copy failed: {e:?}"))
    }
}
