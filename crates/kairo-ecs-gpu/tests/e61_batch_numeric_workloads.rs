//! Integration test suite for the 26 Report 26 numeric workload candidates and data economics.
//!
//! Verifies:
//! - Full 26 candidate matrix spanning K-ELT through K-SORT-FULL across batch sizes N in [64, 1048576]
//! - Explicit separation of K-W1-PRESORT and K-W1-WEIGHTED / K-SORT-FULL
//! - Break-even formulations S_e2e(N) and S_resident(N, k)
//! - Hardware fallback and capability reporting with zero unhandled panics

use kairo_ecs_gpu::compute::{
    CpuFallbackCompute, GpuBackendAvailability, GpuCompute, GpuComputeError, GpuState,
};
use kairo_ecs_gpu::numeric_batch::{
    report_26_workload_catalog, CostModelProfile, NumericBatchExecutor, NumericWorkloadKind,
};

#[test]
fn test_report_26_catalog_contains_exactly_26_candidates() {
    let catalog = report_26_workload_catalog();
    assert_eq!(
        catalog.len(),
        26,
        "Report 26 matrix must contain exactly 26 workload candidates"
    );

    // Verify all 9 kernel families are present
    let mut kinds_found = std::collections::HashSet::new();
    for cand in &catalog {
        kinds_found.insert(cand.kind);
        assert!(!cand.id.is_empty());
        assert!(cand.batch_size >= 64, "Minimum batch size is 64");
        assert!(cand.batch_size <= 1_048_576, "Maximum batch size is 1M");
    }

    assert_eq!(
        kinds_found.len(),
        9,
        "All 9 kernel families must be represented"
    );
    assert!(kinds_found.contains(&NumericWorkloadKind::KElt));
    assert!(kinds_found.contains(&NumericWorkloadKind::KW1Presort));
    assert!(kinds_found.contains(&NumericWorkloadKind::KW1Weighted));
    assert!(kinds_found.contains(&NumericWorkloadKind::KKsCdf));
    assert!(kinds_found.contains(&NumericWorkloadKind::KHist));
    assert!(kinds_found.contains(&NumericWorkloadKind::KScan));
    assert!(kinds_found.contains(&NumericWorkloadKind::KCand));
    assert!(kinds_found.contains(&NumericWorkloadKind::KTransit));
    assert!(kinds_found.contains(&NumericWorkloadKind::KSortFull));
}

#[test]
fn test_sorting_cost_separation_in_catalog() {
    let catalog = report_26_workload_catalog();
    let presort_candidates: Vec<_> = catalog
        .iter()
        .filter(|c| c.kind == NumericWorkloadKind::KW1Presort)
        .collect();
    let weighted_candidates: Vec<_> = catalog
        .iter()
        .filter(|c| c.kind == NumericWorkloadKind::KW1Weighted)
        .collect();
    let sort_full_candidates: Vec<_> = catalog
        .iter()
        .filter(|c| c.kind == NumericWorkloadKind::KSortFull)
        .collect();

    assert_eq!(presort_candidates.len(), 4);
    assert_eq!(weighted_candidates.len(), 3);
    assert_eq!(sort_full_candidates.len(), 1);

    // Confirm that sort_full isolates sorting cost
    assert_eq!(sort_full_candidates[0].id, "CAND-26");
    assert_eq!(sort_full_candidates[0].name, "Full Sort and W1 Pipeline");
}

#[test]
fn test_break_even_formulations() {
    let profile = CostModelProfile::default();
    let catalog = report_26_workload_catalog();

    // CAND-01: Elementwise N=1024 (small) -> CPU overhead bound
    let cand_small = &catalog[0];
    let breakdown_small = profile.compute_breakdown(cand_small);
    assert!(
        breakdown_small.speedup_e2e < 1.0,
        "Small elementwise batch S_e2e < 1.0 (got {})",
        breakdown_small.speedup_e2e
    );

    // CAND-03: Elementwise N=1M (large) -> GPU accelerated
    let cand_large = &catalog[2];
    let breakdown_large = profile.compute_breakdown(cand_large);
    assert!(
        breakdown_large.speedup_e2e > 1.0,
        "Large elementwise batch S_e2e > 1.0 (got {})",
        breakdown_large.speedup_e2e
    );

    // CAND-24: Spatial Transit N=16384 -> Chosen kernel with measurable E2E benefit
    let cand_transit = &catalog[23];
    assert_eq!(cand_transit.kind, NumericWorkloadKind::KTransit);
    let breakdown_transit = profile.compute_breakdown(cand_transit);
    assert!(
        breakdown_transit.speedup_e2e > 2.0,
        "Spatial transit achieves high E2E speedup (got {})",
        breakdown_transit.speedup_e2e
    );

    // CAND-21: Resident Candidate Evaluation M=32, k=100
    let cand_cand = &catalog[20];
    assert_eq!(cand_cand.kind, NumericWorkloadKind::KCand);
    let resident_chain = profile.compute_resident_chain(cand_cand, 100);
    assert!(
        resident_chain.speedup_resident > 3.0,
        "Resident chain achieves high speedup (got {})",
        resident_chain.speedup_resident
    );
}

#[test]
fn test_hardware_fallback_and_capability_reporting() {
    let executor = NumericBatchExecutor::new_cpu_fallback();
    assert!(!executor.is_device_active());

    let caps = executor.fallback_compute().capabilities();
    assert_eq!(caps.backend_name, "cpu-fallback");
    assert_eq!(caps.availability, GpuBackendAvailability::CpuFallback);
    assert_eq!(caps.max_workgroup_size, 256);
    assert!(!caps.supports_zero_copy_borrow);
    assert!(!caps.supports_unified_memory);

    // Verify graceful execution on fallback
    let mut fallback = CpuFallbackCompute::new();
    let state = GpuState::default();
    let stats = fallback.upload_state(&state).unwrap();
    assert_eq!(stats.uploaded_bytes, 0);

    let step_stats = fallback.run_abm_step(0.1, 12345).unwrap();
    assert_eq!(step_stats.dispatched_workgroups, 0);

    let downloaded = fallback.download_state().unwrap();
    assert_eq!(downloaded, state);
}

#[test]
fn test_fallback_handles_out_of_range_entities_cleanly_without_panic() {
    let mut fallback = CpuFallbackCompute::new();
    let state = GpuState {
        particles: vec![],
        entity_values: vec![10, 20], // only 2 entities (indices 0 and 1)
    };
    fallback.upload_state(&state).unwrap();

    let invalid_event = vec![kairo_ecs_gpu::compute::DesEvent {
        timestamp_ns: 100,
        entity_id: 999, // Out of range!
        delta: 5,
    }];

    let result = fallback.run_des_step(&invalid_event);
    assert!(result.is_err(), "Must return Err for out of range entity");
    match result.unwrap_err() {
        GpuComputeError::EntityOutOfRange {
            entity_id,
            entity_count,
        } => {
            assert_eq!(entity_id, 999);
            assert_eq!(entity_count, 2);
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }
}
