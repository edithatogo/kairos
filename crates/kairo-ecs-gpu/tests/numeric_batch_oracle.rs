//! Integration and unit tests for numeric batch workloads, CPU oracle,
//! contract validation (D0-D5), and economic break-even profiling per Report 26.

#![forbid(unsafe_code)]

use kairo_ecs_gpu::{
    compare_f32_slices, compute_candidate_evaluations, compute_elt, compute_hist, compute_ks_cdf,
    compute_scan, compute_sort_full_w1, compute_transit, compute_w1_presort, compute_w1_weighted,
    evaluate_kernel_selection, float_ulp_diff, report_26_workload_catalog,
    spearman_rank_correlation, AgentTransitQuery, AgentTransitResult, ContractClass,
    CostModelProfile, D0StateAuthority, GpuCompute, NumericBatchError, NumericBatchExecutor,
    NumericWorkloadKind, CATALOG_BATCH_SIZES, D2_MAX_ABSOLUTE_ERROR, D2_MAX_RELATIVE_ERROR,
    D3_REDUCTION_TOLERANCE, D4_MIN_SPEARMAN_RHO, D5_REPEAT_RUN_COUNT,
};

#[test]
fn test_26_workload_candidate_catalog_and_kernel_selection() {
    let catalog = report_26_workload_catalog();

    // Verify ContractClass label formatting
    assert!(ContractClass::D0.label().contains("D0"));
    assert!(ContractClass::D1.label().contains("D1"));
    assert!(ContractClass::D2.label().contains("D2"));
    assert!(ContractClass::D3.label().contains("D3"));
    assert!(ContractClass::D4.label().contains("D4"));
    assert!(ContractClass::D5.label().contains("D5"));

    // Exactly 26 workload candidates per Report 26
    assert_eq!(
        catalog.len(),
        26,
        "Catalog must contain exactly 26 workload candidates"
    );

    // Verify all candidates have valid identifiers and names
    for (idx, cand) in catalog.iter().enumerate() {
        assert!(
            !cand.id.is_empty(),
            "Candidate {idx} must have non-empty ID"
        );
        assert!(
            !cand.name.is_empty(),
            "Candidate {idx} must have non-empty name"
        );
        assert!(
            CATALOG_BATCH_SIZES.contains(&cand.batch_size),
            "Candidate {} batch size {} must be one of CATALOG_BATCH_SIZES",
            cand.id,
            cand.batch_size
        );
    }

    // Verify all 8 batch sizes are covered in the catalog
    for &bs in &CATALOG_BATCH_SIZES {
        let count = catalog.iter().filter(|c| c.batch_size == bs).count();
        assert!(
            count > 0,
            "Catalog must contain at least one workload of batch size {bs}"
        );
    }

    // Verify all 9 workload kinds are represented
    let expected_kinds = [
        NumericWorkloadKind::KElt,
        NumericWorkloadKind::KW1Presort,
        NumericWorkloadKind::KW1Weighted,
        NumericWorkloadKind::KKsCdf,
        NumericWorkloadKind::KHist,
        NumericWorkloadKind::KScan,
        NumericWorkloadKind::KCand,
        NumericWorkloadKind::KTransit,
        NumericWorkloadKind::KSortFull,
    ];
    for kind in expected_kinds {
        let count = catalog.iter().filter(|c| c.kind == kind).count();
        assert!(
            count > 0,
            "Workload kind {:?} must be present in catalog",
            kind
        );
    }

    // Verify the kernel selection decision per Report 26
    let decision = evaluate_kernel_selection(&catalog);
    assert_eq!(
        decision.primary_kernel,
        NumericWorkloadKind::KTransit,
        "Primary one-shot accelerated kernel must be K-TRANSIT"
    );
    assert_eq!(
        decision.resident_kernel,
        NumericWorkloadKind::KCand,
        "Primary resident chain kernel must be K-CAND"
    );
    assert!(
        decision.transit_speedup_e2e > 1.0,
        "K-TRANSIT must demonstrate measurable end-to-end benefit (S_e2e > 1.0)"
    );
    assert!(
        decision.resident_cand_speedup > 1.0,
        "Resident K-CAND must demonstrate measurable end-to-end benefit (S_resident > 1.0)"
    );
    assert!(
        !decision.justification.is_empty(),
        "Selection decision must provide explicit technical justification"
    );
}

