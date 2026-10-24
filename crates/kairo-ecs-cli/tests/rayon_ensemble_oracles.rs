use kairo_ecs_cli::ensemble::{
    build_bounded_thread_pool, AbcSmcCalibrator, ParallelEnsembleRunner, SobolSensitivityAnalyzer,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[test]
fn test_parallel_sweep_thread_count_bit_invariance() {
    let runner1 = ParallelEnsembleRunner::with_threads(1337, 1);
    let runner2 = ParallelEnsembleRunner::with_threads(1337, 2);
    let runner4 = ParallelEnsembleRunner::with_threads(1337, 4);
    let runner8 = ParallelEnsembleRunner::with_threads(1337, 8);

    // Non-linear evaluation function with mixed transcendental terms
    let points: Vec<Vec<f64>> = (0..128)
        .map(|i| {
            let x = (i as f64) * 0.1;
            vec![x, x.sin(), (x * 0.5).cos()]
        })
        .collect();

    let evaluator = |seed: u64, pt: &[f64]| {
        let s = (seed as f64) * 1e-5;
        (pt[0] * 1.5 + pt[1].powi(2) - pt[2] * s).sin()
    };

    let res1 = runner1.run_sweep(&points, evaluator);
    let res2 = runner2.run_sweep(&points, evaluator);
    let res4 = runner4.run_sweep(&points, evaluator);
    let res8 = runner8.run_sweep(&points, evaluator);

    assert_eq!(res1.len(), 128);
    assert_eq!(res1, res2, "1-worker vs 2-worker bit divergence");
    assert_eq!(res1, res4, "1-worker vs 4-worker bit divergence");
    assert_eq!(res1, res8, "1-worker vs 8-worker bit divergence");
}

#[test]
fn test_sobol_sensitivity_parallel_bit_invariance() {
    // Ishigami function: standard non-linear, non-monotonic sensitivity benchmark
    // f(x) = sin(x1) + 7 * sin^2(x2) + 0.1 * x3^4 * sin(x1)
    let ishigami = |x: &[f64]| {
        let x1 = x[0] * 2.0 * std::f64::consts::PI - std::f64::consts::PI;
        let x2 = x[1] * 2.0 * std::f64::consts::PI - std::f64::consts::PI;
        let x3 = x[2] * 2.0 * std::f64::consts::PI - std::f64::consts::PI;
        x1.sin() + 7.0 * x2.sin().powi(2) + 0.1 * x3.powi(4) * x1.sin()
    };

    let sample_size = 64;
    let dimensions = 3;

    let seq = SobolSensitivityAnalyzer::analyze(&ishigami, dimensions, sample_size);
    let par1 = SobolSensitivityAnalyzer::analyze_parallel(&ishigami, dimensions, sample_size, 1);
    let par2 = SobolSensitivityAnalyzer::analyze_parallel(&ishigami, dimensions, sample_size, 2);
    let par4 = SobolSensitivityAnalyzer::analyze_parallel(&ishigami, dimensions, sample_size, 4);

    for d in 0..dimensions {
        assert!(
            (seq.first_order[d] - par1.first_order[d]).abs() < 1e-12,
            "dim {} seq vs par1 first_order mismatch",
            d
        );
        assert!(
            (par1.first_order[d] - par2.first_order[d]).abs() < 1e-12,
            "dim {} par1 vs par2 first_order mismatch",
            d
        );
        assert!(
            (par2.first_order[d] - par4.first_order[d]).abs() < 1e-12,
            "dim {} par2 vs par4 first_order mismatch",
            d
        );

        assert!(
            (seq.total_effect[d] - par1.total_effect[d]).abs() < 1e-12,
            "dim {} seq vs par1 total_effect mismatch",
            d
        );
        assert!(
            (par1.total_effect[d] - par2.total_effect[d]).abs() < 1e-12,
            "dim {} par1 vs par2 total_effect mismatch",
            d
        );
        assert!(
            (par2.total_effect[d] - par4.total_effect[d]).abs() < 1e-12,
            "dim {} par2 vs par4 total_effect mismatch",
            d
        );
    }
}

#[test]
fn test_abc_smc_calibrator_parallel_bit_invariance() {
    let calibrator = AbcSmcCalibrator::new(0.5);
    let particles: Vec<Vec<f64>> = (0..50)
        .map(|i| vec![(i as f64) * 0.1, ((50 - i) as f64) * 0.2])
        .collect();
    let reference = vec![2.5, 5.0];
    let simulator = |p: &[f64]| vec![p[0] * 1.1, p[1] * 0.95];

    let (w_seq, a_seq) = calibrator.evaluate_particles(&particles, &reference, &simulator);
    let (w_par1, a_par1) =
        calibrator.evaluate_particles_parallel(&particles, &reference, &simulator, 1);
    let (w_par2, a_par2) =
        calibrator.evaluate_particles_parallel(&particles, &reference, &simulator, 2);
    let (w_par4, a_par4) =
        calibrator.evaluate_particles_parallel(&particles, &reference, &simulator, 4);

    assert_eq!(a_seq, a_par1, "acceptance mismatch seq vs par1");
    assert_eq!(a_par1, a_par2, "acceptance mismatch par1 vs par2");
    assert_eq!(a_par2, a_par4, "acceptance mismatch par2 vs par4");

    for i in 0..particles.len() {
        assert!(
            (w_seq[i] - w_par1[i]).abs() < 1e-12,
            "particle {} weight mismatch seq vs par1",
            i
        );
        assert!(
            (w_par1[i] - w_par2[i]).abs() < 1e-12,
            "particle {} weight mismatch par1 vs par2",
            i
        );
        assert!(
            (w_par2[i] - w_par4[i]).abs() < 1e-12,
            "particle {} weight mismatch par2 vs par4",
            i
        );
    }
}

#[test]
fn test_skewed_task_distribution_work_stealing() {
    let runner = ParallelEnsembleRunner::with_threads(42, 4);

    // Highly heterogeneous workload: prime-number trial divisions for skewed latency
    let points: Vec<Vec<f64>> = (0..64)
        .map(|i| {
            // point 0 and 63 have heavy load (large prime candidate), others light
            let difficulty = if i % 16 == 0 { 20000.0 } else { 50.0 };
            vec![difficulty, i as f64]
        })
        .collect();

    let completed_count = Arc::new(AtomicUsize::new(0));
    let cc = Arc::clone(&completed_count);

    let evaluator = move |_seed: u64, pt: &[f64]| {
        let iters = pt[0] as usize;
        let mut acc = pt[1];
        for k in 0..iters {
            acc = (acc + (k as f64) * 0.001).sin();
        }
        cc.fetch_add(1, Ordering::SeqCst);
        acc
    };

    let results = runner.run_sweep(&points, evaluator);
    assert_eq!(results.len(), 64);
    assert_eq!(completed_count.load(Ordering::SeqCst), 64);

    // Verify ordering is preserved bit-identically across threads
    let single_runner = ParallelEnsembleRunner::with_threads(42, 1);
    let single_evaluator = |_seed: u64, pt: &[f64]| {
        let iters = pt[0] as usize;
        let mut acc = pt[1];
        for k in 0..iters {
            acc = (acc + (k as f64) * 0.001).sin();
        }
        acc
    };
    let single_results = single_runner.run_sweep(&points, single_evaluator);
    assert_eq!(results, single_results, "skewed workload ordering mismatch");
}

#[test]
fn test_panic_propagation_in_rayon_pool() {
    let runner = ParallelEnsembleRunner::with_threads(99, 4);

    let points = vec![
        vec![1.0],
        vec![2.0],
        vec![-999.0], // trigger panic
        vec![4.0],
    ];

    let panic_evaluator = |_seed: u64, pt: &[f64]| {
        if pt[0] < 0.0 {
            panic!("intentional test panic on negative parameter point");
        }
        pt[0] * 2.0
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runner.run_sweep(&points, panic_evaluator)
    }));

    assert!(
        result.is_err(),
        "panic should have been propagated out of the rayon pool"
    );
}

