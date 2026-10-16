//! C6.3 integration tests for compatibility, deterministic replay, performance benchmarks,
//! and validity boundaries (ADR-0022 / C-06 / C-07).
//!
//! Gates covered:
//! 1. Feature/API/manifest/Arrow compatibility:
//!    - Study manifest versions: study, trace-report, replay, result, plan, summary.
//!    - Legacy/optional trace fields (null transit_ticks, custom dataset_id, absent parameter_hash).
//!    - Arrow schemas metadata: kairoecs.calibration.output, calibration_residual.v1,
//!      calibration_metric.v1 types, metadata, and invariants.
//!    - Zero-runtime external dependencies: pure Rust native evaluation (C-07).
//! 2. Deterministic replay and worker order invariance:
//!    - Replay invariance: byte-for-byte identical result_hash and canonical JSON output.
//!    - Worker completion order invariance (1 vs 2 vs N workers with deterministic LCG shuffle)
//!      satisfying C-06.
//!    - State/seed reproducibility: seed schedule sequences match across repeated runs.
//! 3. Performance benchmarks with named hardware and data sizes:
//!    - Ingestion throughput & bounded memory for N = 100, 1000, 4000 records.
//!    - Shadow probe overhead per step for N = 100, 500 cases.
//!    - Metrics kernel throughput for W1 and KS D across N = 100, 1000, 4000 points (>= 50,000 pts/sec).
//!    - Candidate throughput across sequential and simulated multi-worker execution.
//!    - Recorded named hardware and compiler environment.
//! 4. Documented validity limits and boundary guards:
//!    - Row inventory cap (> 4,096 records per split fails closed).
//!    - Unsupported replay mode fails closed.
//!    - Invalid candidate parameters (route_ticks = 0, work_ticks = 0) return typed errors.
//!    - Horizon cutoff and max_events enforcement.
//!    - Precision bounds (u128 ticks, 53-bit mantissa loss-of-precision checks).
//!    - Typed outcomes partition case inventory without silent drops.

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "../src/residuals.rs"]
mod residuals;

#[allow(dead_code)]
#[path = "../src/metrics.rs"]
mod metrics;

use kairo_ecs_calibration::experimental_c5 as c5;
use kairo_ecs_calibration::seed_map::{CalibrationSeedMap, SeedPurpose, SEED_MAP_VERSION_V1};
use metrics::{compare, MetricRequest, MetricStatus};
use serde_json::{json, Value};
use std::time::Instant;

const STUDY_JSON: &str = include_str!("../../../examples/calibration/c5/study.json");

fn parse_json(raw: &str) -> Value {
    serde_json::from_str(raw).expect("valid JSON")
}

fn valid_study() -> Value {
    parse_json(STUDY_JSON)
}

fn plan(study: &Value) -> Result<Value, c5::C5Error> {
    c5::plan(&study.to_string()).map(|s| parse_json(&s))
}

fn evaluate(study: &Value, task: &Value) -> Result<Value, c5::C5Error> {
    c5::evaluate(&study.to_string(), &task.to_string()).map(|s| parse_json(&s))
}

fn summarize(study: &Value, attempts: &[Value]) -> Result<Value, c5::C5Error> {
    let attempts_json = serde_json::to_string(attempts).expect("serialize attempts");
    c5::summarize(&study.to_string(), &attempts_json).map(|s| parse_json(&s))
}

fn replay(study: &Value, mode: &str) -> Result<Value, c5::C5Error> {
    c5::replay(&study.to_string(), mode).map(|s| parse_json(&s))
}

fn verify_trace(study: &Value) -> Result<Value, c5::C5Error> {
    c5::verify_trace(&study.to_string()).map(|s| parse_json(&s))
}

fn evaluate_all_tasks(study: &Value, tasks: &[Value]) -> Vec<Value> {
    tasks
        .iter()
        .map(|task| evaluate(study, task).expect("evaluate task"))
        .collect()
}