#[test]
fn test_d0_exact_state_authority_and_rng_retention() {
    let initial_timestamp_ns = 1_000_000_000u64;
    let initial_seq_key = 0u64;
    let initial_seed = 42u64;

    let mut authority = D0StateAuthority::new(initial_timestamp_ns, initial_seq_key, initial_seed);
    let dt_step_ns = 16_666_667u64; // ~60 FPS step

    // Advance 100 simulation steps
    for _ in 0..100 {
        authority.advance_step(dt_step_ns);
    }

    assert_eq!(
        authority.timestamp_ns,
        initial_timestamp_ns + 100 * dt_step_ns,
        "CPU clock must advance with exact nanosecond arithmetic"
    );
    assert_eq!(
        authority.sequence_key, 100,
        "Sequence key must advance strictly deterministically"
    );

    // Verify that identical authority produces identical pseudo-random draws
    let mut auth_a = D0StateAuthority::new(100, 1, 999);
    let mut auth_b = D0StateAuthority::new(100, 1, 999);
    for _ in 0..50 {
        let draw_a = auth_a.next_uniform_f32();
        let draw_b = auth_b.next_uniform_f32();
        assert_eq!(
            draw_a.to_bits(),
            draw_b.to_bits(),
            "RNG draws on identical seed must be bit-identical"
        );
    }

    // Verify that batch numeric execution does NOT mutate authority state
    let state_before = authority;
    let test_queries = [AgentTransitQuery {
        start_x: 0.0,
        start_y: 0.0,
        dest_x: 3.0,
        dest_y: 4.0,
        speed: 5.0,
    }];
    let _ = compute_transit(&test_queries).unwrap();
    assert_eq!(
        authority, state_before,
        "Device/numeric execution must never mutate CPU D0 authority state"
    );
}

#[test]
fn test_d1_exact_integer_histogram_and_scan_parity() {
    // 1. Exact histogram binning
    let samples = [0.1f32, 0.4, 0.8, 1.2, 1.5, 1.8, 2.0];
    let counts = compute_hist(&samples, 4, 0.0, 2.0).unwrap();
    // Bins: [0.0, 0.5) -> [0.1, 0.4] (2)
    //       [0.5, 1.0) -> [0.8] (1)
    //       [1.0, 1.5) -> [1.2] (1)
    //       [1.5, 2.0] -> [1.5, 1.8, 2.0] (3)
    assert_eq!(counts, vec![2, 1, 1, 3]);

    let total_counted: u64 = counts.iter().sum();
    assert_eq!(total_counted, samples.len() as u64);

    // 2. Exact prefix scan
    let counts_in = [5u64, 10, 15, 20, 25, 30];
    let scan = compute_scan(&counts_in).unwrap();
    assert_eq!(scan, vec![5, 15, 30, 50, 75, 105]);

    // Bit-identity check across multiple calls
    let scan_repeat = compute_scan(&counts_in).unwrap();
    assert_eq!(scan, scan_repeat);

    // 3. Exact integer overflow detection
    let overflow_counts = [u64::MAX - 2, 5];
    let overflow_err = compute_scan(&overflow_counts);
    assert_eq!(overflow_err, Err(NumericBatchError::Overflow));
}

