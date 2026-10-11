use super::*;

fn study() -> Value {
    json!({
        "schema_version": STUDY_SCHEMA,
        "study_id":"study-test", "dataset_id":"synthetic", "mapping_version":"normalized-ticks.v1",
        "adapter":"synthetic_flow_probe.v1", "root_seed":42,
        "parameters":{
            "route_ticks":{"unit":"tick","values":[3,1,2]},
            "work_ticks":{"unit":"tick","values":[4,2,3]}
        },
        "objective":{"completion_weight":1,"transit_weight":1,"scale_ticks":1,"failure_penalty":100,"tie_tolerance":0},
        "budget":{"max_candidates":20,"replications":1,"confirmation_replications":2,"final_test_replications":2,"max_tasks":1000,"horizon_ticks":1000,"max_events":1000},
        "precision":{"kind":"fixed_confirmation","target_half_width":1,"alpha":0.05},
        "splits":{"training":[0,6],"validation":[6,10],"final_test":[10,20]},
        "trace":[
            {"case_id":"train-event-1","day":"day-a","start_tick":0,"end_tick":1,"transit_ticks":0},
            {"case_id":"train-event-2","day":"day-b","start_tick":1,"end_tick":3,"transit_ticks":0},
            {"case_id":"train-censor","day":"day-b","start_tick":2,"end_tick":6,"transit_ticks":0},
            {"case_id":"validation","day":"day-c","start_tick":6,"end_tick":8,"transit_ticks":1},
            {"case_id":"final","day":"day-d","start_tick":10,"end_tick":12,"transit_ticks":1}
        ]
    })
}

fn result_for(task: &Value, outcome: &str) -> Value {
    let params = &task["params"];
    let sum =
        params["route_ticks"].as_u64().unwrap_or(0) + params["work_ticks"].as_u64().unwrap_or(0);
    let score = sum.abs_diff(5);
    let task_records = task["records"].as_array().unwrap();
    let records: Vec<Value> = task_records
        .iter()
        .map(|r| {
            let input_censored = r["censored"] == true;
            let valid = outcome == "complete" || outcome == "censored";
            let row_outcome = if valid {
                "complete"
            } else if outcome == "invalid" {
                "invalid"
            } else {
                "failed"
            };
            let mut rec = json!({"case_id":r["case_id"],"outcome":row_outcome,"observed_censored":input_censored});
            if valid {
                let observed = r["end_tick"].as_u64().unwrap_or(0);
                rec["predicted_tick"] = json!((observed + score).to_string());
                rec["observed_tick"] = r["end_tick"].clone();
                rec["study_service_seed"] = json!(1);
                rec["native_events"] = json!(0);
                rec["predicted_transit_ticks"] = r["transit_ticks"].as_u64().map(|n| json!(n.to_string())).unwrap_or(Value::Null);
            }
            rec
        })
        .collect();
    let mut eligible_count = 0usize;
    let mut censored_count = 0usize;
    let mut missing_count = 0usize;
    let mut failed_count = 0usize;
    let mut completion_errors = Vec::new();
    let mut transit_errors = Vec::new();
    for (record, source) in records.iter().zip(task_records) {
        if ["failed", "invalid"].contains(&record["outcome"].as_str().unwrap()) {
            failed_count += 1;
        } else if source["censored"] == true || record["outcome"] == "censored" {
            censored_count += 1;
        } else if record["outcome"] == "missing" {
            missing_count += 1;
        } else {
            eligible_count += 1;
            let predicted = record["predicted_tick"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let observed = source["end_tick"].as_u64().unwrap();
            completion_errors.push(predicted.abs_diff(observed) as f64);
        }
        if let (Some(a), Some(b)) = (
            source["transit_ticks"].as_u64(),
            record["predicted_transit_ticks"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok()),
        ) {
            transit_errors.push(a.abs_diff(b) as f64);
        }
    }
    let completion_error = (!completion_errors.is_empty()).then(|| mean(&completion_errors));
    let transit_error = (!transit_errors.is_empty()).then(|| mean(&transit_errors));
    json!({
        "schema_version":ATTEMPT_SCHEMA,
        "task_id":task["task_id"],"study_hash":task["study_hash"],"candidate_id":task["candidate_id"],
        "replication_id":task["replication_id"],"split":task["split"],"seed_schedule_id":task["seed_schedule_id"],"params":params,
        "outcome":outcome,"completion_error":completion_error,"transit_error":transit_error,
        "eligible_count":eligible_count,"censored_count":censored_count,"missing_count":missing_count,"failed_count":failed_count,"records":records
    })
}

fn complete_attempts(_config: &Value, plan: &Value) -> Vec<Value> {
    plan["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let outcome = if t["valid_candidate"] == false {
                "invalid"
            } else if t["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["censored"] == true)
            {
                "censored"
            } else {
                "complete"
            };
            result_for(t, outcome)
        })
        .collect()
}