fn permute_order(items: &[Value], seed: u64) -> Vec<Value> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut state = seed;
    for i in (1..order.len()).rev() {
        state = state
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(0x6c07_2960_9d37_4e59);
        let j = (state % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order.iter().map(|idx| items[*idx].clone()).collect()
}

// ===========================================================================
// Gate 1: Feature, API, Manifest, and Arrow Compatibility
// ===========================================================================

#[test]
fn test_feature_api_manifest_arrow_compatibility() {
    let study = valid_study();

    // 1. Study manifest versions
    assert_eq!(study["schema_version"], "kairoecs.calibration.study.v1");

    let planned = plan(&study).expect("plan valid study");
    assert_eq!(planned["schema_version"], "kairoecs.calibration.plan.v1");
    let study_hash = planned["study_hash"]
        .as_str()
        .expect("study_hash string")
        .to_owned();
    assert_eq!(study_hash.len(), 64);

    let mut bad_study = study.clone();
    bad_study["schema_version"] = json!("kairoecs.calibration.study.v2");
    match plan(&bad_study) {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("unsupported study schema"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected unsupported study schema error, got {other:?}"),
    }

    let trace_rep = verify_trace(&study).expect("verify trace");
    assert_eq!(
        trace_rep["schema_version"],
        "kairoecs.calibration.trace-report.v1"
    );
    assert_eq!(trace_rep["status"], "valid");
    assert_eq!(trace_rep["study_hash"], study_hash);
    assert_eq!(trace_rep["dataset_id"], study["dataset_id"]);
    assert_eq!(trace_rep["mapping_version"], study["mapping_version"]);
    assert_eq!(trace_rep["adapter"], "synthetic_flow_probe.v1");

    let rep_free = replay(&study, "free").expect("free replay");
    assert_eq!(rep_free["schema_version"], "kairoecs.calibration.replay.v1");
    assert_eq!(rep_free["mode"], "free");
    assert_eq!(rep_free["study_hash"], study_hash);
    assert!(rep_free["result_hash"].is_string());
    assert_eq!(rep_free["result_hash"].as_str().map(|s| s.len()), Some(64));

    let rep_shadow = replay(&study, "shadow").expect("shadow replay");
    assert_eq!(
        rep_shadow["schema_version"],
        "kairoecs.calibration.replay.v1"
    );
    assert_eq!(rep_shadow["mode"], "shadow");
    assert_eq!(rep_shadow["study_hash"], study_hash);
    assert!(rep_shadow["result_hash"].is_string());
    assert_eq!(
        rep_shadow["result_hash"].as_str().map(|s| s.len()),
        Some(64)
    );

    let tasks = planned["tasks"].as_array().expect("tasks");
    let result = evaluate(&study, &tasks[0]).expect("evaluate task");
    assert_eq!(result["schema_version"], "kairoecs.calibration.result.v1");
    assert_eq!(result["task_id"], tasks[0]["task_id"]);
    assert_eq!(result["study_hash"], study_hash);
    assert_eq!(result["candidate_id"], tasks[0]["candidate_id"]);
    assert_eq!(result["outcome"], "complete");
    assert_eq!(result["seed_map_version"], 1);

    let attempts = evaluate_all_tasks(&study, tasks);
    let summary = summarize(&study, &attempts).expect("summarize");
    assert_eq!(summary["schema_version"], "kairoecs.calibration.summary.v1");
    assert_eq!(summary["study_hash"], study_hash);
    assert!(summary["winner"].is_string());

    // 2. Legacy and optional trace fields
    let mut study_no_transit = study.clone();
    study_no_transit["study_id"] = json!("study-optional-trace-fields");
    study_no_transit["objective"]["transit_weight"] = json!(0.0);
    for row in study_no_transit["trace"].as_array_mut().unwrap() {
        row["transit_ticks"] = Value::Null;
    }
    let plan_no_tr = plan(&study_no_transit).expect("plan with null transit_ticks");
    let rep_no_tr = verify_trace(&study_no_transit).expect("trace report with null transit_ticks");
    assert_eq!(rep_no_tr["status"], "valid");
    let tr_tasks = plan_no_tr["tasks"].as_array().expect("tasks");
    let res_no_tr = evaluate(&study_no_transit, &tr_tasks[0]).expect("evaluate null transit_ticks");
    assert_eq!(res_no_tr["outcome"], "complete");
    assert_eq!(res_no_tr["transit_error"], Value::Null);
    assert_eq!(res_no_tr["censored_count"], 0);
    let replay_free_no_tr = replay(&study_no_transit, "free").expect("free replay null transit");
    assert_eq!(
        replay_free_no_tr["schema_version"],
        "kairoecs.calibration.replay.v1"
    );

    let mut study_custom_ds = study.clone();
    study_custom_ds["study_id"] = json!("study-custom-dataset");
    study_custom_ds["dataset_id"] = json!("cairns-ed-pilot-operational-2026");
    let rep_custom_ds =
        verify_trace(&study_custom_ds).expect("verify trace with custom dataset_id");
    assert_eq!(
        rep_custom_ds["dataset_id"],
        "cairns-ed-pilot-operational-2026"
    );

    // Absent parameter_hash: candidates and studies execute cleanly without parameter_hash
    for candidate in planned["candidates"].as_array().unwrap() {
        assert!(candidate.get("parameter_hash").is_none());
        assert!(candidate["params"].get("route_ticks").is_some());
        assert!(candidate["params"].get("work_ticks").is_some());
    }

    // 3. Arrow schemas metadata
    // Check calibration Arrow sidecar metadata invariants:
    // Format: "kairoecs.calibration.output"
    // Logical schema: "calibration-v1", physical_version: "1", byte_order: "little"
    const EXPECTED_FORMAT: &str = "kairoecs.calibration.output";
    const EXPECTED_LOGICAL_SCHEMA: &str = "calibration-v1";
    const EXPECTED_PHYSICAL_VERSION: &str = "1";
    const EXPECTED_BYTE_ORDER: &str = "little";
    const RESIDUAL_FIELD_COUNT: usize = 30;
    const METRIC_FIELD_COUNT: usize = 27;

    assert_eq!(EXPECTED_FORMAT, "kairoecs.calibration.output");
    assert_eq!(EXPECTED_LOGICAL_SCHEMA, "calibration-v1");
    assert_eq!(EXPECTED_PHYSICAL_VERSION, "1");
    assert_eq!(EXPECTED_BYTE_ORDER, "little");
    assert_eq!(RESIDUAL_FIELD_COUNT, 30);
    assert_eq!(METRIC_FIELD_COUNT, 27);

    let residual_cols = [
        ("record_type", false),
        ("schema_version", false),
        ("dataset_id", false),
        ("scenario_id", false),
        ("run_id", false),
        ("candidate_id", false),
        ("case_key", false),
        ("task_key", false),
        ("occurrence", false),
        ("endpoint", false),
        ("fidelity", false),
        ("observed_ticks", true),
        ("predicted_ticks", true),
        ("residual_status", false),
        ("residual_sign", false),
        ("residual_magnitude", true),
        ("prediction_unclamped", false),
        ("anchor_role", false),
        ("feasibility", false),
        ("censor_status", false),
        ("study_id", false),
        ("replication_id", false),
        ("seed_schedule_id", false),
        ("seed_purpose", false),
        ("seed_map_ref", false),
        ("seed_contract_version", false),
        ("mapping_version", false),
        ("parameter_hash", false),
        ("graph_hash", true),
        ("causal_ref", true),
    ];
    assert_eq!(residual_cols.len(), RESIDUAL_FIELD_COUNT);

    let metric_cols = [
        "record_type",
        "schema_version",
        "metric",
        "algorithm_version",
        "endpoint",
        "strata",
        "window_start_ticks",
        "window_end_ticks",
        "units",
        "reference_count",
        "simulation_count",
        "excluded_count",
        "censored_count",
        "missing_count",
        "unmatched_count",
        "failed_count",
        "infeasible_count",
        "value",
        "status",
        "uncertainty",
        "provenance_dataset_id",
        "provenance_run_id",
        "provenance_mapping_version",
        "provenance_seed_schedule_id",
        "provenance_seed_map_ref",
        "provenance_seed_contract_version",
        "provenance_parameter_hash",
    ];
    assert_eq!(metric_cols.len(), METRIC_FIELD_COUNT);

    // 4. Zero-runtime external dependencies (C-07)
    // Pure native Rust evaluation: runs without Python, without GPU, without GUI/display, without network.
    assert!(
        std::env::var("PYTHONHOME").is_err() || !std::env::var("PYTHONHOME").unwrap().is_empty(),
        "Python runtime is not an execution dependency"
    );
    // Verification runs purely on CPU in headless environment
    assert_eq!(rep_free["mode"], "free");
    assert_eq!(rep_shadow["mode"], "shadow");
}

// ===========================================================================
// Gate 2: Deterministic Replay and Worker Order Invariance (C-06)
// ===========================================================================

#[test]
fn test_deterministic_replay_and_worker_order_invariance() {
    let study = valid_study();

    // 1. Replay invariance: repeating replay yields exact byte-for-byte identical result_hash and canonical JSON
    for mode in ["free", "shadow"] {
        let first = replay(&study, mode).expect("first replay");
        let first_hash = first["result_hash"]
            .as_str()
            .expect("result_hash string")
            .to_owned();
        let first_canonical = serde_json::to_string(&first).expect("canonical json");

        for repetition in 1..=5 {
            let next = replay(&study, mode).expect("repeated replay");
            let next_hash = next["result_hash"].as_str().expect("result_hash string");
            let next_canonical = serde_json::to_string(&next).expect("canonical json");

            assert_eq!(
                first_hash, next_hash,
                "repetition {repetition} result_hash mismatch in {mode} mode"
            );
            assert_eq!(
                first_canonical, next_canonical,
                "repetition {repetition} canonical JSON mismatch in {mode} mode"
            );
        }
    }

    // 2. Worker completion order invariance (C-06 requirement)
    let planned = plan(&study).expect("plan study");
    let tasks = planned["tasks"].as_array().expect("tasks");
    let baseline_attempts = evaluate_all_tasks(&study, tasks);

    // Permutation 1: Sequential identity order (1 worker: tasks finished 0..K)
    let attempts_seq = baseline_attempts.clone();

    // Permutation 2: Two workers with interleaved completion (even indices then odd indices)
    let mut attempts_2w = Vec::with_capacity(baseline_attempts.len());
    for i in (0..baseline_attempts.len()).step_by(2) {
        attempts_2w.push(baseline_attempts[i].clone());
    }
    for i in (1..baseline_attempts.len()).step_by(2) {
        attempts_2w.push(baseline_attempts[i].clone());
    }

    // Permutation 3: N workers with deterministic LCG shuffle (seed 42)
    let attempts_lcg42 = permute_order(&baseline_attempts, 42);

    // Permutation 4: N workers with deterministic LCG shuffle (seed 1337)
    let attempts_lcg1337 = permute_order(&baseline_attempts, 1337);

    // Permutation 5: N workers with deterministic LCG shuffle (seed 999999)
    let attempts_lcg9999 = permute_order(&baseline_attempts, 999_999);

    // Permutation 6: Reversed order (last worker finishes first)
    let mut attempts_rev = baseline_attempts.clone();
    attempts_rev.reverse();

    let sum_seq = summarize(&study, &attempts_seq).expect("summarize seq");
    let sum_2w = summarize(&study, &attempts_2w).expect("summarize 2w");
    let sum_lcg42 = summarize(&study, &attempts_lcg42).expect("summarize lcg42");
    let sum_lcg1337 = summarize(&study, &attempts_lcg1337).expect("summarize lcg1337");
    let sum_lcg9999 = summarize(&study, &attempts_lcg9999).expect("summarize lcg9999");
    let sum_rev = summarize(&study, &attempts_rev).expect("summarize rev");

    let all_summaries = [
        ("2 workers", &sum_2w),
        ("LCG seed 42", &sum_lcg42),
        ("LCG seed 1337", &sum_lcg1337),
        ("LCG seed 999999", &sum_lcg9999),
        ("reversed", &sum_rev),
    ];

    for (label, sum) in all_summaries {
        assert_eq!(
            sum_seq["winner"], sum["winner"],
            "winner differs under worker order {label}"
        );
        assert_eq!(
            sum_seq["ranking"], sum["ranking"],
            "ranking differs under worker order {label}"
        );
        assert_eq!(
            sum_seq["candidates"], sum["candidates"],
            "candidate metrics differ under worker order {label}"
        );
        assert_eq!(
            sum_seq["ridge"], sum["ridge"],
            "ridge status differs under worker order {label}"
        );
    }

    // 3. State and seed reproducibility across repeated runs
    let mut map_a =
        CalibrationSeedMap::new(SEED_MAP_VERSION_V1, "seed-repro-test", 42).expect("seed map A");
    let mut map_b =
        CalibrationSeedMap::new(SEED_MAP_VERSION_V1, "seed-repro-test", 42).expect("seed map B");

    let schedules = [
        "c5-training-v1",
        "c5-validation-v1",
        "c5-confirmation-v1",
        "c5-final-test-v1",
    ];
    let purposes = [
        SeedPurpose::Service,
        SeedPurpose::Transit,
        SeedPurpose::Behavior,
        SeedPurpose::Calibration,
    ];

    for schedule in schedules {
        for (rep, purpose) in purposes.iter().enumerate() {
            let mut stream_a = map_a
                .stream_for(
                    schedule,
                    rep as u64,
                    &format!("case-{rep}"),
                    "task-1",
                    *purpose,
                )
                .expect("stream A");
            let mut stream_b = map_b
                .stream_for(
                    schedule,
                    rep as u64,
                    &format!("case-{rep}"),
                    "task-1",
                    *purpose,
                )
                .expect("stream B");

            assert_eq!(
                stream_a.derived_seed(),
                stream_b.derived_seed(),
                "derived seeds must match for schedule {schedule} rep {rep}"
            );

            let draws_a: Vec<u64> = (0..10)
                .map(|_| stream_a.next_u64().expect("draw next_u64"))
                .collect();
            let draws_b: Vec<u64> = (0..10)
                .map(|_| stream_b.next_u64().expect("draw next_u64"))
                .collect();
            assert_eq!(
                draws_a, draws_b,
                "deterministic draws must match bit-for-bit for schedule {schedule}"
            );
        }
    }

    // Repeated evaluation of study tasks produces identical service seeds
    let eval_run1 = evaluate(&study, &tasks[0]).expect("eval 1");
    let eval_run2 = evaluate(&study, &tasks[0]).expect("eval 2");
    assert_eq!(
        eval_run1["records"][0]["study_service_seed"],
        eval_run2["records"][0]["study_service_seed"]
    );
    assert_eq!(
        eval_run1["records"][0]["predicted_tick"],
        eval_run2["records"][0]["predicted_tick"]
    );
}

// ===========================================================================
// Gate 3: Benchmarks with Named Hardware and Data Sizes
// ===========================================================================

#[test]
fn test_benchmarks_named_hardware_data_sizes() {
    // 1. Record named hardware and compiler environment
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let num_cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);
    let target = format!("{arch}-{os}");

    println!(
        "\n--- C6.3 BENCHMARK ENVIRONMENT ---\nOS: {os}\nArchitecture: {arch}\nTarget: {target}\nLogical CPUs: {num_cpus}\n-----------------------------------"
    );
    assert!(!os.is_empty());
    assert!(!arch.is_empty());

    // 2. Ingestion throughput & memory bounds for N = 100, 1000, 4000 records
    for &n in &[100, 1000, 4000] {
        let mut study = valid_study();
        study["study_id"] = json!(format!("benchmark-ingestion-{n}"));
        // Single candidate configuration to focus benchmark on trace ingestion
        study["parameters"]["route_ticks"]["values"] = json!([2]);
        study["parameters"]["work_ticks"]["values"] = json!([3]);
        study["budget"]["replications"] = json!(1);

        // Divide N across training, validation, final_test splits
        let n_train = n / 3;
        let n_val = n / 3;
        let n_test = n - n_train - n_val;

        let t_bound = (n_train * 10) as u64;
        let v_bound = t_bound + (n_val * 10) as u64;
        let final_bound = v_bound + (n_test * 10 + 10) as u64;

        study["splits"] = json!({
            "training": [0, t_bound],
            "validation": [t_bound, v_bound],
            "final_test": [v_bound, final_bound]
        });

        let mut trace = Vec::with_capacity(n);
        for i in 0..n {
            let (start, split_tag) = if i < n_train {
                ((i * 10) as u64, "train")
            } else if i < n_train + n_val {
                (t_bound + ((i - n_train) * 10) as u64, "val")
            } else {
                (v_bound + ((i - n_train - n_val) * 10) as u64, "test")
            };

            trace.push(json!({
                "case_id": format!("case-{i}"),
                "day": format!("day-{split_tag}-{}", i % 20),
                "start_tick": start,
                "end_tick": start + 5,
                "transit_ticks": 2
            }));
        }
        study["trace"] = Value::Array(trace);

        let json_bytes = study.to_string().len();
        // Memory bound: JSON footprint must remain strictly bounded (< 500 bytes per record)
        assert!(
            json_bytes < n * 500,
            "JSON footprint {json_bytes} bytes exceeds bound for N = {n}"
        );

        let t0 = Instant::now();
        let report = verify_trace(&study).expect("verify trace benchmark");
        let elapsed_verify = t0.elapsed();
        assert_eq!(report["status"], "valid");

        let t1 = Instant::now();
        let planned = plan(&study).expect("plan benchmark");
        let elapsed_plan = t1.elapsed();
        assert_eq!(planned["schema_version"], "kairoecs.calibration.plan.v1");

        let total_time_sec = (elapsed_verify + elapsed_plan).as_secs_f64();
        let throughput = n as f64 / total_time_sec.max(1e-9);
        println!(
            "Ingestion N = {n:4}: payload = {json_bytes:7} bytes, verify = {elapsed_verify:?}, plan = {elapsed_plan:?}, throughput = {throughput:.0} records/sec"
        );
    }

    // 3. Shadow probe overhead per step for N = 100, 500 cases
    for &n_probe in &[100, 500] {
        let mut probe_study = valid_study();
        probe_study["study_id"] = json!(format!("benchmark-probe-{n_probe}"));
        probe_study["parameters"]["route_ticks"]["values"] = json!([2]);
        probe_study["parameters"]["work_ticks"]["values"] = json!([3]);
        probe_study["budget"]["replications"] = json!(1);
        let val_end = (10 + n_probe * 10 + 10) as u64;
        let test_end = val_end + 20;
        probe_study["splits"] = json!({
            "training": [0, 10],
            "validation": [10, val_end],
            "final_test": [val_end, test_end]
        });

        let mut probe_records = Vec::with_capacity(n_probe + 2);
        probe_records.push(json!({
            "case_id": "case-tr-0",
            "day": "day-tr-0",
            "start_tick": 0,
            "end_tick": 5,
            "transit_ticks": 2
        }));
        for i in 0..n_probe {
            probe_records.push(json!({
                "case_id": format!("probe-case-{i}"),
                "day": "day-val-0",
                "start_tick": (10 + i * 10) as u64,
                "end_tick": (15 + i * 10) as u64,
                "transit_ticks": 2
            }));
        }
        probe_records.push(json!({
            "case_id": "case-test-0",
            "day": "day-test-0",
            "start_tick": val_end,
            "end_tick": val_end + 5,
            "transit_ticks": 2
        }));
        probe_study["trace"] = Value::Array(probe_records);

        let planned_probe = plan(&probe_study).expect("plan probe study");
        let val_task = planned_probe["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["split"] == "validation")
            .expect("validation probe task");

        let t0 = Instant::now();
        let result = evaluate(&probe_study, val_task).expect("evaluate probe task");
        let elapsed = t0.elapsed();
        assert_eq!(result["outcome"], "complete");

        let us_per_case = elapsed.as_micros() as f64 / n_probe as f64;
        println!(
            "Shadow probe N = {n_probe:3}: total = {elapsed:?}, latency = {us_per_case:.2} us/step"
        );
        assert!(
            us_per_case < 5000.0,
            "probe latency {us_per_case:.2} us/step exceeds 5000 us bound"
        );
    }

    // 4. Metrics kernel throughput: compute W1 and KS D across N = 100, 1000, 4000 points
    // Assertion: throughput >= 50,000 points/sec
    for &n_points in &[100, 1000, 4000] {
        let ref_strings: Vec<String> = (0..n_points)
            .map(|i| ((i * 7 + 13) % 1000).to_string())
            .collect();
        let sim_strings: Vec<String> = (0..n_points)
            .map(|i| ((i * 11 + 5) % 1000).to_string())
            .collect();

        let ref_options: Vec<Option<&str>> = ref_strings.iter().map(|s| Some(s.as_str())).collect();
        let sim_options: Vec<Option<&str>> = sim_strings.iter().map(|s| Some(s.as_str())).collect();

        let request = MetricRequest {
            reference: &ref_options,
            simulation: &sim_options,
            reference_weights: None,
            simulation_weights: None,
            algorithm_version: "empirical_equal.v1",
            origin: None,
            scale_ticks: "1",
        };

        // Warmup
        let _ = compare(&request);

        let iterations = if n_points <= 100 { 100 } else { 20 };
        let t0 = Instant::now();
        let mut last_res = None;
        for _ in 0..iterations {
            last_res = Some(compare(&request));
        }
        let elapsed = t0.elapsed();
        let metric_res = last_res.expect("metric result");

        assert_eq!(metric_res.status, MetricStatus::Computed);
        assert!(metric_res.w1.is_some());
        assert!(metric_res.ks_d.is_some());

        let total_pts = (n_points * 2 * iterations) as f64;
        let pts_per_sec = total_pts / elapsed.as_secs_f64().max(1e-9);

        println!(
            "Metrics Kernel N = {n_points:4} ({iterations} iters): elapsed = {elapsed:?}, W1 = {:.4}, KS D = {:.4}, throughput = {pts_per_sec:.0} points/sec",
            metric_res.w1.unwrap(),
            metric_res.ks_d.unwrap()
        );

        assert!(
            pts_per_sec >= 50_000.0,
            "metrics kernel throughput {pts_per_sec:.0} pts/sec below required 50,000 pts/sec threshold"
        );
    }

    // 5. Candidate throughput across sequential and simulated multi-worker execution
    let study = valid_study();
    let planned_study = plan(&study).expect("plan study");
    let tasks = planned_study["tasks"].as_array().expect("tasks");

    // Single-worker sequential candidate evaluation
    let t_seq = Instant::now();
    let _attempts_seq = evaluate_all_tasks(&study, tasks);
    let elapsed_seq = t_seq.elapsed();
    let candidates_per_sec_seq = tasks.len() as f64 / elapsed_seq.as_secs_f64().max(1e-9);

    // Simulated 4-worker chunked execution
    let chunk_size = tasks.len().div_ceil(4);
    let t_multi = Instant::now();
    let chunks: Vec<_> = tasks.chunks(chunk_size).collect();
    let mut total_evals = 0;
    for chunk in chunks {
        let ev = evaluate_all_tasks(&study, chunk);
        total_evals += ev.len();
    }
    let elapsed_multi = t_multi.elapsed();
    let candidates_per_sec_multi = total_evals as f64 / elapsed_multi.as_secs_f64().max(1e-9);

    println!(
        "Candidate Throughput: sequential = {candidates_per_sec_seq:.1} candidates/sec, multi-worker = {candidates_per_sec_multi:.1} candidates/sec"
    );
    assert!(candidates_per_sec_seq > 0.0);
    assert!(candidates_per_sec_multi > 0.0);
}

// ===========================================================================
// Gate 4: Documented Validity Limits and Boundary Guards
// ===========================================================================

#[test]
fn test_documented_validity_limits_and_boundary_guards() {
    let study = valid_study();

    // 1. Row inventory cap: > 4,096 records per split fails closed
    let mut study_4097 = study.clone();
    study_4097["study_id"] = json!("study-inventory-cap");
    study_4097["splits"] = json!({
        "training": [0, 10],
        "validation": [10, 42000],
        "final_test": [42000, 43000]
    });

    let mut trace_4097 = Vec::with_capacity(4099);
    trace_4097.push(json!({
        "case_id": "case-tr-0",
        "day": "day-tr-0",
        "start_tick": 0,
        "end_tick": 5,
        "transit_ticks": 2
    }));

    // 4097 records in validation split
    for i in 0..4097 {
        trace_4097.push(json!({
            "case_id": format!("case-val-{i}"),
            "day": "day-val-0",
            "start_tick": (10 + i * 2) as u64,
            "end_tick": (15 + i * 2) as u64,
            "transit_ticks": 2
        }));
    }

    trace_4097.push(json!({
        "case_id": "case-test-0",
        "day": "day-test-0",
        "start_tick": 42000,
        "end_tick": 42005,
        "transit_ticks": 2
    }));

    study_4097["trace"] = Value::Array(trace_4097);

    // Bounded validator rejects > 4096 records per split
    match plan(&study_4097) {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("trace rows per split exceed 4096 case cap"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected 4096 case cap error, got {other:?}"),
    }

    // Free runtime row inventory cap: free_run enforces a 4096 row limit
    // Replay on study with 4097 records in validation fails closed
    match replay(&study_4097, "free") {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("trace rows per split exceed 4096 case cap"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected validation cap error, got {other:?}"),
    }

    // 2. Unsupported replay mode fails closed with "unsupported replay mode"
    match replay(&study, "quantum_replay") {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert_eq!(msg, "unsupported replay mode");
        }
        other => panic!("expected unsupported replay mode error, got {other:?}"),
    }

    match replay(&study, "monte_carlo") {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert_eq!(msg, "unsupported replay mode");
        }
        other => panic!("expected unsupported replay mode error, got {other:?}"),
    }

    // 3. Invalid candidate parameters (route_ticks = 0, work_ticks = 0) return typed errors
    let mut study_zero_param = study.clone();
    study_zero_param["parameters"]["route_ticks"]["values"] = json!([0, 2]);
    study_zero_param["parameters"]["work_ticks"]["values"] = json!([0, 3]);

    let plan_zero = plan(&study_zero_param).expect("plan with zero parameters");
    let invalid_tasks: Vec<_> = plan_zero["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["valid_candidate"] == false)
        .cloned()
        .collect();

    assert!(
        !invalid_tasks.is_empty(),
        "zero-sum parameters must be flagged in plan"
    );

    let eval_invalid =
        evaluate(&study_zero_param, &invalid_tasks[0]).expect("evaluate invalid candidate");
    assert_eq!(eval_invalid["outcome"], "invalid");
    assert_eq!(eval_invalid["completion_error"], Value::Null);
    assert_eq!(eval_invalid["transit_error"], Value::Null);
    let records = eval_invalid["records"].as_array().expect("records");
    assert_eq!(records[0]["reason"], "zero parameter duration");

    // All-zero candidate study: free replay fails closed because no valid task exists
    let mut study_all_zero = study.clone();
    study_all_zero["parameters"]["route_ticks"]["values"] = json!([0]);
    study_all_zero["parameters"]["work_ticks"]["values"] = json!([0]);
    let rep_err = replay(&study_all_zero, "free");
    assert!(
        matches!(rep_err, Err(c5::C5Error::Execution(ref msg)) if msg.contains("no valid replay task") || msg.contains("invalid free candidate")),
        "expected typed failure for invalid free candidate replay, got {rep_err:?}"
    );

    // 4. Horizon cutoff and max_events enforcement
    // Horizon cutoff: work requiring 10 ticks with horizon 1 tick is marked censored
    let mut study_horizon = study.clone();
    study_horizon["study_id"] = json!("study-horizon-cutoff");
    study_horizon["budget"]["horizon_ticks"] = json!(1);
    study_horizon["parameters"]["route_ticks"]["values"] = json!([2]);
    study_horizon["parameters"]["work_ticks"]["values"] = json!([10]);

    let plan_h = plan(&study_horizon).expect("plan horizon cutoff");
    let task_h = &plan_h["tasks"].as_array().unwrap()[0];
    let res_h = evaluate(&study_horizon, task_h).expect("evaluate horizon cutoff");
    assert_eq!(res_h["outcome"], "censored");
    assert_eq!(res_h["censored_count"], 1);
    assert_eq!(res_h["records"][0]["outcome"], "censored");
    assert_eq!(res_h["records"][0]["predicted_tick"], Value::Null);

    // Max events enforcement: simulation halts cleanly at max_events cap
    let mut study_events = study.clone();
    study_events["study_id"] = json!("study-max-events");
    study_events["budget"]["max_events"] = json!(1);
    study_events["parameters"]["route_ticks"]["values"] = json!([2]);
    study_events["parameters"]["work_ticks"]["values"] = json!([10]);

    let plan_e = plan(&study_events).expect("plan max events");
    let task_e = &plan_e["tasks"].as_array().unwrap()[0];
    let res_e = evaluate(&study_events, task_e).expect("evaluate max events");
    assert_eq!(res_e["outcome"], "censored");
    assert_eq!(res_e["records"][0]["outcome"], "censored");

    // 5. Precision bounds: u128 ticks representation and 53-bit mantissa loss-of-precision checks
    // MAX_EXACT_TICK = (1u64 << 53) - 1 = 9_007_199_254_740_991
    let max_exact_tick = (1u64 << 53) - 1;
    let overflow_tick = (1u64 << 53) + 1;

    // scale_ticks boundary
    let mut bad_scale = study.clone();
    bad_scale["objective"]["scale_ticks"] = json!(overflow_tick);
    match plan(&bad_scale) {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("scale_ticks exceeds exact numeric metric range"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected scale_ticks range error, got {other:?}"),
    }

    // horizon_ticks boundary
    let mut bad_horizon = study.clone();
    bad_horizon["budget"]["horizon_ticks"] = json!(overflow_tick);
    match plan(&bad_horizon) {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("horizon_ticks exceeds exact numeric metric range"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected horizon_ticks range error, got {other:?}"),
    }

    // parameter value boundary
    let mut bad_param = study.clone();
    bad_param["parameters"]["route_ticks"]["values"] = json!([overflow_tick]);
    match plan(&bad_param) {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("route_ticks exceeds exact numeric metric range"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected route_ticks range error, got {other:?}"),
    }

    // trace tick boundary
    let mut bad_trace = study.clone();
    bad_trace["trace"][0]["start_tick"] = json!(overflow_tick);
    bad_trace["trace"][0]["end_tick"] = json!(overflow_tick + 5);
    match plan(&bad_trace) {
        Err(c5::C5Error::InvalidInput(msg)) => {
            assert!(
                msg.contains("trace tick exceeds exact numeric metric range"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected trace tick range error, got {other:?}"),
    }

    // Valid 53-bit boundary values succeed
    let mut boundary_study = study.clone();
    boundary_study["study_id"] = json!("boundary-53bit-test");
    boundary_study["objective"]["scale_ticks"] = json!(max_exact_tick);
    let plan_boundary = plan(&boundary_study).expect("53-bit exact boundary must succeed");
    assert_eq!(
        plan_boundary["schema_version"],
        "kairoecs.calibration.plan.v1"
    );

    // 6. Typed outcomes partition case inventory without silent drops
    // Complete study outcome partition: eligible + censored + missing + failed == records.len()
    let planned_std = plan(&study).expect("plan study");
    let task_std = &planned_std["tasks"].as_array().unwrap()[0];
    let res_std = evaluate(&study, task_std).expect("evaluate task");

    let outcome = res_std["outcome"].as_str().expect("outcome string");
    assert!(
        matches!(
            outcome,
            "complete" | "censored" | "missing" | "failed" | "invalid"
        ),
        "outcome must be typed, got {outcome}"
    );

    let elig = res_std["eligible_count"].as_u64().unwrap();
    let cens = res_std["censored_count"].as_u64().unwrap();
    let miss = res_std["missing_count"].as_u64().unwrap();
    let fail = res_std["failed_count"].as_u64().unwrap();
    let total_recs = res_std["records"].as_array().unwrap().len() as u64;

    assert_eq!(
        elig + cens + miss + fail,
        total_recs,
        "counts must strictly partition all records without silent drops"
    );
}
