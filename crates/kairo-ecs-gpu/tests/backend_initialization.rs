#![cfg(all(feature = "wgpu-backend", feature = "cuda-backend"))]
use kairo_ecs_gpu::{
    backends::cuda_backend::{CudaBackend, CUDA_BACKEND_NAME, CUDA_DEVICE_UNAVAILABLE_REASON},
    wgpu::backend::{WgpuBackend, WGPU_DEVICE_UNAVAILABLE_REASON},
    GpuComputeError, NativeGpuBackendKind, CUDA_BACKEND_NOT_CONFIGURED,
    WGPU_BACKEND_NOT_CONFIGURED,
};

#[test]
fn wgpu_init_reports_device_unavailable_without_cpu_fallback() {
    assert_eq!(
        WgpuBackend::initialize_required_device(),
        Err(GpuComputeError::DeviceUnavailable {
            backend: WGPU_BACKEND_NOT_CONFIGURED,
            reason: WGPU_DEVICE_UNAVAILABLE_REASON,
        })
    );

    let report = WgpuBackend::initialization_report();
    assert_eq!(report.backend, NativeGpuBackendKind::Wgpu);
    assert!(report.attempted_real_device);
    assert!(!report.available);
    assert_eq!(report.reason, WGPU_DEVICE_UNAVAILABLE_REASON);
}

#[test]
fn cuda_initialization_reports_the_observed_driver_state() {
    let report = CudaBackend::initialization_report();
    assert_eq!(report.backend, NativeGpuBackendKind::Cuda);
    assert!(report.attempted_real_device);
    let backend = CudaBackend::initialize_required_context();
    assert_eq!(backend.is_ok(), report.available);
    if let Err(error) = backend {
        assert_ne!(report.reason, CUDA_DEVICE_UNAVAILABLE_REASON);
        assert_eq!(
            error,
            GpuComputeError::DeviceUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: report.reason,
            }
        );
    }
}

#[test]
fn backend_errors_distinguish_not_configured_from_no_device() {
    assert_eq!(
        NativeGpuBackendKind::Wgpu.backend_name(),
        WGPU_BACKEND_NOT_CONFIGURED
    );
    assert_eq!(
        NativeGpuBackendKind::Cuda.backend_name(),
        CUDA_BACKEND_NOT_CONFIGURED
    );
    assert_ne!(WGPU_BACKEND_NOT_CONFIGURED, WGPU_DEVICE_UNAVAILABLE_REASON);
    assert_ne!(CUDA_BACKEND_NOT_CONFIGURED, CUDA_DEVICE_UNAVAILABLE_REASON);
}