#[test]
fn plan_is_canonical_bounded_and_pairs_candidate_independent_schedules() {
    let cfg = study();
    let a = plan(&cfg).unwrap();
    let b = plan(&cfg).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        a["candidates"][0]["params"],
        json!({"route_ticks":1,"work_ticks":2})
    );
    let tasks = a["tasks"].as_array().unwrap();
    assert!(tasks
        .windows(2)
        .all(|w| w[0]["task_id"].as_str() < w[1]["task_id"].as_str()));
    let training: Vec<_> = tasks.iter().filter(|t| t["split"] == "training").collect();
    assert_eq!(training.len(), 9);
    let one: Vec<_> = training
        .iter()
        .filter(|t| t["candidate_id"] == "c000000")
        .collect();
    let two: Vec<_> = training
        .iter()
        .filter(|t| t["candidate_id"] == "c000001")
        .collect();
    assert_eq!(one[0]["seed_schedule_id"], two[0]["seed_schedule_id"]);
    assert_eq!(one[0]["records"], two[0]["records"]);
    let confirmation = tasks.iter().find(|t| t["split"] == "confirmation").unwrap();
    assert_eq!(confirmation["records"][0]["case_id"], "validation");
    assert_eq!(confirmation["seed_schedule_id"], SCHEDULES[2]);
    assert!(plan(&{
        let mut c = cfg.clone();
        c["budget"]["max_tasks"] = json!(1);
        c
    })
    .is_err());
}

