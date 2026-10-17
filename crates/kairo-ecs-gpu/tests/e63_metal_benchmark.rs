#![forbid(unsafe_code)]

use kairo_ecs_gpu::benchmark::{
    benchmark_batch_transit, benchmark_candidate_scoring, compute_percentiles,
    evaluate_mlx_comparative_adr, AccelerationStatus, WorkloadBenchmarkKind,
    MLX_COMPARATIVE_ADR_ID,
};
use kairo_ecs_gpu::wgpu::WgpuDeviceCompute;

#[test]
fn test_01_percentile_computation_and_distribution() {
    let mut samples = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
    let p = compute_percentiles(&mut samples);

    assert_eq!(p.min_ns, 100);
    assert_eq!(p.median_ns, 600);
    assert_eq!(p.max_ns, 1000);
    assert!(p.p95_ns >= 900);
    assert!(p.p25_ns <= 400);
}

#[test]
fn test_02_batch_transit_benchmark_stages_and_parity() {
    let report = benchmark_batch_transit(512, 10, true)
        .expect("batch transit benchmark should execute cleanly");

    assert_eq!(report.workload, WorkloadBenchmarkKind::BatchTransit);
    assert_eq!(report.batch_size, 512);
    assert!(report.stages.is_valid());
    assert!(report.stages.total_e2e_ns > 0);
    assert!(report.stages.staging_upload_ns > 0);
    assert!(report.stages.sync_readback_ns > 0);
    assert!(report.cpu_distribution.median_ns > 0);
    assert!(report.device_distribution.median_ns > 0);
    assert!(report.queue_arbitration_verified);
    assert_eq!(
        report.status,
        AccelerationStatus::HardwareUnavailableFallback
    );
    assert!(report.resident_chains.len() >= 4);

    // Resident chain speeds up with k iterations
    let k1 = report
        .resident_chains
        .iter()
        .find(|c| c.iterations == 1)
        .unwrap();
    let k50 = report
        .resident_chains
        .iter()
        .find(|c| c.iterations == 50)
        .unwrap();
    assert!(
        k50.speedup > k1.speedup,
        "Resident chains must amortize staging overhead"
    );
}

#[test]
fn test_03_candidate_scoring_benchmark_and_resident_chains() {
    let report = benchmark_candidate_scoring(256, 16, 10, true)
        .expect("candidate scoring benchmark should execute cleanly");

    assert_eq!(report.workload, WorkloadBenchmarkKind::CandidateScoring);
    assert_eq!(report.batch_size, 256 * 16);
    assert!(report.stages.is_valid());
    assert!(report.cpu_distribution.median_ns > 0);
    assert!(report.device_distribution.median_ns > 0);
    assert!(report.queue_arbitration_verified);
    assert!(report.host_memory_bytes > 0);
    assert!(report.device_memory_bytes > 0);
}

#[test]
fn test_04_experimental_status_retention_when_speedup_sub_threshold() {
    // Force fallback/emulated execution where transfer dominates and speedup is evaluated
    let report =
        benchmark_batch_transit(64, 10, false).expect("batch transit benchmark should run");

    // Regardless of whether running on Apple Metal or emulated CPU, the invariant holds:
    // If speedup < 1.05 and not hardware, it must report accurately without false claims.
    if report.is_device_available {
        if report.speedup_e2e < 1.05 && report.resident_chains.iter().all(|c| !c.material_gain) {
            assert_eq!(
                report.status,
                AccelerationStatus::ExperimentalStatusRetained
            );
            assert!(report
                .findings_summary
                .contains("experimental status is retained"));
        } else {
            assert_eq!(report.status, AccelerationStatus::MaterialGain);
        }
    } else {
        assert_eq!(
            report.status,
            AccelerationStatus::HardwareUnavailableFallback
        );
    }
}

#[test]
fn test_05_mlx_comparative_adr_evaluation() {
    let summary = evaluate_mlx_comparative_adr();
    assert_eq!(summary.adr_id, MLX_COMPARATIVE_ADR_ID);
    assert!(summary.rust_wgpu_safe);
    assert!(summary.queue_arbitration_cpu_owned);
    assert!(summary.cross_platform_support.contains(&"macOS (Metal)"));
    assert!(summary.cross_platform_support.contains(&"Linux (Vulkan)"));
    assert!(summary.decision.contains("Retain Rust-native"));
}

#[test]
fn test_06_cpu_queue_arbitration_invariance_under_benchmark() {
    let compute = WgpuDeviceCompute::new_with_host_probe();
    assert!(
        compute.is_queue_arbitration_cpu_owned(),
        "Queue arbitration MUST remain CPU-owned"
    );
}
