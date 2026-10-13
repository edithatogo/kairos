//! C5.1 integration fixtures for bounded grid search, split integrity, penalties,
//! tie order, resume invariance, and research-ledger oracles (C-05 / C-06).
//!
//! Worker shard equivalence for 1/2/N processes with seeded wall-clock delay schedules
//! is qualified in `kairo-ecs-cli` (`c5_workers` and `tests/c5_cli.rs`).

#![forbid(unsafe_code)]

use kairo_ecs_calibration::experimental_c5 as c5;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const STUDY_JSON: &str = include_str!("../../../examples/calibration/c5/study.json");

fn parse_value(raw: &str) -> Value {
    serde_json::from_str(raw).expect("fixture JSON")
}

fn plan(study: &Value) -> Value {
    parse_value(&c5::plan(&study.to_string()).expect("plan"))
}

fn summarize(study: &Value, attempts: &[Value]) -> Value {
    let attempts_json = serde_json::to_string(attempts).expect("attempts");
    parse_value(&c5::summarize(&study.to_string(), &attempts_json).expect("summarize"))
}

fn evaluate(study: &Value, task: &Value) -> Value {
    parse_value(&c5::evaluate(&study.to_string(), &task.to_string()).expect("evaluate"))
}

fn evaluate_all_tasks(study: &Value, tasks: &[Value]) -> Vec<Value> {
    tasks.iter().map(|task| evaluate(study, task)).collect()
}

fn complete_native_study(study: &Value) -> (Value, Vec<Value>) {
    let planned = plan(study);
    let tasks = planned["tasks"].as_array().expect("tasks").clone();
    let mut attempts = evaluate_all_tasks(study, &tasks);
    let mut summary = summarize(study, &attempts);
    while summary["complete"] == false {
        let next = summary["next_tasks"]
            .as_array()
            .expect("next_tasks")
            .clone();
        assert!(!next.is_empty(), "incomplete study without next tasks");
        attempts.extend(evaluate_all_tasks(study, &next));
        summary = summarize(study, &attempts);
    }
    (summary, attempts)
}