#[test]
fn invalid_zero_candidate_is_retained_as_attempt_not_enumeration_hole() {
    let mut cfg = study();
    cfg["parameters"]["route_ticks"]["values"] = json!([0, 2]);
    cfg["parameters"]["work_ticks"]["values"] = json!([0, 3]);
    let p = plan(&cfg).unwrap();
    assert_eq!(p["candidates"].as_array().unwrap().len(), 4);
    let invalid = p["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["valid_candidate"] == false)
        .count();
    assert_eq!(invalid, 12);
    assert!(p["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["params"] == json!({"route_ticks":0,"work_ticks":0})));
}

#[test]
fn result_inventory_and_identity_are_exact_and_duplicate_attempts_fail() {
    let cfg = study();
    let p = plan(&cfg).unwrap();
    let mut attempts = complete_attempts(&cfg, &p);
    assert!(summarize(&cfg, &attempts).is_ok());
    attempts.pop();
    assert!(summarize(&cfg, &attempts)
        .unwrap_err()
        .contains("incomplete"));
    let mut attempts = complete_attempts(&cfg, &p);
    attempts[0]["params"]["route_ticks"] = json!(999);
    assert!(summarize(&cfg, &attempts)
        .unwrap_err()
        .contains("identity mismatch"));
    let mut attempts = complete_attempts(&cfg, &p);
    attempts[0]["completion_error"] = json!(999);
    assert!(summarize(&cfg, &attempts)
        .unwrap_err()
        .contains("differs from successful per-case"));
    let mut attempts = complete_attempts(&cfg, &p);
    attempts.push(attempts[0].clone());
    assert!(summarize(&cfg, &attempts)
        .unwrap_err()
        .contains("duplicate"));
}

#[test]
fn required_missing_transit_metric_moves_observation_to_penalty_bucket() {
    let cfg = study();
    let p = plan(&cfg).unwrap();
    let mut attempts = complete_attempts(&cfg, &p);
    let task = p["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["split"] == "validation" && t["candidate_id"] == "c000000")
        .unwrap();
    let result = attempts
        .iter_mut()
        .find(|r| r["task_id"] == task["task_id"])
        .unwrap();
    result["records"][0]["predicted_transit_ticks"] = Value::Null;
    result["outcome"] = json!("missing");
    result["completion_error"] = Value::Null;
    result["transit_error"] = Value::Null;
    result["eligible_count"] = json!(0);
    result["missing_count"] = json!(1);
    let summary = summarize(&cfg, &attempts).unwrap();
    let candidate = summary["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["candidate_id"] == "c000000")
        .unwrap();
    assert!(candidate["validation_objective_mean"].as_f64().unwrap() >= 100.0);
}

#[test]
fn mixed_failure_and_source_censoring_are_exclusive_and_case_penalized() {
    let cfg = study();
    let p = plan(&cfg).unwrap();
    let mut attempts = complete_attempts(&cfg, &p);
    let task = p["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["candidate_id"] == "c000000" && t["split"] == "training")
        .unwrap();
    let result_index = attempts
        .iter()
        .position(|r| r["task_id"] == task["task_id"])
        .unwrap();
    let mut result = attempts[result_index].clone();
    result["records"][1]["outcome"] = json!("failed");
    result["records"][1]
        .as_object_mut()
        .unwrap()
        .remove("predicted_tick");
    result["records"][1]
        .as_object_mut()
        .unwrap()
        .remove("predicted_transit_ticks");
    result["records"][1]
        .as_object_mut()
        .unwrap()
        .remove("study_service_seed");
    result["records"][1]
        .as_object_mut()
        .unwrap()
        .remove("native_events");
    result["records"][1]["reason"] = json!("fixture failure");
    result["outcome"] = json!("failed");
    result["eligible_count"] = json!(1);
    result["censored_count"] = json!(1);
    result["missing_count"] = json!(0);
    result["failed_count"] = json!(1);
    attempts[result_index] = result.clone();
    // One eligible row has normalized objective error 2; two uncovered rows
    // receive the declared penalty, regardless of the aggregate outer outcome.
    let expected = (2.0 + 2.0 * 100.0) / 3.0;
    assert!(summarize(&cfg, &attempts).is_ok());
    assert!(
        (task_score(&result, &canonical_config(&cfg).unwrap()).unwrap() - expected).abs() < 1e-12
    );

    let mut forged = attempts.clone();
    let bad = forged
        .iter_mut()
        .find(|r| r["task_id"] == task["task_id"])
        .unwrap();
    bad["eligible_count"] = json!(2);
    bad["failed_count"] = json!(0);
    assert!(summarize(&cfg, &forged)
        .unwrap_err()
        .contains("exclusive case outcome partition"));
}

#[test]
fn validation_ridge_freezes_winner_then_final_test_is_winner_only() {
    let cfg = study();
    let p = plan(&cfg).unwrap();
    let attempts = complete_attempts(&cfg, &p);
    let s = summarize(&cfg, &attempts).unwrap();
    assert_eq!(s["winner"], "c000002"); // first canonical member of (1,4),(2,3),(3,2)
    assert_eq!(s["ridge"], true);
    assert_eq!(s["identifiability"], "non_identifiable_ridge");
    let winner = s["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["candidate_id"] == "c000002")
        .unwrap();
    assert_eq!(
        winner["bound_hits"],
        json!({"route_ticks":true,"work_ticks":true})
    );
    assert_eq!(s["next_tasks"].as_array().unwrap().len(), 2);
    assert!(s["next_tasks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["candidate_id"] == "c000002"
            && t["split"] == "final_test"
            && t["seed_schedule_id"] == SCHEDULES[3]));
    let final_tasks = final_tasks(&cfg, "c000002").unwrap();
    assert_eq!(s["next_tasks"], json!(final_tasks));
    let mut all = attempts;
    all.extend(final_tasks.iter().map(|t| result_for(t, "complete")));
    let finished = summarize(&cfg, &all).unwrap();
    assert_eq!(finished["complete"], true);
    assert_eq!(finished["next_tasks"], json!([]));
    assert_eq!(finished["winner"], s["winner"]);
}

#[test]
fn failed_probe_penalty_affects_validation_and_invalid_attempt_remains_visible() {
    let cfg = study();
    let p = plan(&cfg).unwrap();
    let mut attempts = complete_attempts(&cfg, &p);
    for task in p["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["candidate_id"] == "c000002" && t["split"] == "validation")
    {
        let idx = attempts
            .iter()
            .position(|a| a["task_id"] == task["task_id"])
            .unwrap();
        attempts[idx] = result_for(task, "failed");
    }
    let s = summarize(&cfg, &attempts).unwrap();
    assert_ne!(s["winner"], "c000002");
    assert_eq!(s["candidates"][2]["failure_count"], 2);
    let mut invalid_cfg = cfg.clone();
    invalid_cfg["parameters"]["route_ticks"]["values"] = json!([0, 1]);
    invalid_cfg["parameters"]["work_ticks"]["values"] = json!([0, 4]);
    let ip = plan(&invalid_cfg).unwrap();
    let ia = complete_attempts(&invalid_cfg, &ip);
    let is = summarize(&invalid_cfg, &ia).unwrap();
    assert_eq!(is["candidates"][0]["status"], "invalid_candidate");
}

#[test]
fn training_only_censoring_and_bootstrap_oracles_are_hand_computable() {
    let cfg = study();
    let p = plan(&cfg).unwrap();
    let d = &p["diagnostics"]["training"];
    assert_eq!(d["event_count"], 2);
    assert_eq!(d["censored_count"], 1);
    assert_eq!(d["exponential_mean_ticks"], json!(3.5));
    assert_eq!(d["exposure_ticks"], "7");
    assert!((d["event_rate_per_tick"].as_f64().unwrap() - 2.0 / 7.0).abs() < 1e-12);
    assert_eq!(bootstrap_mean_variance(&[0.0, 10.0]), Some(12.5));
    assert_eq!(bootstrap_mean_variance(&[0.0, 0.0, 10.0, 10.0]), Some(6.25));
    let mut grouped = study();
    grouped["splits"] = json!({"training":[0,20],"validation":[20,30],"final_test":[30,40]});
    grouped["trace"] = json!([
        {"case_id":"a1","day":"a","start_tick":0,"end_tick":0,"transit_ticks":0},
        {"case_id":"a2","day":"a","start_tick":1,"end_tick":1,"transit_ticks":0},
        {"case_id":"b1","day":"b","start_tick":0,"end_tick":10,"transit_ticks":0},
        {"case_id":"b2","day":"b","start_tick":0,"end_tick":10,"transit_ticks":0},
        {"case_id":"v","day":"v","start_tick":20,"end_tick":21,"transit_ticks":0},
        {"case_id":"f","day":"f","start_tick":30,"end_tick":31,"transit_ticks":0}
    ]);
    let grouped_diagnostics = &plan(&grouped).unwrap()["diagnostics"]["training"];
    assert_eq!(grouped_diagnostics["day_cluster_means"], json!([0.0, 10.0]));
    assert_eq!(
        grouped_diagnostics["day_cluster_bootstrap_variance"],
        json!(12.5)
    );
    assert_eq!(
        grouped_diagnostics["row_iid_bootstrap_variance"],
        json!(6.25)
    );
    let training = p["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["split"] == "training")
        .unwrap();
    let clipped = training["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["case_id"] == "train-censor")
        .unwrap();
    assert_eq!(clipped["exposure_ticks"], 4);
    assert_eq!(clipped["end_tick"], Value::Null);
    assert_eq!(clipped["censored"], true);
    assert_eq!(clipped["transit_ticks"], Value::Null);

    let mut cutoff = study();
    cutoff["splits"]["training"] = json!([0, 4]);
    cutoff["splits"]["validation"] = json!([6, 10]);
    cutoff["trace"].as_array_mut().unwrap().push(json!({
        "case_id":"train-cutoff", "day":"day-cutoff", "start_tick":3,
        "end_tick":6, "transit_ticks":0
    }));
    let cutoff_plan = plan(&cutoff).unwrap();
    let clipped_at_cutoff = cutoff_plan["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["split"] == "training")
        .unwrap()["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["case_id"] == "train-cutoff")
        .unwrap();
    assert_eq!(clipped_at_cutoff["start_tick"], 3);
    assert_eq!(clipped_at_cutoff["exposure_ticks"], 1);
    assert_eq!(clipped_at_cutoff["end_tick"], Value::Null);
    assert_eq!(clipped_at_cutoff["censored"], true);
    assert_eq!(clipped_at_cutoff["transit_ticks"], Value::Null);
}

#[test]
fn leakage_and_bad_configuration_are_rejected_before_dispatch() {
    let mut cfg = study();
    cfg["trace"][0]["day"] = json!("shared-day");
    cfg["trace"][3]["day"] = json!("shared-day");
    assert!(plan(&cfg).unwrap_err().contains("day cluster crosses"));
    let mut cfg = study();
    cfg["trace"][0]["start_tick"] = json!(2);
    cfg["trace"][0]["end_tick"] = json!(1);
    assert!(plan(&cfg).unwrap_err().contains("duration"));
    let mut cfg = study();
    cfg["parameters"]["route_ticks"]["unit"] = json!("minutes");
    assert!(plan(&cfg).unwrap_err().contains("tick parameter"));
    let mut cfg = study();
    cfg["trace"][0]["start_tick"] = json!(1);
    cfg["budget"]["horizon_ticks"] = json!(MAX_EXACT_TICK);
    assert!(plan(&cfg).unwrap_err().contains("start_tick plus horizon"));
}

#[test]
fn per_split_case_cap_is_checked_before_task_materialization() {
    let mut cfg = study();
    let mut rows = cfg["trace"].as_array().unwrap().clone();
    for i in 0..4094 {
        rows.push(json!({"case_id":format!("extra-{i}"),"day":format!("day-extra-{i}"),"start_tick":0,"end_tick":0,"transit_ticks":0}));
    }
    cfg["trace"] = json!(rows);
    assert!(plan(&cfg)
        .unwrap_err()
        .contains("trace rows per split exceed 4096"));
}

#[test]
fn fixed_confirmation_uses_bounded_hoeffding_and_reports_cap_indeterminacy() {
    let mut cfg = study();
    cfg["precision"]["target_half_width"] = json!(5);
    cfg["budget"]["confirmation_replications"] = json!(2);
    let p = plan(&cfg).unwrap();
    let attempts = complete_attempts(&cfg, &p);
    let s = summarize(&cfg, &attempts).unwrap();
    assert_eq!(
        s["precision"]["validity"],
        "bounded_score_hoeffding_no_optional_stopping"
    );
    assert_eq!(s["mc_indeterminate"], true);
    cfg["budget"]["confirmation_replications"] = json!(1000);
    cfg["budget"]["max_tasks"] = json!(10000);
    let p = plan(&cfg).unwrap();
    let attempts = complete_attempts(&cfg, &p);
    let s = summarize(&cfg, &attempts).unwrap();
    assert_eq!(s["mc_indeterminate"], false);
}

#[test]
fn oversized_repeated_case_payload_rejects_before_task_cloning() {
    let mut cfg = study();
    cfg["budget"]["replications"] = json!(100);
    cfg["budget"]["max_tasks"] = json!(10_000);
    let rows = cfg["trace"].as_array_mut().unwrap();
    for i in 0..60 {
        rows.push(json!({"case_id":format!("long-{i}-{}", "x".repeat(500)),
            "day":format!("long-day-{i}-{}", "y".repeat(500)),
            "start_tick":0,"end_tick":1,"transit_ticks":0}));
    }
    assert!(plan(&cfg)
        .unwrap_err()
        .contains("estimated plan output exceeds 16 MiB"));
}
