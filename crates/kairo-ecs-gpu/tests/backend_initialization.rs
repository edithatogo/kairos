#![cfg(all(feature = "wgpu-backend", feature = "cuda-backend"))]
use kairo_ecs_gpu::{
    backends::cuda_backend::{CudaBackend, CUDA_DEVICE_UNAVAILABLE_REASON},
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
fn cuda_init_reports_runtime_unavailable_without_cpu_fallback() {
    assert_eq!(
        CudaBackend::initialize_required_context(),
        Err(GpuComputeError::DeviceUnavailable {
            backend: CUDA_BACKEND_NOT_CONFIGURED,
            reason: CUDA_DEVICE_UNAVAILABLE_REASON,
        })
    );

    let report = CudaBackend::initialization_report();
    assert_eq!(report.backend, NativeGpuBackendKind::Cuda);
    assert!(report.attempted_real_device);
    assert!(!report.available);
    assert_eq!(report.reason, CUDA_DEVICE_UNAVAILABLE_REASON);
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