fn permute_attempts(attempts: &[Value], seed: u64) -> Vec<Value> {
    let mut order: Vec<usize> = (0..attempts.len()).collect();
    let mut state = seed;
    for i in (1..order.len()).rev() {
        state = state
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(0x6c07_2960_9d37_4e59);
        let j = (state % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order.iter().map(|idx| attempts[*idx].clone()).collect()
}

// --- fixture family: grid-search ---

#[test]
fn grid_search_native_pipeline_recovers_identifiable_candidate_on_bounded_grid() {
    let study = parse_value(STUDY_JSON);
    let planned = plan(&study);
    assert_eq!(planned["candidates"].as_array().unwrap().len(), 9);
    let (summary, _) = complete_native_study(&study);
    assert_eq!(summary["ridge"], false);
    assert_eq!(summary["winner"], "c000004");
    let winner = summary["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["candidate_id"] == summary["winner"])
        .expect("winner row");
    assert_eq!(winner["params"]["route_ticks"], 2);
    assert_eq!(winner["params"]["work_ticks"], 3);
    assert_eq!(summary["final_test"]["mean_completion_error"], 0.0);
}

// --- fixture family: invalid-candidate ---

#[test]
fn invalid_candidate_is_materialized_and_summarized_not_silently_dropped() {
    let mut study = parse_value(STUDY_JSON);
    study["parameters"]["route_ticks"]["values"] = json!([0, 2]);
    study["parameters"]["work_ticks"]["values"] = json!([0, 3]);
    let planned = plan(&study);
    let invalid_tasks = planned["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["valid_candidate"] == false)
        .count();
    assert!(
        invalid_tasks >= 4,
        "zero-sum candidates remain in the task inventory"
    );
    let attempts = evaluate_all_tasks(&study, planned["tasks"].as_array().expect("tasks"));
    let summary = summarize(&study, &attempts);
    let invalid = summary["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["candidate_id"] == "c000000")
        .expect("invalid candidate row");
    assert_eq!(invalid["status"], "invalid_candidate");
}

// --- fixture family: tie-ranking ---

#[test]
fn tie_ranking_uses_stable_candidate_id_when_objectives_match() {
    let mut study = parse_value(STUDY_JSON);
    study["parameters"]["route_ticks"]["values"] = json!([1, 2, 3]);
    study["parameters"]["work_ticks"]["values"] = json!([4, 3, 2]);
    study["objective"]["tie_tolerance"] = json!(0);
    study["objective"]["transit_weight"] = json!(0);
    for row in study["trace"].as_array_mut().unwrap() {
        row["transit_ticks"] = Value::Null;
    }
    let planned = plan(&study);
    let attempts = evaluate_all_tasks(&study, planned["tasks"].as_array().expect("tasks"));
    let summary = summarize(&study, &attempts);
    assert_eq!(summary["ridge"], true);
    assert_eq!(summary["winner"], "c000002");
    let ranking = summary["ranking"].as_array().expect("ranking");
    assert!(ranking.len() >= 2);
    assert_eq!(ranking[0]["score"], ranking[1]["score"]);
    assert!(
        ranking[0]["candidate_id"].as_str() < ranking[1]["candidate_id"].as_str(),
        "equal scores must tie-break by ascending candidate_id"
    );
}

// --- fixture family: split-leakage ---

#[test]
fn split_leakage_rejects_day_cluster_crossing_and_out_of_split_starts() {
    let mut study = parse_value(STUDY_JSON);
    study["trace"][0]["day"] = json!("shared-day");
    study["trace"][1]["day"] = json!("shared-day");
    assert!(
        c5::plan(&study.to_string())
            .unwrap_err()
            .to_string()
            .contains("day cluster crosses"),
        "patient/episode day clusters must not straddle splits"
    );

    let mut study = parse_value(STUDY_JSON);
    study["trace"].as_array_mut().unwrap().push(json!({
        "case_id":"future-leak",
        "day":"future-day",
        "start_tick":400,
        "end_tick":405,
        "transit_ticks":1
    }));
    assert!(
        c5::plan(&study.to_string())
            .unwrap_err()
            .to_string()
            .contains("outside declared splits"),
        "future leakage: trace rows must land inside declared split windows"
    );
}

// --- fixture family: failed-probe penalty ---

#[test]
fn failed_probe_penalty_blocks_candidate_from_winning_validation() {
    let study = parse_value(STUDY_JSON);
    let planned = plan(&study);
    let mut attempts = evaluate_all_tasks(&study, planned["tasks"].as_array().expect("tasks"));
    let target = planned["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["candidate_id"] == "c000004" && t["split"] == "validation")
        .expect("validation task for leading candidate");
    let idx = attempts
        .iter()
        .position(|a| a["task_id"] == target["task_id"])
        .unwrap();
    let mut failed = evaluate(&study, target);
    for record in failed["records"].as_array_mut().unwrap() {
        record["outcome"] = json!("failed");
        record.as_object_mut().unwrap().remove("predicted_tick");
        record
            .as_object_mut()
            .unwrap()
            .remove("predicted_transit_ticks");
        record.as_object_mut().unwrap().remove("study_service_seed");
        record.as_object_mut().unwrap().remove("native_events");
        record["reason"] = json!("injected validation failure");
    }
    failed["outcome"] = json!("failed");
    failed["completion_error"] = Value::Null;
    failed["transit_error"] = Value::Null;
    failed["eligible_count"] = json!(0);
    failed["censored_count"] = json!(0);
    failed["missing_count"] = json!(0);
    failed["failed_count"] = json!(failed["records"].as_array().unwrap().len());
    attempts[idx] = failed;

    let summary = summarize(&study, &attempts);
    assert_ne!(summary["winner"], "c000004");
    let penalized = summary["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["candidate_id"] == "c000004")
        .unwrap();
    assert!(penalized["failure_count"].as_u64().unwrap() >= 1);
}

// --- fixture family: resume / completion-order invariance ---

#[test]
fn resume_and_permuted_completion_order_preserve_canonical_summary() {
    let study = parse_value(STUDY_JSON);
    let planned = plan(&study);
    let tasks = planned["tasks"].as_array().expect("tasks");
    let attempts = evaluate_all_tasks(&study, tasks);
    let canonical = summarize(&study, &attempts);

    for seed in [1_u64, 7, 42, 99] {
        let shuffled = permute_attempts(&attempts, seed);
        let permuted = summarize(&study, &shuffled);
        assert_eq!(
            permuted, canonical,
            "summary must not depend on attempt order"
        );
    }

    let prefix = tasks.len() / 2;
    let partial = evaluate_all_tasks(&study, &tasks[..prefix]);
    let resumed = evaluate_all_tasks(&study, &tasks[prefix..]);
    let mut combined = partial;
    combined.extend(resumed);
    let resumed_summary = summarize(&study, &combined);
    assert_eq!(resumed_summary["ranking"], canonical["ranking"]);
    assert_eq!(resumed_summary["winner"], canonical["winner"]);
}

// --- research ledger: censoring rate 2/7 ---

#[test]
fn research_ledger_censoring_rate_two_sevenths_on_training_split() {
    let study = json!({
        "schema_version":"kairoecs.calibration.study.v1",
        "study_id":"c51-censor","dataset_id":"synthetic","mapping_version":"normalized-ticks.v1",
        "adapter":"synthetic_flow_probe.v1","root_seed":42,
        "parameters":{"route_ticks":{"unit":"tick","values":[1]},"work_ticks":{"unit":"tick","values":[4]}},
        "objective":{"completion_weight":1,"transit_weight":1,"scale_ticks":1,"failure_penalty":100,"tie_tolerance":0},
        "budget":{"max_candidates":4,"replications":1,"confirmation_replications":1,"final_test_replications":1,"max_tasks":256,"horizon_ticks":1000,"max_events":64},
        "precision":{"kind":"fixed_confirmation","target_half_width":100,"alpha":0.05},
        "splits":{"training":[0,6],"validation":[6,10],"final_test":[10,20]},
        "trace":[
            {"case_id":"e1","day":"d1","start_tick":1,"end_tick":2,"transit_ticks":0},
            {"case_id":"e2","day":"d2","start_tick":2,"end_tick":4,"transit_ticks":0},
            {"case_id":"censor","day":"d3","start_tick":2,"end_tick":6,"transit_ticks":0},
            {"case_id":"validation","day":"d4","start_tick":6,"end_tick":8,"transit_ticks":1},
            {"case_id":"final","day":"d5","start_tick":10,"end_tick":12,"transit_ticks":1}
        ]
    });
    let diagnostics = &plan(&study)["diagnostics"]["training"];
    assert_eq!(diagnostics["event_count"], 2);
    assert_eq!(diagnostics["censored_count"], 1);
    assert!((diagnostics["event_rate_per_tick"].as_f64().unwrap() - 2.0 / 7.0).abs() < 1e-12);
    assert_eq!(diagnostics["exponential_mean_ticks"], 3.5);
}

// --- research ledger: day vs row bootstrap ---

#[test]
fn research_ledger_day_cluster_bootstrap_exceeds_row_iid_variance() {
    let study = json!({
        "schema_version":"kairoecs.calibration.study.v1",
        "study_id":"c51-bootstrap","dataset_id":"synthetic","mapping_version":"normalized-ticks.v1",
        "adapter":"synthetic_flow_probe.v1","root_seed":42,
        "parameters":{"route_ticks":{"unit":"tick","values":[1]},"work_ticks":{"unit":"tick","values":[4]}},
        "objective":{"completion_weight":1,"transit_weight":1,"scale_ticks":1,"failure_penalty":100,"tie_tolerance":0},
        "budget":{"max_candidates":4,"replications":1,"confirmation_replications":1,"final_test_replications":1,"max_tasks":256,"horizon_ticks":1000,"max_events":64},
        "precision":{"kind":"fixed_confirmation","target_half_width":100,"alpha":0.05},
        "splits":{"training":[0,20],"validation":[20,30],"final_test":[30,40]},
        "trace":[
            {"case_id":"a1","day":"a","start_tick":0,"end_tick":0,"transit_ticks":0},
            {"case_id":"a2","day":"a","start_tick":1,"end_tick":1,"transit_ticks":0},
            {"case_id":"b1","day":"b","start_tick":0,"end_tick":10,"transit_ticks":0},
            {"case_id":"b2","day":"b","start_tick":0,"end_tick":10,"transit_ticks":0},
            {"case_id":"v","day":"v","start_tick":20,"end_tick":21,"transit_ticks":0},
            {"case_id":"f","day":"f","start_tick":30,"end_tick":31,"transit_ticks":0}
        ]
    });
    let diagnostics = &plan(&study)["diagnostics"]["training"];
    assert_eq!(diagnostics["day_cluster_bootstrap_variance"], 12.5);
    assert_eq!(diagnostics["row_iid_bootstrap_variance"], 6.25);
}

// --- research ledger: temporal cutoff ---

#[test]
fn research_ledger_temporal_cutoff_censors_future_completion() {
    let study = json!({
        "schema_version":"kairoecs.calibration.study.v1",
        "study_id":"c51-cutoff","dataset_id":"synthetic","mapping_version":"normalized-ticks.v1",
        "adapter":"synthetic_flow_probe.v1","root_seed":42,
        "parameters":{"route_ticks":{"unit":"tick","values":[1]},"work_ticks":{"unit":"tick","values":[4]}},
        "objective":{"completion_weight":1,"transit_weight":1,"scale_ticks":1,"failure_penalty":100,"tie_tolerance":0},
        "budget":{"max_candidates":4,"replications":1,"confirmation_replications":1,"final_test_replications":1,"max_tasks":256,"horizon_ticks":1000,"max_events":64},
        "precision":{"kind":"fixed_confirmation","target_half_width":100,"alpha":0.05},
        "splits":{"training":[0,4],"validation":[6,10],"final_test":[10,20]},
        "trace":[
            {"case_id":"train-cutoff","day":"day-cutoff","start_tick":3,"end_tick":6,"transit_ticks":0},
            {"case_id":"validation","day":"day-v","start_tick":6,"end_tick":8,"transit_ticks":1},
            {"case_id":"final","day":"day-f","start_tick":10,"end_tick":12,"transit_ticks":1}
        ]
    });
    let planned = plan(&study);
    let training_row = planned["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["split"] == "training")
        .unwrap()["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["case_id"] == "train-cutoff")
        .unwrap();
    assert_eq!(training_row["start_tick"], 3);
    assert_eq!(training_row["exposure_ticks"], 1);
    assert_eq!(training_row["end_tick"], Value::Null);
    assert_eq!(training_row["censored"], true);
}

// --- research ledger: paired CRN ---

#[test]
fn research_ledger_paired_crn_native_evaluations_repeat_per_candidate() {
    let study = parse_value(STUDY_JSON);
    let planned = plan(&study);
    let training: Vec<_> = planned["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["split"] == "training" && t["replication_id"] == 0)
        .take(2)
        .cloned()
        .collect();
    assert_eq!(
        training[0]["seed_schedule_id"],
        training[1]["seed_schedule_id"]
    );
    let first = evaluate(&study, &training[0]);
    let second = evaluate(&study, &training[1]);
    let seed_key = |r: &Value| {
        (
            r["records"][0]["study_service_seed"].clone(),
            r["records"][0]["native_events"].clone(),
        )
    };
    assert_eq!(
        seed_key(&first),
        seed_key(&second),
        "paired CRN aligns comparable candidates"
    );
    let again = evaluate(&study, &training[0]);
    assert_eq!(
        seed_key(&first),
        seed_key(&again),
        "CRN draws repeat for the same task"
    );
}

// --- research ledger: precision cap ---

#[test]
fn research_ledger_precision_cap_marks_mc_indeterminate_under_fixed_confirmation() {
    let mut study = parse_value(STUDY_JSON);
    study["precision"]["target_half_width"] = json!(5);
    study["budget"]["confirmation_replications"] = json!(2);
    let planned = plan(&study);
    let attempts = evaluate_all_tasks(&study, planned["tasks"].as_array().expect("tasks"));
    let summary = summarize(&study, &attempts);
    assert_eq!(
        summary["precision"]["validity"],
        "bounded_score_hoeffding_no_optional_stopping"
    );
    assert_eq!(summary["mc_indeterminate"], true);
}

#[test]
fn grid_search_fixture_ids_are_unique() {
    let names = [
        "grid_search_native_pipeline_recovers_identifiable_candidate_on_bounded_grid",
        "invalid_candidate_is_materialized_and_summarized_not_silently_dropped",
        "tie_ranking_uses_stable_candidate_id_when_objectives_match",
        "split_leakage_rejects_day_cluster_crossing_and_out_of_split_starts",
        "failed_probe_penalty_blocks_candidate_from_winning_validation",
        "resume_and_permuted_completion_order_preserve_canonical_summary",
        "research_ledger_censoring_rate_two_sevenths_on_training_split",
        "research_ledger_day_cluster_bootstrap_exceeds_row_iid_variance",
        "research_ledger_temporal_cutoff_censors_future_completion",
        "research_ledger_paired_crn_native_evaluations_repeat_per_candidate",
        "research_ledger_precision_cap_marks_mc_indeterminate_under_fixed_confirmation",
    ];
    let set: BTreeSet<_> = names.iter().collect();
    assert_eq!(set.len(), names.len());
}
