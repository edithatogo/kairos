use crate::capability::WEBGPU_WORKGROUP_SIZE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentSnapshot {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchStats {
    pub workgroups: u32,
    pub uploaded_bytes: usize,
    pub downloaded_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WebGpuDispatchError {
    BrowserBackendNotConfigured,
}

pub fn try_run_browser_webgpu_step(
    _agents: &mut [AgentSnapshot],
    _dt: f32,
    _seed: u32,
) -> Result<DispatchStats, WebGpuDispatchError> {
    Err(WebGpuDispatchError::BrowserBackendNotConfigured)
}

pub fn run_reference_step(agents: &mut [AgentSnapshot], dt: f32, seed: u32) -> DispatchStats {
    for (index, agent) in agents.iter_mut().enumerate() {
        let jitter = pcg_jitter(seed, index as u32) * 0.001;
        agent.x += (agent.vx + jitter) * dt;
        agent.y += (agent.vy - jitter) * dt;
    }

    DispatchStats {
        workgroups: ((agents.len() as u32).saturating_add(WEBGPU_WORKGROUP_SIZE - 1))
            / WEBGPU_WORKGROUP_SIZE,
        uploaded_bytes: agents.len() * core::mem::size_of::<AgentSnapshot>(),
        downloaded_bytes: agents.len() * core::mem::size_of::<AgentSnapshot>(),
    }
}

fn pcg_jitter(seed: u32, index: u32) -> f32 {
    let mut state = seed ^ index.wrapping_mul(747_796_405);
    state = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    state = (state >> 22) ^ state;
    ((state & 65_535) as f32 / 65_535.0) - 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_uses_browser_safe_workgroup_size() {
        let mut agents = vec![
            AgentSnapshot {
                x: 0.0,
                y: 0.0,
                vx: 1.0,
                vy: 1.0,
            };
            257
        ];

        let stats = run_reference_step(&mut agents, 0.016, 7);

        assert_eq!(stats.workgroups, 2);
    }

    #[test]
    fn browser_dispatch_reports_backend_not_configured() {
        let mut agents = [];

        assert_eq!(
            try_run_browser_webgpu_step(&mut agents, 0.016, 7),
            Err(WebGpuDispatchError::BrowserBackendNotConfigured)
        );
    }
}