#[test]
fn test_d2_floating_elementwise_precision_and_ulp() {
    let n = 1024;
    let mut pred = Vec::with_capacity(n);
    let mut obs = Vec::with_capacity(n);
    for i in 0..n {
        pred.push((i as f32) * 0.1);
        obs.push(((i as f32) * 0.1) - 0.00005);
    }

    // Compute elementwise residuals with clamp
    let results = compute_elt(&pred, &obs, 0.0, 10.0).unwrap();
    assert_eq!(results.len(), n);

    // Manual reference computation
    let mut reference = Vec::with_capacity(n);
    for i in 0..n {
        let diff = pred[i] - obs[i];
        let loss = diff * diff;
        reference.push(loss.clamp(0.0, 10.0));
    }

    let metrics = compare_f32_slices(&results, &reference).unwrap();
    assert!(
        metrics.satisfies_contract(),
        "Observed max_abs_error {} or max_rel_error {} exceeded D2 limits",
        metrics.max_abs_error,
        metrics.max_rel_error
    );
    assert!(
        metrics.max_ulp_diff <= 2,
        "ULP difference must be minimal for elementwise transforms (got {})",
        metrics.max_ulp_diff
    );

    // Test K-TRANSIT precision
    let transit_queries = (0..512)
        .map(|i| AgentTransitQuery {
            start_x: (i as f32) * 0.5,
            start_y: (i as f32) * 0.25,
            dest_x: (i as f32) * 0.5 + 3.0,
            dest_y: (i as f32) * 0.25 + 4.0,
            speed: 5.0,
        })
        .collect::<Vec<_>>();

    // Direct verification of float_ulp_diff and AgentTransitResult type
    assert_eq!(float_ulp_diff(1.0, 1.0), 0);
    assert!(float_ulp_diff(1.0, 1.0000001) <= 2);

    let transit_results: Vec<AgentTransitResult> = compute_transit(&transit_queries).unwrap();
    assert_eq!(transit_results.len(), 512);
    for r in &transit_results {
        // sqrt(3^2 + 4^2) = 5.0, speed = 5.0 -> time = 1.0
        assert!(((r.distance - 5.0).abs() as f64) <= D2_MAX_ABSOLUTE_ERROR);
        assert!(((r.distance - 5.0).abs() as f64) <= D2_MAX_RELATIVE_ERROR * 5.0);
        assert!(((r.time_to_dest - 1.0).abs() as f64) <= D2_MAX_ABSOLUTE_ERROR);
    }
}

#[test]
fn test_d3_w1_presort_vs_weighted_reduction_invariants() {
    let u = [1.0f32, 3.0, 5.0, 7.0];
    let v = [2.0f32, 2.5, 6.0, 8.0];

    // Presorted equal-size W1
    let w1_presort = compute_w1_presort(&u, &v).unwrap();
    // Weighted/unequal W1 with default uniform weights
    let w1_weighted = compute_w1_weighted(&u, None, &v, None).unwrap();

    // Verify mathematical equivalence between presorted reduction and general interval integral
    let diff = (w1_presort - w1_weighted).abs();
    assert!(
        diff <= D3_REDUCTION_TOLERANCE,
        "Presorted W1 ({w1_presort}) and weighted uniform W1 ({w1_weighted}) must match within tolerance"
    );

    // Invariant 1: Non-negativity W1(u, v) >= 0
    assert!(w1_presort >= 0.0);
    assert!(w1_weighted >= 0.0);

    // Invariant 2: Identity of indiscernibles W1(u, u) == 0
    let w1_identity = compute_w1_presort(&u, &u).unwrap();
    assert!(w1_identity.abs() < 1e-12);
    let w1_weighted_identity = compute_w1_weighted(&u, None, &u, None).unwrap();
    assert!(w1_weighted_identity.abs() < 1e-12);

    // Invariant 3: Symmetry W1(u, v) == W1(v, u)
    let w1_sym = compute_w1_presort(&v, &u).unwrap();
    assert!((w1_presort - w1_sym).abs() < 1e-12);

    // Invariant 4: Triangle Inequality W1(u, w) <= W1(u, v) + W1(v, w)
    let w = [0.5f32, 4.0, 5.5, 9.0];
    let w1_uw = compute_w1_presort(&u, &w).unwrap();
    let w1_vw = compute_w1_presort(&v, &w).unwrap();
    assert!(
        w1_uw <= w1_presort + w1_vw + 1e-6,
        "Triangle inequality must hold: W1(u, w) <= W1(u, v) + W1(v, w)"
    );

    // Unequal-length weighted distributions
    let u_unequal = [1.0f32, 2.0, 5.0];
    let w_u = [0.2f64, 0.5, 0.3];
    let v_unequal = [1.5f32, 3.0, 4.0, 6.0];
    let w_v = [0.1f64, 0.3, 0.4, 0.2];

    let w1_unequal = compute_w1_weighted(&u_unequal, Some(&w_u), &v_unequal, Some(&w_v)).unwrap();
    assert!(w1_unequal > 0.0);

    // Invariant check on KS-CDF
    let ks_ident = compute_ks_cdf(&u, &u).unwrap();
    assert!(ks_ident.abs() < 1e-12);
    let ks_uv = compute_ks_cdf(&u, &v).unwrap();
    let ks_vu = compute_ks_cdf(&v, &u).unwrap();
    assert!((ks_uv - ks_vu).abs() < 1e-12);
    assert!((0.0..=1.0).contains(&ks_uv));

    // Full sort + W1 pipeline test
    let u_unsorted = [7.0f32, 1.0, 5.0, 3.0];
    let v_unsorted = [8.0f32, 2.0, 6.0, 2.5];
    let w1_sorted_full = compute_sort_full_w1(&u_unsorted, &v_unsorted).unwrap();
    assert!((w1_sorted_full - w1_presort).abs() < 1e-12);
}

