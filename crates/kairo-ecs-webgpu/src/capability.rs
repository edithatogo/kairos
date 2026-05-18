use crate::adapter::{detect_adapter, AdapterStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputeBackend {
    BrowserWebGpu,
    CpuFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParityStatus {
    ReferenceOnly,
    PendingBrowserDispatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserCapability {
    pub adapter_status: AdapterStatus,
    pub requested_backend: ComputeBackend,
    pub effective_backend: ComputeBackend,
    pub parity_status: ParityStatus,
    pub workgroup_size: u32,
    pub browser_gpu_required_for_validation: bool,
}

pub const WEBGPU_WORKGROUP_SIZE: u32 = 256;

pub fn browser_capability(requested_backend: ComputeBackend) -> BrowserCapability {
    let adapter_status = detect_adapter();
    let effective_backend = match (requested_backend, adapter_status) {
        (ComputeBackend::BrowserWebGpu, AdapterStatus::BrowserApiDetected) => {
            ComputeBackend::BrowserWebGpu
        }
        _ => ComputeBackend::CpuFallback,
    };
    let parity_status = match effective_backend {
        ComputeBackend::BrowserWebGpu => ParityStatus::PendingBrowserDispatch,
        ComputeBackend::CpuFallback => ParityStatus::ReferenceOnly,
    };

    BrowserCapability {
        adapter_status,
        requested_backend,
        effective_backend,
        parity_status,
        workgroup_size: WEBGPU_WORKGROUP_SIZE,
        browser_gpu_required_for_validation: effective_backend == ComputeBackend::BrowserWebGpu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_cpu_fallback_until_browser_bindings_are_configured() {
        let capability = browser_capability(ComputeBackend::BrowserWebGpu);

        assert_eq!(
            capability.adapter_status,
            AdapterStatus::BrowserBindingsNotConfigured
        );
        assert_eq!(capability.effective_backend, ComputeBackend::CpuFallback);
        assert_eq!(capability.parity_status, ParityStatus::ReferenceOnly);
        assert_eq!(capability.workgroup_size, WEBGPU_WORKGROUP_SIZE);
        assert!(!capability.browser_gpu_required_for_validation);
    }
}
