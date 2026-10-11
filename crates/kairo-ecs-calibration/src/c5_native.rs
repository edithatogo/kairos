//! C5 synthetic execution: actual Flow worlds, with study-owned CRN draws.
use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
use crate::shadow::{ObservedEvent, ProbeAdapter, ResourcePolicy, Transition};
use crate::shadow_c2_model::{SyntheticFlowConfig, SyntheticFlowProbeModel};
use crate::shadow_ledger::{InitialResource, LedgerLimits, ObservedLedger};
use crate::shadow_native::NativeFlowAdapter;
use crate::trace_order::{EventKindRank, TraceOrderKeyV1};
use kairo_ecs_des::{
    FlowCheckpointCodecs, FlowCheckpointLimits, FlowRuntime, WorkHandlers, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn error(e: impl std::fmt::Debug) -> String {
    format!("native: {e:?}")
}
fn number(v: &Value, key: &str) -> Result<u64, String> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("invalid integer {key}"))
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("invalid string {key}"))
}
fn digest(v: &Value) -> String {
    Sha256::digest(v.to_string().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn verify_trace(config: &Value) -> Result<Value, String> {
    let p = crate::c5_study::plan(config)?;
    let tasks = p["tasks"].as_array().ok_or("plan tasks")?;
    let mut splits = BTreeMap::new();
    for task in tasks {
        if task["split"] == "confirmation" {
            continue;
        }
        splits
            .entry(string(task, "split")?.to_owned())
            .or_insert_with(|| task["records"].clone());
    }
    Ok(
        json!({"schema_version":"kairoecs.calibration.trace-report.v1","status":"valid","study_hash":p["study_hash"],"dataset_id":config["dataset_id"],"mapping_version":config["mapping_version"],"normalized_partitions":splits,"adapter":"synthetic_flow_probe.v1","scope":"synthetic normalized integer tick records"}),
    )
}

pub(crate) fn admitted(config: &Value, task: &Value) -> Result<(), String> {
    let p = crate::c5_study::plan(config)?;
    let mut expected = p["tasks"].as_array().ok_or("plan tasks")?.clone();
    if task["split"] == "final_test" {
        expected.extend(crate::c5_study::final_tasks(
            config,
            string(task, "candidate_id")?,
        )?);
    }
    if !expected.iter().any(|t| t == task) {
        return Err("task does not match immutable study plan".into());
    }
    Ok(())
}

fn sampled_work(config: &Value, task: &Value, row: &Value) -> Result<(u128, u64), String> {
    let mut seeds =
        CalibrationSeedMap::new(1, string(config, "study_id")?, number(config, "root_seed")?)
            .map_err(error)?;
    let mut stream = seeds
        .stream_for(
            string(task, "seed_schedule_id")?,
            number(task, "replication_id")?,
            string(row, "case_id")?,
            "service",
            SeedPurpose::Service,
        )
        .map_err(error)?;
    let seed = stream.derived_seed();
    let noise = config.get("service_noise_ticks").and_then(Value::as_array);
    let perturbation = if let Some(noise) = noise {
        if noise.is_empty() || noise.len() > 4096 {
            return Err("invalid noise inventory".into());
        }
        // Rejection sampling avoids modulo bias; draw count is bounded independently.
        let n = noise.len() as u64;
        let threshold = n.wrapping_neg() % n;
        let mut selected = None;
        for _ in 0..128 {
            let draw = stream.next_u64().map_err(error)?;
            if draw >= threshold {
                selected = Some((draw % n) as usize);
                break;
            }
        }
        noise[selected.ok_or("noise rejection cap")?]
            .as_i64()
            .ok_or("signed integer noise required")?
    } else {
        0
    };
    let ticks = i128::from(number(&task["params"], "work_ticks")?) + i128::from(perturbation);
    if ticks <= 0 {
        return Err("sampled work duration must be positive".into());
    }
    Ok((ticks as u128, seed))
}

fn snapshot(row: &Value) -> Result<crate::shadow::LedgerSnapshot, String> {
    let event = ObservedEvent {
        order: TraceOrderKeyV1 {
            relative_ticks: number(row, "start_tick")?.into(),
            case_key: string(row, "case_id")?.into(),
            occurrence: 0,
            event_kind_rank: EventKindRank::from_canonical_decimal("1").map_err(error)?,
            source_event_key: "anchor".into(),
            source_order: 0,
        },
        available_at: Some(number(row, "start_tick")?.into()),
        source_defined: true,
        transition: Transition::None,
        payload: vec![],
    };
    let mut ledger = ObservedLedger::new(
        vec![event],
        BTreeMap::from([(
            "resource".into(),
            InitialResource {
                capacity: 1,
                claims: vec![],
            },
        )]),
        vec!["Synthetic independent empty-resource anchor".into()],
        ResourcePolicy::Strict,
        LedgerLimits {
            max_events: 4,
            max_payload_bytes: 4096,
            max_assumptions: 4,
            max_identifier_bytes: 8192,
            max_initial_resources: 4,
            max_initial_claims: 4,
            max_assumption_bytes: 4096,
        },
    )
    .map_err(error)?;
    ledger
        .advance()
        .map_err(error)?
        .cloned()
        .ok_or("missing anchor".into())
}

fn shadow_case(config: &Value, task: &Value, row: &Value) -> Result<Value, String> {
    let (work, seed) = sampled_work(config, task, row)?;
    let route = number(&task["params"], "route_ticks")?;
    let at = u128::from(number(row, "start_tick")?);
    let horizon = at
        .checked_add(u128::from(number(task, "horizon_ticks")?))
        .ok_or("horizon overflow")?;
    let model = SyntheticFlowProbeModel {
        config: SyntheticFlowConfig {
            seed_schedule: string(task, "seed_schedule_id")?.into(),
            replication: number(task, "replication_id")?,
            case_key: string(row, "case_id")?.into(),
            work_duration_ticks: work,
            route_speed_mm_per_second: 1,
            route_length_mm: route,
            max_horizon_ticks: horizon,
            ..Default::default()
        },
    };
    let input = model.probe_input().map_err(error)?;
    let adapter = NativeFlowAdapter {
        model,
        max_image_bytes: 4 * 1024 * 1024,
    };
    let snap = snapshot(row)?;
    let mut world = adapter.start(&snap, &input).map_err(error)?;
    let mut predicted = None;
    let mut count = 0;
    let cap = number(task, "max_events")?;
    while count < cap {
        let Some(next) = adapter.next_tick(&world).map_err(error)? else {
            break;
        };
        if next > horizon {
            break;
        }
        let step = adapter.step(&mut world).map_err(error)?;
        count += step.dispatches;
        if let Some(failure) = step.failure {
            return Err(error(failure));
        }
        if let Some(target) = step.target {
            predicted = Some(target);
            break;
        }
    }
    Ok(
        json!({"case_id":row["case_id"],"predicted_tick":predicted.map(|n|n.to_string()),"observed_tick":row["end_tick"],"observed_censored":row["censored"],"predicted_transit_ticks":adapter.model.arrival_tick(&world).and_then(|n|n.checked_sub(at)).map(|n|n.to_string()),"study_service_seed":seed,"native_events":count,"snapshot_digest":snap.digest.iter().map(|b|format!("{b:02x}")).collect::<String>(),"outcome":if predicted.is_some(){"complete"}else{"censored"},"sampled_work_ticks":work.to_string(),"runtime_seed_contract":"sealed-c3-fixed-duration-v1; study randomness sampled before native admission"}),
    )
}

pub(crate) fn evaluate(config: &Value, task: &Value) -> Result<Value, String> {
    admitted(config, task)?;
    evaluate_admitted(config, task)
}
fn evaluate_admitted(config: &Value, task: &Value) -> Result<Value, String> {
    let rows = task["records"].as_array().ok_or("task records")?;
    let invalid = task["valid_candidate"] == false
        || number(&task["params"], "route_ticks")? == 0
        || number(&task["params"], "work_ticks")? == 0;
    let mut records = Vec::new();
    let mut completion = 0f64;
    let mut transit = 0f64;
    let mut eligible = 0u64;
    let mut transit_count = 0u64;
    let mut censored = 0u64;
    let mut missing = 0u64;
    let mut failed = 0u64;
    let free = if task["split"] == "final_test" && !invalid {
        Some(free_run(config, task)?)
    } else {
        None
    };
    for row in rows {
        if invalid {
            failed += 1;
            records.push(json!({"case_id":row["case_id"],"outcome":"invalid","reason":"zero parameter duration"}));
            continue;
        }
        let native = if let Some(free) = &free {
            free["records"]
                .as_array()
                .and_then(|r| r.iter().find(|r| r["case_id"] == row["case_id"]))
                .cloned()
                .ok_or_else(|| "free record missing".to_owned())
        } else {
            shadow_case(config, task, row)
        };
        match native {
            Err(reason) => {
                failed += 1;
                records.push(json!({"case_id":row["case_id"],"outcome":"failed","reason":reason}));
            }
            Ok(record) => {
                let pred = record["predicted_tick"]
                    .as_str()
                    .and_then(|n| n.parse::<u128>().ok());
                let transit_pair = row["transit_ticks"].as_u64().zip(
                    record["predicted_transit_ticks"]
                        .as_str()
                        .and_then(|s| s.parse::<u128>().ok()),
                );
                let need_transit = config["objective"]["transit_weight"]
                    .as_f64()
                    .unwrap_or(0.0)
                    > 0.0;
                if row["censored"] == true || pred.is_none() {
                    censored += 1;
                } else if need_transit && transit_pair.is_none() {
                    missing += 1;
                } else if let Some(observed) = row["end_tick"].as_u64() {
                    eligible += 1;
                    completion += pred.unwrap().abs_diff(observed.into()) as f64;
                    if let Some((observed, predicted)) = transit_pair {
                        transit_count += 1;
                        transit += predicted.abs_diff(observed.into()) as f64;
                    }
                } else {
                    missing += 1;
                }
                records.push(record);
            }
        }
    }
    let outcome = if invalid {
        "invalid"
    } else if failed > 0 {
        "failed"
    } else if censored > 0 {
        "censored"
    } else if missing > 0 || eligible == 0 {
        "missing"
    } else {
        "complete"
    };
    Ok(
        json!({"schema_version":"kairoecs.calibration.result.v1","task_id":task["task_id"],"study_hash":task["study_hash"],"candidate_id":task["candidate_id"],"replication_id":task["replication_id"],"split":task["split"],"seed_schedule_id":task["seed_schedule_id"],"params":task["params"],"outcome":outcome,"replay_role":if free.is_some(){"free"}else{"shadow"},"seed_map_version":1,"root_seed":config["root_seed"],"completion_error":if eligible>0 {Some(completion/eligible as f64)} else {None},"transit_error":if transit_count>0 {Some(transit/transit_count as f64)}else{None},"eligible_count":eligible,"censored_count":censored,"missing_count":missing,"failed_count":failed,"records":records}),
    )
}

// A distinct free-running shared FCFS resource. Arrivals/transit and work are
// inputs; observed completions are consulted only after simulation for reporting.
fn free_run(config: &Value, task: &Value) -> Result<Value, String> {
    let input = task["records"].as_array().ok_or("records")?;
    if input.len() > 4096 {
        return Err("free runtime row inventory cap".into());
    }
    let mut rows = input.clone();
    rows.sort_by_key(|r| {
        (
            r["start_tick"].as_u64().unwrap_or(0),
            r["case_id"].as_str().unwrap_or("").to_owned(),
        )
    });
    let route = number(&task["params"], "route_ticks")?;
    if route == 0 || number(&task["params"], "work_ticks")? == 0 {
        return Err("invalid free candidate".into());
    }
    let mut flow = FlowRuntime::new();
    flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
        .map_err(error)?;
    let resource = flow.create_resource(1).map_err(error)?;
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<u32>(
            "c5.free",
            1,
            |v, cap| {
                if cap < 4 {
                    return Err(kairo_ecs_des::FlowCheckpointCodecError("cap".into()));
                }
                Ok(v.to_le_bytes().to_vec())
            },
            |b, _| {
                Ok(u32::from_le_bytes(b.try_into().map_err(|_| {
                    kairo_ecs_des::FlowCheckpointCodecError("u32".into())
                })?))
            },
        )
        .map_err(error)?;
    codecs
        .register_work_handlers::<u32>(
            "c5.free",
            kairo_ecs_des::FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .map_err(error)?;
    let mut pending = Vec::new();
    let mut horizon = 0;
    for row in rows {
        let (duration, seed) = sampled_work(config, task, &row)?;
        let start = u128::from(number(&row, "start_tick")?);
        horizon = horizon.max(start + u128::from(number(task, "horizon_ticks")?));
        let actor = flow.spawn_actor().map_err(error)?;
        let work = flow
            .create_work(actor, SimDuration::from_ticks(duration), "c5.free", 0u32)
            .map_err(error)?;
        flow.acquire(resource)
            .owner(actor)
            .timed_work(work)
            .at(SimTime::from_ticks(start + u128::from(route)))
            .submit()
            .map_err(error)?;
        pending.push((row, work, seed));
    }
    let mut events = 0;
    while events < number(task, "max_events")? {
        let image = flow
            .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .map_err(error)?;
        let next = image
            .scheduler
            .entries
            .iter()
            .filter(|e| e.live)
            .map(|e| e.request.at.ticks())
            .min();
        if next.is_none_or(|t| t > horizon) {
            break;
        }
        let Some(dispatch) = flow.step().map_err(error)? else {
            break;
        };
        events += 1;
        if let Some(e) = dispatch.error {
            return Err(error(e));
        }
    }
    let mut result = Vec::new();
    for (row, work, seed) in pending {
        let progress = flow.work_progress(work).map_err(error)?;
        let predicted = if progress.state == WorkState::Completed {
            progress.completion_at.map(|t| t.ticks().to_string())
        } else {
            None
        };
        result.push(json!({"case_id":row["case_id"],"predicted_tick":predicted,"observed_tick":row["end_tick"],"observed_censored":row["censored"],"study_service_seed":seed,"native_events":events,"event_count_scope":"shared-task","predicted_transit_ticks":route.to_string(),"outcome":if predicted.is_some(){"complete"}else{"censored"}}));
    }
    Ok(
        json!({"mode":"free","resource_capacity":1,"records":result,"native_events":events,"prediction_uses_observed_completion":false}),
    )
}

pub(crate) fn replay(config: &Value, mode: &str) -> Result<Value, String> {
    if mode != "shadow" && mode != "free" {
        return Err("unsupported replay mode".into());
    }
    let p = crate::c5_study::plan(config)?;
    let task = p["tasks"]
        .as_array()
        .ok_or("tasks")?
        .iter()
        .find(|t| t["split"] == "validation" && t["valid_candidate"] != false)
        .ok_or("no valid replay task")?;
    let result = if mode == "shadow" {
        evaluate_admitted(config, task)?
    } else {
        free_run(config, task)?
    };
    Ok(
        json!({"schema_version":"kairoecs.calibration.replay.v1","study_hash":p["study_hash"],"mode":mode,"candidate_id":task["candidate_id"],"task_id":task["task_id"],"adapter":"synthetic_flow_probe.v1","result_hash":digest(&result),"result":result}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn study() -> Value {
        serde_json::from_str(include_str!("../../../examples/calibration/c5/study.json")).unwrap()
    }
    fn task(c: &Value) -> Value {
        crate::c5_study::plan(c).unwrap()["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| {
                t["split"] == "validation"
                    && t["params"]["route_ticks"] == 2
                    && t["params"]["work_ticks"] == 3
            })
            .unwrap()
            .clone()
    }
    #[test]
    fn c5_native_actual_flow_recovers_known_route_and_work() {
        let c = study();
        let t = task(&c);
        let result = evaluate(&c, &t).unwrap();
        assert_eq!(result["outcome"], "complete", "{result}");
        assert_eq!(result["completion_error"], 0.0);
        assert_eq!(result["transit_error"], 0.0);
        assert_eq!(result["records"][0]["predicted_tick"], "105");
        assert!(result["records"][0]["native_events"].as_u64().unwrap() > 0);
    }
    #[test]
    fn c5_native_task_forgery_fails_before_execution() {
        let c = study();
        let mut t = task(&c);
        t["params"]["route_ticks"] = json!(99);
        assert!(evaluate(&c, &t).unwrap_err().contains("immutable"));
    }
    #[test]
    fn c5_native_horizon_and_event_cap_retain_failed_outcomes() {
        for (name, value) in [("horizon_ticks", 1), ("max_events", 1)] {
            let mut c = study();
            c["budget"][name] = json!(value);
            let t = task(&c);
            let result = evaluate(&c, &t).unwrap();
            assert_eq!(result["outcome"], "censored");
            assert_eq!(result["eligible_count"], 0);
            assert_eq!(result["censored_count"], 1);
        }
    }
    #[test]
    fn c5_native_observed_completion_never_drives_prediction() {
        let c = study();
        let a = evaluate(&c, &task(&c)).unwrap();
        let mut changed = c.clone();
        changed["trace"][1]["end_tick"] = json!(109);
        let b = evaluate(&changed, &task(&changed)).unwrap();
        assert_eq!(
            a["records"][0]["predicted_tick"],
            b["records"][0]["predicted_tick"]
        );
        assert_eq!(
            a["records"][0]["snapshot_digest"],
            b["records"][0]["snapshot_digest"]
        );
        assert_eq!(b["completion_error"], 4.0);
    }
    #[test]
    fn c5_native_crn_draws_are_paired_and_repeatable() {
        let mut c = study();
        c["service_noise_ticks"] = json!([-1, 0, 1]);
        let p = crate::c5_study::plan(&c).unwrap();
        let tasks = p["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["split"] == "validation" && t["replication_id"] == 0)
            .collect::<Vec<_>>();
        let results = tasks
            .iter()
            .map(|t| evaluate(&c, t).unwrap())
            .collect::<Vec<_>>();
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result["outcome"], "complete", "{result}");
            assert_eq!(
                result["records"][0]["study_service_seed"],
                results[0]["records"][0]["study_service_seed"]
            );
            assert_eq!(result, &evaluate(&c, tasks[i]).unwrap());
        }
    }
    #[test]
    fn c5_native_free_queue_is_coupled_and_observation_independent() {
        let mut c = study();
        c["parameters"]["route_ticks"]["values"] = json!([2]);
        c["parameters"]["work_ticks"]["values"] = json!([3]);
        c["trace"].as_array_mut().unwrap().push(json!({"case_id":"validation-2","day":"validation-1","start_tick":100,"end_tick":105,"transit_ticks":2}));
        let s = replay(&c, "shadow").unwrap();
        let f = replay(&c, "free").unwrap();
        assert_eq!(s["result"]["records"][0]["predicted_tick"], "105");
        assert_eq!(s["result"]["records"][1]["predicted_tick"], "105");
        assert_eq!(f["result"]["records"][0]["predicted_tick"], "105");
        assert_eq!(f["result"]["records"][1]["predicted_tick"], "108");
        c["trace"][1]["end_tick"] = json!(190);
        let changed = replay(&c, "free").unwrap();
        assert_eq!(changed["result"]["records"][1]["predicted_tick"], "108");
    }
    #[test]
    fn c5_native_complete_study_and_final_holdout_are_separate() {
        let c = study();
        let p = crate::c5_study::plan(&c).unwrap();
        let mut attempts = p["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| evaluate(&c, t).unwrap())
            .collect::<Vec<_>>();
        let before = crate::c5_study::summarize(&c, &attempts).unwrap();
        assert_eq!(before["complete"], false);
        assert_eq!(before["winner"], "c000004", "{before}");
        for t in before["next_tasks"].as_array().unwrap() {
            attempts.push(evaluate(&c, t).unwrap());
        }
        let after = crate::c5_study::summarize(&c, &attempts).unwrap();
        assert_eq!(after["complete"], true);
        assert_eq!(before["ranking"], after["ranking"]);
        attempts.reverse();
        assert_eq!(crate::c5_study::summarize(&c, &attempts).unwrap(), after);
    }
}