#[test]
fn test_d4_decision_stability_and_candidate_ranking_retention() {
    let n_obs = 1024;
    let n_cands = 16;

    // Reference observations
    let reference_obs: Vec<f32> = (0..n_obs).map(|i| (i as f32) * 0.05).collect();

    // 16 candidates with increasing synthetic errors
    let mut candidates = Vec::with_capacity(n_cands);
    for cand_idx in 0..n_cands {
        let offset = (cand_idx as f32) * 0.1;
        let preds: Vec<f32> = reference_obs.iter().map(|&x| x + offset).collect();
        candidates.push(preds);
    }

    // Baseline ranking
    let baseline_evals = compute_candidate_evaluations(&candidates, &reference_obs).unwrap();
    assert_eq!(baseline_evals.len(), n_cands);

    // Verify ordering: candidate 0 has 0 offset, should be rank 0
    assert_eq!(
        baseline_evals[0].candidate_id, 0,
        "Candidate 0 must be winning candidate"
    );
    for i in 0..n_cands - 1 {
        assert!(baseline_evals[i].score <= baseline_evals[i + 1].score);
    }

    // Perturb predictions slightly (within D2 floating noise ~ 1e-7)
    let mut perturbed_candidates = candidates.clone();
    for preds in &mut perturbed_candidates {
        for (i, p) in preds.iter_mut().enumerate() {
            *p += if i % 2 == 0 { 1e-7 } else { -1e-7 };
        }
    }

    let perturbed_evals =
        compute_candidate_evaluations(&perturbed_candidates, &reference_obs).unwrap();

    // D4 winning-candidate preservation: winning candidate identity must NOT flip
    assert_eq!(
        perturbed_evals[0].candidate_id, baseline_evals[0].candidate_id,
        "Winning candidate identity must be preserved across numerical perturbations"
    );

    // Extract score vectors for Spearman correlation
    let mut scores_base = vec![0.0f64; n_cands];
    let mut scores_pert = vec![0.0f64; n_cands];
    for eval in &baseline_evals {
        scores_base[eval.candidate_id] = eval.score;
    }
    for eval in &perturbed_evals {
        scores_pert[eval.candidate_id] = eval.score;
    }

    let rho = spearman_rank_correlation(&scores_base, &scores_pert).unwrap();
    assert!(
        rho >= D4_MIN_SPEARMAN_RHO,
        "Spearman rank correlation {rho} must be >= {D4_MIN_SPEARMAN_RHO}"
    );
}

