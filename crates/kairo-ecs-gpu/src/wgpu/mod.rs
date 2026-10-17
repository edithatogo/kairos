//! wgpu hardware and software device compute module for KairoECS.

pub mod backend;
pub mod device;

pub use backend::WgpuBackend;
pub use device::*;
