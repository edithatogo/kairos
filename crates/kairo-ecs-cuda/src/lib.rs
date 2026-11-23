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
mod session;

#[cfg(feature = "cuda")]
pub use session::CudaDeviceSession;