#[test]
fn test_d5_repeated_execution_identical_draws() {
    let mut authority = D0StateAuthority::new(0, 0, 123_456_789);
    let sample_count = 256;

    // Generate fixed deterministic inputs
    let mut fixed_values = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        fixed_values.push(authority.next_uniform_f32() * 10.0);
    }

    // Baseline integer histogram and prefix scan
    let baseline_hist = compute_hist(&fixed_values, 16, 0.0, 10.0).unwrap();
    let baseline_scan = compute_scan(&baseline_hist).unwrap();

    // Baseline floating transit evaluation
    let transit_queries: Vec<AgentTransitQuery> = fixed_values
        .chunks_exact(4)
        .map(|chunk| AgentTransitQuery {
            start_x: chunk[0],
            start_y: chunk[1],
            dest_x: chunk[2],
            dest_y: chunk[3],
            speed: 1.5,
        })
        .collect();
    let baseline_transit = compute_transit(&transit_queries).unwrap();

    // Repeat identical runs for D5_REPEAT_RUN_COUNT iterations
    for run in 0..D5_REPEAT_RUN_COUNT {
        let repeated_hist = compute_hist(&fixed_values, 16, 0.0, 10.0).unwrap();
        assert_eq!(
            repeated_hist, baseline_hist,
            "Histogram counts must be bit-identical across repetition {run}"
        );

        let repeated_scan = compute_scan(&repeated_hist).unwrap();
        assert_eq!(
            repeated_scan, baseline_scan,
            "Prefix scan results must be bit-identical across repetition {run}"
        );

        let repeated_transit = compute_transit(&transit_queries).unwrap();
        for (i, (act, exp)) in repeated_transit
            .iter()
            .zip(baseline_transit.iter())
            .enumerate()
        {
            assert_eq!(
                act.distance.to_bits(),
                exp.distance.to_bits(),
                "Transit distance must be bit-identical at item {i} in repetition {run}"
            );
            assert_eq!(
                act.time_to_dest.to_bits(),
                exp.time_to_dest.to_bits(),
                "Transit time must be bit-identical at item {i} in repetition {run}"
            );
        }
    }
}

#[test]
fn test_break_even_and_resident_chain_economics() {
    let profile = CostModelProfile::default();
    let catalog = report_26_workload_catalog();

    // Find K-TRANSIT candidates of various batch sizes
    let transit_small = catalog
        .iter()
        .find(|c| c.id == "CAND-23" && c.kind == NumericWorkloadKind::KTransit)
        .unwrap();
    let transit_med = catalog
        .iter()
        .find(|c| c.id == "CAND-24" && c.kind == NumericWorkloadKind::KTransit)
        .unwrap();
    let transit_large = catalog
        .iter()
        .find(|c| c.id == "CAND-25" && c.kind == NumericWorkloadKind::KTransit)
        .unwrap();

    let bd_transit_small = profile.compute_breakdown(transit_small);
    let bd_transit_med = profile.compute_breakdown(transit_med);
    let bd_transit_large = profile.compute_breakdown(transit_large);

    // K-TRANSIT has high compute intensity: S_e2e grows with N and exceeds 1.0
    assert!(
        bd_transit_small.speedup_e2e > 1.0,
        "K-TRANSIT at N=1024 breaks even"
    );
    assert!(
        bd_transit_med.speedup_e2e > bd_transit_small.speedup_e2e,
        "Speedup must increase with batch size"
    );
    assert!(
        bd_transit_large.speedup_e2e > 10.0,
        "Large batch transit speedup should exceed 10x"
    );

    // In contrast, examine presorted W1 (low arithmetic intensity)
    let w1_micro = catalog.iter().find(|c| c.id == "CAND-04").unwrap();
    let bd_w1_micro = profile.compute_breakdown(w1_micro);
    assert!(
        bd_w1_micro.speedup_e2e < 1.0,
        "One-shot presorted W1 at N=256 loses due to transfer/launch overhead"
    );

    // Examine resident chain economics on K-CAND
    let cand_workload = catalog.iter().find(|c| c.id == "CAND-21").unwrap();
    let chain_1 = profile.compute_resident_chain(cand_workload, 1);
    let chain_8 = profile.compute_resident_chain(cand_workload, 8);
    let chain_32 = profile.compute_resident_chain(cand_workload, 32);

    assert!(
        chain_8.speedup_resident > chain_1.speedup_resident,
        "Resident chain speedup must grow as ingress overhead is amortized"
    );
    assert!(
        chain_32.speedup_resident > chain_8.speedup_resident,
        "Deep resident chain must achieve higher speedup"
    );
    assert!(
        chain_32.speedup_resident > 2.0,
        "Resident K-CAND at k=32 should have significant speedup (> 2.0x)"
    );
}

