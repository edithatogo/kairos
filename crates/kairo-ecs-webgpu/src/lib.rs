//! Browser WebGPU compute scaffold for KairoECS.
//!
//! The current crate is dependency-free so it can be validated in CPU-only CI.
//! Browser bindings are introduced behind the `webgpu` feature in a later task.

pub mod adapter;
pub mod bridge;
pub mod dispatch;

pub use adapter::{is_webgpu_available, AdapterStatus};
pub use bridge::{BufferBridge, BufferBridgeError, BufferDescriptor};
pub use dispatch::{
    run_reference_step, try_run_browser_webgpu_step, AgentSnapshot, DispatchStats,
    WebGpuDispatchError,
};

pub fn init_webgpu() -> AdapterStatus {
    adapter::detect_adapter()
}

pub fn get_result_buffer(agents: &[AgentSnapshot]) -> Vec<f32> {
    agents
        .iter()
        .flat_map(|agent| [agent.x, agent.y, agent.vx, agent.vy])
        .collect()
}