#[test]
fn test_reusable_pool_isolation_and_throughput() {
    let pool = build_bounded_thread_pool(4).expect("build pool");
    let runner = ParallelEnsembleRunner::with_threads(555, 4);

    let points: Vec<Vec<f64>> = (0..500).map(|i| vec![i as f64]).collect();
    let evaluator = |seed: u64, pt: &[f64]| pt[0] + (seed as f64) * 1e-4;

    // Run 5 sequential sweeps using the exact same pre-built thread pool
    let start = Instant::now();
    for _ in 0..5 {
        let res = runner.run_sweep_with_pool(&pool, &points, evaluator);
        assert_eq!(res.len(), 500);
    }
    let elapsed = start.elapsed();

    // 2500 evaluations should complete very rapidly
    let total_evals = 5 * 500;
    let rate = (total_evals as f64) / elapsed.as_secs_f64().max(1e-6);
    assert!(
        rate > 50_000.0,
        "Reusable pool throughput should exceed 50k evals/sec, got {:.1}",
        rate
    );
}

#[test]
fn test_empty_and_single_point_boundaries() {
    let runner = ParallelEnsembleRunner::with_threads(42, 8);
    let empty_points: Vec<Vec<f64>> = vec![];
    let empty_res = runner.run_sweep(&empty_points, |_s, _p| 1.0);
    assert!(empty_res.is_empty());

    let single_point = vec![vec![42.0]];
    let single_res = runner.run_sweep(&single_point, |_s, p| p[0] * 2.0);
    assert_eq!(single_res, vec![84.0]);
}

#[test]
fn test_large_ensemble_throughput_budget() {
    let runner = ParallelEnsembleRunner::with_threads(777, 4);
    let num_points = 10_000;
    let points: Vec<Vec<f64>> = (0..num_points)
        .map(|i| vec![(i as f64) * 0.01, (i as f64) * 0.02])
        .collect();

    let start = Instant::now();
    let results = runner.run_sweep(&points, |seed, pt| (seed as f64) * 0.001 + pt[0] + pt[1]);
    let elapsed = start.elapsed();

    assert_eq!(results.len(), num_points);
    let rate = (num_points as f64) / elapsed.as_secs_f64().max(1e-6);
    assert!(
        rate > 100_000.0,
        "Large ensemble evaluation rate should exceed 100k evals/sec, got {:.1}",
        rate
    );
}