#[test]
fn test_device_unavailable_fallback_and_error_handling() {
    // 1. Fallback executor
    let executor = NumericBatchExecutor::new_cpu_fallback();
    assert!(
        !executor.is_device_active(),
        "Fallback executor must indicate device is inactive"
    );
    assert_eq!(
        executor.fallback_compute().capabilities().backend_name,
        "cpu-fallback"
    );

    // Verify all executor calls succeed on CPU fallback
    let queries = [AgentTransitQuery {
        start_x: 0.0,
        start_y: 0.0,
        dest_x: 6.0,
        dest_y: 8.0,
        speed: 2.0,
    }];
    let res = executor.execute_transit(&queries).unwrap();
    assert_eq!(res.len(), 1);
    assert!((res[0].distance - 10.0).abs() < 1e-5);
    assert!((res[0].time_to_dest - 5.0).abs() < 1e-5);

    let elt_res = executor
        .execute_elt(&[1.0, 2.0], &[0.5, 1.0], 0.0, 5.0)
        .unwrap();
    assert_eq!(elt_res.len(), 2);

    let w1_res = executor
        .execute_w1_presort(&[1.0, 2.0], &[1.5, 2.5])
        .unwrap();
    assert!((w1_res - 0.5).abs() < 1e-6);

    let hist_res = executor.execute_hist(&[0.5, 1.5], 2, 0.0, 2.0).unwrap();
    assert_eq!(hist_res, vec![1, 1]);

    let scan_res = executor.execute_scan(&[3, 4, 5]).unwrap();
    assert_eq!(scan_res, vec![3, 7, 12]);

    // 2. Strict typed error handling
    // Shape mismatch
    let err_shape = compute_elt(&[1.0, 2.0], &[1.0], 0.0, 1.0);
    assert_eq!(
        err_shape,
        Err(NumericBatchError::BufferShapeMismatch {
            expected: 1,
            actual: 2
        })
    );

    // Invalid candidate bounds (min > max)
    let err_invalid = compute_elt(&[1.0], &[1.0], 5.0, 2.0);
    assert!(matches!(
        err_invalid,
        Err(NumericBatchError::InvalidCandidate { .. })
    ));

    // Empty input
    let err_empty = compute_w1_presort(&[], &[]);
    assert_eq!(err_empty, Err(NumericBatchError::EmptyInput));

    // Out of range bin
    let err_bin = compute_hist(&[5.0], 4, 0.0, 2.0);
    assert_eq!(
        err_bin,
        Err(NumericBatchError::OutOfRangeBin {
            value: 5.0,
            min_val: 0.0,
            max_val: 2.0
        })
    );

    // Overflow in prefix scan
    let err_overflow = compute_scan(&[u64::MAX, 1]);
    assert_eq!(err_overflow, Err(NumericBatchError::Overflow));
}
