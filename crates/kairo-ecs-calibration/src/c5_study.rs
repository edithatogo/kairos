//! Private finite C5 study planning and reduction contract (ADR-0022).
//! This module is synthetic development infrastructure, not a stable API.

use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const STUDY_SCHEMA: &str = "kairoecs.calibration.study.v1";
const PLAN_SCHEMA: &str = "kairoecs.calibration.plan.v1";
const ATTEMPT_SCHEMA: &str = "kairoecs.calibration.result.v1";
const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;
const MAX_ID_BYTES: usize = 1024;
const MAX_EXACT_TICK: u64 = (1u64 << 53) - 1;
const SCHEDULES: [&str; 4] = [
    "c5-training-v1",
    "c5-validation-v1",
    "c5-confirmation-v1",
    "c5-final-test-v1",
];

fn err(message: impl Into<String>) -> String {
    message.into()
}
fn object<'a>(v: &'a Value, name: &str) -> Result<&'a Map<String, Value>, String> {
    v.as_object()
        .ok_or_else(|| err(format!("{name} must be an object")))
}
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    v.get(key)
        .ok_or_else(|| err(format!("missing field {key}")))
}
fn string<'a>(v: &'a Value, name: &str) -> Result<&'a str, String> {
    let s = v
        .as_str()
        .ok_or_else(|| err(format!("{name} must be a string")))?;
    if s.is_empty() || s.len() > MAX_ID_BYTES || s.trim() != s || s.chars().any(char::is_control) {
        return Err(err(format!(
            "{name} must be a bounded nonempty canonical string"
        )));
    }
    Ok(s)
}
fn u64v(v: &Value, name: &str) -> Result<u64, String> {
    v.as_u64()
        .ok_or_else(|| err(format!("{name} must be a nonnegative integer")))
}
fn pos(v: &Value, name: &str) -> Result<u64, String> {
    let n = u64v(v, name)?;
    if n == 0 {
        return Err(err(format!("{name} must be positive")));
    }
    Ok(n)
}
fn hash(v: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(err("canonical JSON exceeds byte limit"));
    }
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}
fn sorted_values(v: &Value, name: &str, signed: bool) -> Result<Vec<Value>, String> {
    let values = v
        .as_array()
        .ok_or_else(|| err(format!("{name} must be an array")))?;
    if values.is_empty() || values.len() > 100_000 {
        return Err(err(format!("{name} size out of bounds")));
    }
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        if signed {
            if value.as_i64().is_none() {
                return Err(err(format!("{name} values must be signed integers")));
            }
        } else if value.as_u64().is_none() {
            return Err(err(format!("{name} values must be nonnegative integers")));
        }
        out.push(value.clone());
    }
    out.sort_by(|a, b| {
        if signed {
            a.as_i64().cmp(&b.as_i64())
        } else {
            a.as_u64().cmp(&b.as_u64())
        }
    });
    out.dedup();
    Ok(out)
}
fn canonical_config(config: &Value) -> Result<Validated, String> {
    let root = object(config, "study")?;
    if !(12..=13).contains(&root.len())
        || root.keys().any(|k| {
            ![
                "schema_version",
                "study_id",
                "dataset_id",
                "mapping_version",
                "adapter",
                "root_seed",
                "parameters",
                "objective",
                "budget",
                "precision",
                "splits",
                "trace",
                "service_noise_ticks",
            ]
            .contains(&k.as_str())
        })
    {
        return Err(err("study has missing or unknown top-level fields"));
    }
    if string(field(config, "schema_version")?, "schema_version")? != STUDY_SCHEMA {
        return Err(err("unsupported study schema"));
    }
    let study_id = string(field(config, "study_id")?, "study_id")?.to_owned();
    string(field(config, "dataset_id")?, "dataset_id")?;
    string(field(config, "mapping_version")?, "mapping_version")?;
    if string(field(config, "adapter")?, "adapter")? != "synthetic_flow_probe.v1" {
        return Err(err("unsupported adapter"));
    }
    let root_seed = u64v(field(config, "root_seed")?, "root_seed")?;

    let parameters = field(config, "parameters")?;
    if object(parameters, "parameters")?.len() != 2 {
        return Err(err(
            "parameters must contain route_ticks and work_ticks only",
        ));
    }
    let budget = field(config, "budget")?;
    if object(budget, "budget")?.len() != 7 {
        return Err(err("budget has missing or unknown fields"));
    }
    let max_candidates =
        bounded_count(field(budget, "max_candidates")?, "max_candidates", 100_000)?;
    let route = parameter_values(parameters, "route_ticks")?;
    let work = parameter_values(parameters, "work_ticks")?;
    let product = route
        .len()
        .checked_mul(work.len())
        .ok_or_else(|| err("candidate count overflow"))?;
    if product == 0 || product > max_candidates {
        return Err(err("candidate cap exceeded"));
    }
    let mut candidates = Vec::new();
    for r in &route {
        for w in &work {
            candidates.push(json!({"route_ticks":r,"work_ticks":w}));
        }
    }

    let objective = field(config, "objective")?;
    if object(objective, "objective")?.len() != 5 {
        return Err(err("objective has missing or unknown fields"));
    }
    let completion_weight =
        finite_nonnegative(field(objective, "completion_weight")?, "completion_weight")?;
    let transit_weight = finite_nonnegative(field(objective, "transit_weight")?, "transit_weight")?;
    if completion_weight == 0.0 && transit_weight == 0.0 {
        return Err(err("at least one objective weight must be positive"));
    }
    let scale_ticks = pos(field(objective, "scale_ticks")?, "scale_ticks")?;
    if scale_ticks > MAX_EXACT_TICK {
        return Err(err("scale_ticks exceeds exact numeric metric range"));
    }
    let failure_penalty = finite_positive(field(objective, "failure_penalty")?, "failure_penalty")?;
    let tie_tolerance = finite_nonnegative(field(objective, "tie_tolerance")?, "tie_tolerance")?;

    let replications = bounded_count(field(budget, "replications")?, "replications", 10_000)?;
    let confirmation_replications = bounded_count(
        field(budget, "confirmation_replications")?,
        "confirmation_replications",
        10_000,
    )?;
    let final_test_replications = bounded_count(
        field(budget, "final_test_replications")?,
        "final_test_replications",
        10_000,
    )?;
    let max_tasks = bounded_count(field(budget, "max_tasks")?, "max_tasks", 10_000)?;
    let horizon_ticks = pos(field(budget, "horizon_ticks")?, "horizon_ticks")?;
    let max_events = bounded_count(field(budget, "max_events")?, "max_events", 1_000_000)? as u64;
    if horizon_ticks > MAX_EXACT_TICK {
        return Err(err("horizon_ticks exceeds exact numeric metric range"));
    }

    let precision = field(config, "precision")?;
    if object(precision, "precision")?.len() != 3
        || string(field(precision, "kind")?, "precision.kind")? != "fixed_confirmation"
    {
        return Err(err("precision must be fixed_confirmation v1"));
    }
    let target_half_width =
        finite_positive(field(precision, "target_half_width")?, "target_half_width")?;
    let alpha = finite_positive(field(precision, "alpha")?, "alpha")?;
    if alpha >= 1.0 {
        return Err(err("precision.alpha must be below one"));
    }

    let splits = field(config, "splits")?;
    if object(splits, "splits")?.len() != 3 {
        return Err(err("splits must contain training, validation, final_test"));
    }
    let training = interval(field(splits, "training")?, "training")?;
    let validation = interval(field(splits, "validation")?, "validation")?;
    let final_test = interval(field(splits, "final_test")?, "final_test")?;
    if [
        training.0,
        training.1,
        validation.0,
        validation.1,
        final_test.0,
        final_test.1,
    ]
    .iter()
    .any(|tick| *tick > MAX_EXACT_TICK)
    {
        return Err(err("split bound exceeds exact numeric metric range"));
    }
    if training.0 >= training.1
        || validation.0 >= validation.1
        || final_test.0 >= final_test.1
        || training.1 > validation.0
        || validation.1 > final_test.0
    {
        return Err(err(
            "split intervals must be ordered, nonempty, and nonoverlapping",
        ));
    }

    let trace = field(config, "trace")?
        .as_array()
        .ok_or_else(|| err("trace must be an array"))?;
    if trace.is_empty() || trace.len() > 3 * 4096 {
        return Err(err("trace exceeds total 12288 case cap"));
    }
    let mut cases = BTreeSet::new();
    let mut split_rows: [Vec<Value>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for row in trace {
        if object(row, "trace row")?.len() != 5 {
            return Err(err("trace row has missing or unknown fields"));
        }
        let case = string(field(row, "case_id")?, "case_id")?.to_owned();
        let day = string(field(row, "day")?, "day")?.to_owned();
        if !cases.insert(case.clone()) {
            return Err(err("case_id must occur once"));
        }
        let start = u64v(field(row, "start_tick")?, "start_tick")?;
        let end = if field(row, "end_tick")?.is_null() {
            None
        } else {
            Some(u64v(field(row, "end_tick")?, "end_tick")?)
        };
        if end.is_some_and(|e| e < start) {
            return Err(err("negative trace duration"));
        }
        let transit = if field(row, "transit_ticks")?.is_null() {
            None
        } else {
            Some(u64v(field(row, "transit_ticks")?, "transit_ticks")?)
        };
        if start > MAX_EXACT_TICK
            || end.is_some_and(|e| e > MAX_EXACT_TICK)
            || transit.is_some_and(|t| t > MAX_EXACT_TICK)
        {
            return Err(err("trace tick exceeds exact numeric metric range"));
        }
        let split_idx = if start >= training.0 && start < training.1 {
            0
        } else if start >= validation.0 && start < validation.1 {
            1
        } else if start >= final_test.0 && start < final_test.1 {
            2
        } else {
            return Err(err(format!(
                "trace case {case} starts outside declared splits"
            )));
        };
        if start
            .checked_add(horizon_ticks)
            .is_none_or(|h| h > MAX_EXACT_TICK)
        {
            return Err(err(
                "start_tick plus horizon_ticks exceeds exact numeric metric range",
            ));
        }
        let bounds = [training, validation, final_test][split_idx];
        let censored = end.is_none_or(|e| e >= bounds.1);
        if !censored && transit.is_some_and(|t| end.is_some_and(|e| t > e.saturating_sub(start))) {
            return Err(err("transit_ticks exceeds observed duration"));
        }
        let clipped_end = if censored { None } else { end };
        let exposure_end = clipped_end.unwrap_or(bounds.1);
        let exposure = exposure_end
            .checked_sub(start)
            .ok_or_else(|| err("trace exposure underflow"))?;
        if exposure == 0 && (censored || clipped_end.is_none()) {
            return Err(err(
                "zero exposure is valid only for an observed instantaneous event",
            ));
        }
        let row = json!({"case_id":case,"day":day,"start_tick":start,"end_tick":clipped_end,"exposure_ticks":exposure,"transit_ticks":if censored { None } else { transit },"censored":censored});
        if split_rows[split_idx].len() >= 4096 {
            return Err(err("trace rows per split exceed 4096 case cap"));
        }
        split_rows[split_idx].push(row);
    }
    if split_rows.iter().any(Vec::is_empty) {
        return Err(err("each declared split must contain trace rows"));
    }
    if split_rows.iter().any(|rows| rows.len() > 4096) {
        return Err(err("trace rows per split exceed 4096 case cap"));
    }
    // Day clusters are a statistical resampling unit and may not leak across splits.
    let mut day_splits: BTreeMap<String, usize> = BTreeMap::new();
    for (idx, rows) in split_rows.iter().enumerate() {
        for row in rows {
            let day = string(field(row, "day")?, "day")?;
            if day_splits
                .insert(day.to_owned(), idx)
                .is_some_and(|prev| prev != idx)
            {
                return Err(err("day cluster crosses split boundary"));
            }
        }
    }
    for rows in &mut split_rows {
        rows.sort_by(|a, b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    }
    let service_noise = if let Some(v) = config.get("service_noise_ticks") {
        let values = v
            .as_array()
            .ok_or_else(|| err("service_noise_ticks must be an array"))?;
        if values.is_empty() || values.len() > 4096 {
            return Err(err("service_noise_ticks size out of bounds"));
        }
        for value in values {
            let n = value
                .as_i64()
                .ok_or_else(|| err("service_noise_ticks values must be signed integers"))?;
            if n.unsigned_abs() > MAX_EXACT_TICK {
                return Err(err(
                    "service_noise_ticks exceeds exact numeric metric range",
                ));
            }
        }
        values.clone()
    } else {
        vec![json!(0)]
    };
    let initial_count = candidates
        .len()
        .checked_mul(
            replications
                .checked_add(replications)
                .and_then(|n| n.checked_add(confirmation_replications))
                .ok_or_else(|| err("task count overflow"))?,
        )
        .ok_or_else(|| err("task count overflow"))?;
    let all_count = initial_count
        .checked_add(final_test_replications)
        .ok_or_else(|| err("task count overflow"))?;
    if all_count > max_tasks {
        return Err(err("task cap exceeded"));
    }
    preflight_plan_bytes(
        &split_rows,
        &service_noise,
        &candidates,
        replications,
        confirmation_replications,
        final_test_replications,
        &study_id,
    )?;
    Ok(Validated {
        study_id,
        root_seed,
        candidates,
        completion_weight,
        transit_weight,
        scale_ticks,
        failure_penalty,
        tie_tolerance,
        replications,
        confirmation_replications,
        final_test_replications,
        horizon_ticks,
        max_events,
        target_half_width,
        alpha,
        split_rows,
        service_noise,
    })
}

fn parameter_values(parameters: &Value, key: &str) -> Result<Vec<u64>, String> {
    let param = field(parameters, key)?;
    if object(param, key)?.len() != 2 || string(field(param, "unit")?, "parameter unit")? != "tick"
    {
        return Err(err(format!("{key} must be an integer tick parameter")));
    }
    let values = sorted_values(field(param, "values")?, key, false)?;
    values
        .iter()
        .map(|v| {
            let n = u64v(v, key)?;
            if n > MAX_EXACT_TICK {
                Err(err(format!("{key} exceeds exact numeric metric range")))
            } else {
                Ok(n)
            }
        })
        .collect()
}
fn interval(v: &Value, name: &str) -> Result<(u64, u64), String> {
    let a = v
        .as_array()
        .ok_or_else(|| err(format!("{name} split must be [start,end]")))?;
    if a.len() != 2 {
        return Err(err(format!("{name} split must have two bounds")));
    }
    Ok((u64v(&a[0], name)?, u64v(&a[1], name)?))
}
fn bounded_count(v: &Value, name: &str, max: usize) -> Result<usize, String> {
    let n = pos(v, name)?;
    let n: usize = n.try_into().map_err(|_| err(format!("{name} overflow")))?;
    if n > max {
        Err(err(format!("{name} exceeds cap")))
    } else {
        Ok(n)
    }
}
fn finite_nonnegative(v: &Value, name: &str) -> Result<f64, String> {
    let n = v
        .as_f64()
        .ok_or_else(|| err(format!("{name} must be numeric")))?;
    if !n.is_finite() || n < 0.0 {
        Err(err(format!("{name} must be finite and nonnegative")))
    } else {
        Ok(n)
    }
}
fn finite_positive(v: &Value, name: &str) -> Result<f64, String> {
    let n = finite_nonnegative(v, name)?;
    if n == 0.0 {
        Err(err(format!("{name} must be positive")))
    } else {
        Ok(n)
    }
}

fn preflight_plan_bytes(
    rows: &[Vec<Value>; 3],
    noise: &[Value],
    candidates: &[Value],
    replications: usize,
    confirmation_replications: usize,
    final_replications: usize,
    study_id: &str,
) -> Result<(), String> {
    let counts = [replications, replications, confirmation_replications];
    // A conservative envelope allowance includes names, maximal integer fields,
    // hashes and candidate metadata. Count escaped identity bytes explicitly.
    let per_task_overhead = 1024usize
        .checked_add(
            serde_json::to_vec(study_id)
                .map_err(|e| e.to_string())?
                .len(),
        )
        .and_then(|n| n.checked_add(serde_json::to_vec(noise).ok()?.len()))
        .ok_or_else(|| err("plan byte size overflow"))?;
    let mut estimated = serde_json::to_vec(candidates)
        .map_err(|e| e.to_string())?
        .len();
    let serialized_rows = [
        serde_json::to_vec(&rows[0])
            .map_err(|e| e.to_string())?
            .len(),
        serde_json::to_vec(&rows[1])
            .map_err(|e| e.to_string())?
            .len(),
        serde_json::to_vec(&rows[2])
            .map_err(|e| e.to_string())?
            .len(),
    ];
    for _ in candidates {
        for (split, n) in counts.iter().enumerate() {
            let row_slot = if split == 2 { 1 } else { split };
            let base = per_task_overhead
                .checked_mul(*n)
                .ok_or_else(|| err("plan byte size overflow"))?;
            let payload = serialized_rows[row_slot]
                .checked_mul(*n)
                .ok_or_else(|| err("plan byte size overflow"))?;
            estimated = estimated
                .checked_add(base)
                .and_then(|x| x.checked_add(payload))
                .ok_or_else(|| err("plan byte size overflow"))?;
        }
    }
    let test_bytes = per_task_overhead
        .checked_mul(final_replications)
        .and_then(|x| x.checked_add(serialized_rows[2].checked_mul(final_replications)?))
        .ok_or_else(|| err("plan byte size overflow"))?;
    estimated = estimated
        .checked_add(test_bytes)
        .ok_or_else(|| err("plan byte size overflow"))?;
    if estimated > MAX_JSON_BYTES {
        return Err(err("estimated plan output exceeds 16 MiB"));
    }
    Ok(())
}

#[derive(Clone)]
struct Validated {
    study_id: String,
    root_seed: u64,
    candidates: Vec<Value>,
    completion_weight: f64,
    transit_weight: f64,
    scale_ticks: u64,
    failure_penalty: f64,
    tie_tolerance: f64,
    replications: usize,
    confirmation_replications: usize,
    final_test_replications: usize,
    horizon_ticks: u64,
    max_events: u64,
    target_half_width: f64,
    alpha: f64,
    split_rows: [Vec<Value>; 3],
    service_noise: Vec<Value>,
}
fn split_name(i: usize) -> &'static str {
    ["training", "validation", "confirmation", "final_test"][i]
}
fn schedule(i: usize) -> &'static str {
    SCHEDULES[i]
}
fn params_valid(params: &Value) -> bool {
    params
        .get("route_ticks")
        .and_then(Value::as_u64)
        .is_some_and(|x| x > 0)
        && params
            .get("work_ticks")
            .and_then(Value::as_u64)
            .is_some_and(|x| x > 0)
}
fn candidate_id(i: usize) -> String {
    format!("c{i:06}")
}
fn task(
    config: &Validated,
    study_hash: &str,
    candidate: &Value,
    cid: &str,
    rep: usize,
    split: usize,
) -> Value {
    let replication_id = json!(rep as u64);
    let records = &config.split_rows[if split == 2 {
        1
    } else if split == 3 {
        2
    } else {
        split
    }];
    let material = json!({"study_hash":study_hash,"candidate_id":cid,"replication_id":replication_id,"split":split_name(split),"seed_schedule_id":schedule(split),"params":candidate});
    let task_id = hash(&material).expect("bounded canonical task");
    json!({"task_id":task_id,"study_hash":study_hash,"candidate_id":cid,"replication_id":replication_id,"split":split_name(split),"seed_schedule_id":schedule(split),"params":candidate,"records":records,"service_noise_ticks":config.service_noise,"horizon_ticks":config.horizon_ticks,"max_events":config.max_events,"root_seed":config.root_seed,"study_id":config.study_id,"valid_candidate":params_valid(candidate)})
}
fn build_initial(config: &Validated, study_hash: &str) -> Result<Vec<Value>, String> {
    let mut tasks = Vec::new();
    for (ci, candidate) in config.candidates.iter().enumerate() {
        let cid = candidate_id(ci);
        for (split, n) in [
            (0, config.replications),
            (1, config.replications),
            (2, config.confirmation_replications),
        ] {
            for rep in 0..n {
                tasks.push(task(config, study_hash, candidate, &cid, rep, split));
            }
        }
    }
    tasks.sort_by(|a, b| a["task_id"].as_str().cmp(&b["task_id"].as_str()));
    let mut seed_map = CalibrationSeedMap::new(1, &config.study_id, config.root_seed)
        .map_err(|e| e.to_string())?;
    for t in &tasks {
        let rep = u64v(&t["replication_id"], "replication_id")?;
        for row in t["records"].as_array().unwrap() {
            let case = row["case_id"].as_str().unwrap();
            seed_map
                .key_for(
                    t["seed_schedule_id"].as_str().unwrap(),
                    rep,
                    case,
                    "service",
                    SeedPurpose::Service,
                )
                .map_err(|e| e.to_string())?;
        }
    }
    for rep in 0..config.final_test_replications {
        for row in &config.split_rows[2] {
            seed_map
                .key_for(
                    SCHEDULES[3],
                    rep as u64,
                    row["case_id"].as_str().unwrap(),
                    "service",
                    SeedPurpose::Service,
                )
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(tasks)
}

pub(crate) fn plan(config: &Value) -> Result<Value, String> {
    let validated = canonical_config(config)?;
    let study_hash = hash(config)?;
    let tasks = build_initial(&validated, &study_hash)?;
    let candidates: Vec<Value> = validated
        .candidates
        .iter()
        .enumerate()
        .map(|(i, p)| json!({"candidate_id":candidate_id(i),"params":p,"valid":params_valid(p)}))
        .collect();
    Ok(
        json!({"schema_version":PLAN_SCHEMA,"study_hash":study_hash,"candidates":candidates,"tasks":tasks,"diagnostics":{"training":training_diagnostics(&validated)}}),
    )
}

/// Recreate the only permitted post-selection workload: fixed final-test samples
/// for the validation-frozen winner. Root/native code consumes this exact task list.
pub(crate) fn final_tasks(config: &Value, winner: &str) -> Result<Vec<Value>, String> {
    let validated = canonical_config(config)?;
    let study_hash = hash(config)?;
    let index = winner
        .strip_prefix('c')
        .ok_or_else(|| err("invalid winner id"))?
        .parse::<usize>()
        .map_err(|_| err("invalid winner id"))?;
    if candidate_id(index) != winner || index >= validated.candidates.len() {
        return Err(err("winner id outside candidate inventory"));
    }
    let params = &validated.candidates[index];
    let mut tasks = Vec::new();
    for rep in 0..validated.final_test_replications {
        tasks.push(task(&validated, &study_hash, params, winner, rep, 3));
    }
    tasks.sort_by(|a, b| a["task_id"].as_str().cmp(&b["task_id"].as_str()));
    Ok(tasks)
}

fn training_diagnostics(c: &Validated) -> Value {
    let rows = &c.split_rows[0];
    let mut exposure = 0u128;
    let mut events = 0u64;
    let mut censored = 0u64;
    let mut day_values: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for r in rows {
        let e = r["exposure_ticks"].as_u64().unwrap_or(0);
        exposure = exposure.saturating_add(u128::from(e));
        let is_censored = r["censored"].as_bool().unwrap_or(true);
        censored += u64::from(is_censored);
        events += u64::from(!is_censored);
        day_values
            .entry(r["day"].as_str().unwrap_or(""))
            .or_default()
            .push(e as f64);
    }
    let rate = if exposure == 0 {
        Value::Null
    } else {
        json!(events as f64 / exposure as f64)
    };
    // Descriptive two-stage cluster bootstrap variance oracle: variance over equally
    // likely day means, retaining within-day observations as a cluster.
    let means: Vec<f64> = day_values
        .values()
        .map(|x| x.iter().sum::<f64>() / x.len() as f64)
        .collect();
    let cluster_variance = bootstrap_mean_variance(&means);
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r["exposure_ticks"].as_u64().map(|x| x as f64))
        .collect();
    let row_variance = bootstrap_mean_variance(&values);
    let exponential_mean = if events == 0 || exposure == 0 {
        Value::Null
    } else {
        json!(exposure as f64 / events as f64)
    };
    let censoring_rate = if rows.is_empty() {
        Value::Null
    } else {
        json!(censored as f64 / rows.len() as f64)
    };
    json!({"estimator":"training_only_constant_hazard_exposure_over_events","event_count":events,"censored_count":censored,"censoring_rate":censoring_rate,"exposure_ticks":exposure.to_string(),"event_rate_per_tick":rate,"exponential_mean_ticks":exponential_mean,"cluster_count":day_values.len(),"day_cluster_means":means,"day_cluster_bootstrap_estimand":"equal_day_mean","day_cluster_bootstrap_variance":cluster_variance,"row_iid_bootstrap_estimand":"row_weighted_mean","row_iid_bootstrap_variance":row_variance})
}

/// Variance of the mean under an ordinary fixed-size IID bootstrap. The
/// population variance of the empirical support is divided by draw count.
fn bootstrap_mean_variance(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    Some(
        values.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>()
            / (values.len() * values.len()) as f64,
    )
}

pub(crate) fn summarize(config: &Value, attempts: &[Value]) -> Result<Value, String> {
    let c = canonical_config(config)?;
    let study_hash = hash(config)?;
    let initial = build_initial(&c, &study_hash)?;
    let expected: BTreeMap<String, &Value> = initial
        .iter()
        .map(|t| (t["task_id"].as_str().unwrap().to_owned(), t))
        .collect();
    let mut supplied = BTreeMap::new();
    for result in attempts {
        let id = string(field(result, "task_id")?, "task_id")?.to_owned();
        if supplied.insert(id.clone(), result).is_some() {
            return Err(err("duplicate task result"));
        }
        if let Some(task) = expected.get(&id) {
            validate_result(task, result, &c)?;
        } else if result.get("split").and_then(Value::as_str) == Some("final_test") { /* checked after winner freeze */
        } else {
            return Err(err("unknown task result"));
        }
    }
    if expected.keys().any(|id| !supplied.contains_key(id)) {
        return Err(err("incomplete initial task inventory"));
    }
    let mut candidate_summaries = Vec::new();
    for (ci, params) in c.candidates.iter().enumerate() {
        let cid = candidate_id(ci);
        let mut failures = 0usize;
        let mut validation_scores = Vec::new();
        let mut confirmation_scores = Vec::new();
        let mut train_scores = Vec::new();
        for task in initial.iter().filter(|t| t["candidate_id"] == cid) {
            let result = supplied[task["task_id"].as_str().unwrap()];
            let split = result["split"].as_str().unwrap();
            let s = task_score(result, &c)?;
            if !params_valid(params) || result["outcome"] != "complete" {
                failures += 1;
            }
            match split {
                "training" => train_scores.push(s),
                "validation" => validation_scores.push(s),
                "confirmation" => confirmation_scores.push(s),
                _ => return Err(err("unexpected initial split")),
            }
        }
        let avg = |v: &[f64]| {
            if v.is_empty() {
                None
            } else {
                Some(v.iter().map(|x| *x / v.len() as f64).sum::<f64>())
            }
        };
        let val = avg(&validation_scores);
        let conf = avg(&confirmation_scores);
        let status = if !params_valid(params) {
            "invalid_candidate"
        } else if val.is_some() && conf.is_some() {
            "complete"
        } else {
            "mc_indeterminate"
        };
        candidate_summaries.push(json!({"candidate_id":cid,"params":params,"training_objective_mean":avg(&train_scores),"validation_objective_mean":val,"confirmation_objective_mean":conf,"confirmation_precision_met":confirmation_half_width(&c)<=c.target_half_width&&conf.is_some(),"attempt_count":initial.iter().filter(|t|t["candidate_id"]==cid).count(),"failure_count":failures,"status":status,"bound_hits":{"route_ticks":params["route_ticks"]==c.candidates.iter().map(|p|p["route_ticks"].as_u64().unwrap()).min().map(Value::from).unwrap_or(Value::Null)||params["route_ticks"]==c.candidates.iter().map(|p|p["route_ticks"].as_u64().unwrap()).max().map(Value::from).unwrap_or(Value::Null),"work_ticks":params["work_ticks"]==c.candidates.iter().map(|p|p["work_ticks"].as_u64().unwrap()).min().map(Value::from).unwrap_or(Value::Null)||params["work_ticks"]==c.candidates.iter().map(|p|p["work_ticks"].as_u64().unwrap()).max().map(Value::from).unwrap_or(Value::Null)}}));
    }
    let mut ranking: Vec<(usize, f64)> = candidate_summaries
        .iter()
        .enumerate()
        .filter(|(_, s)| s["status"] != "invalid_candidate")
        .filter_map(|(i, s)| s["validation_objective_mean"].as_f64().map(|x| (i, x)))
        .collect();
    ranking.sort_by(|a, b| {
        a.1.total_cmp(&b.1)
            .then_with(|| candidate_id(a.0).cmp(&candidate_id(b.0)))
    });
    let winner = ranking.first().map(|x| candidate_id(x.0));
    let ridge = ranking.len() > 1 && ranking[1].1 - ranking[0].1 <= c.tie_tolerance;
    let next: Vec<Value> = if let Some(w) = winner.as_deref() {
        final_tasks(config, w)?
    } else {
        Vec::new()
    };
    let final_expected: BTreeMap<String, &Value> = next
        .iter()
        .map(|t| (t["task_id"].as_str().unwrap().to_owned(), t))
        .collect();
    let final_results: Vec<&Value> = attempts
        .iter()
        .filter(|r| r.get("split").and_then(Value::as_str) == Some("final_test"))
        .collect();
    if !final_results.is_empty() {
        if final_results.len() != final_expected.len() {
            return Err(err("final-test task inventory mismatch"));
        }
        let mut seen_final = BTreeSet::new();
        for r in &final_results {
            let id = r["task_id"]
                .as_str()
                .ok_or_else(|| err("final result task id"))?;
            if !seen_final.insert(id) {
                return Err(err("duplicate final-test task result"));
            }
            let t = final_expected
                .get(id)
                .ok_or_else(|| err("unexpected final-test task"))?;
            validate_result(t, r, &c)?;
        }
        if seen_final.len() != final_expected.len() {
            return Err(err("incomplete final-test task inventory"));
        }
    }
    let initial_only = final_results.is_empty();
    let any_valid_candidate = candidate_summaries
        .iter()
        .any(|s| s["status"] != "invalid_candidate");
    let mc_indeterminate = candidate_summaries
        .iter()
        .any(|s| s["status"] == "mc_indeterminate")
        || (any_valid_candidate && !confirmation_precision(&candidate_summaries, &c));
    let complete = !initial_only || winner.is_none();
    let final_by_id: BTreeMap<&str, &Value> = final_results
        .iter()
        .filter_map(|r| r["task_id"].as_str().map(|id| (id, *r)))
        .collect();
    let mut final_errors = Vec::new();
    let mut final_scores = Vec::new();
    let mut final_failures = 0usize;
    let mut final_censored = 0usize;
    if !initial_only {
        for t in &next {
            let r = final_by_id[t["task_id"].as_str().unwrap()];
            final_scores.push(task_score(r, &c)?);
            if let Some(value) = finite_metric(r, "completion_error")? {
                final_errors.push(value);
            }
            final_failures += usize::from(r["failed_count"].as_u64().unwrap_or(0) > 0);
            final_censored += usize::from(r["censored_count"].as_u64().unwrap_or(0) > 0);
        }
    }
    let final_score = if final_errors.is_empty() {
        None
    } else {
        Some(mean(&final_errors))
    };
    let final_objective_score = if final_scores.is_empty() {
        None
    } else {
        Some(mean(&final_scores))
    };
    let next_tasks = if initial_only && winner.is_some() {
        json!(next)
    } else {
        json!([])
    };
    Ok(
        json!({"schema_version":"kairoecs.calibration.summary.v1","study_hash":study_hash,"complete":complete,"ranking":ranking.iter().map(|(i,s)|json!({"candidate_id":candidate_id(*i),"score":s})).collect::<Vec<_>>(),"candidates":candidate_summaries,"winner":winner,"winner_frozen":winner.is_some(),"ridge":ridge,"identifiability":if ridge{"non_identifiable_ridge"}else{"not_demonstrated"},"mc_indeterminate":mc_indeterminate,"precision":{"kind":"fixed_confirmation","target_half_width":c.target_half_width,"alpha":c.alpha,"confirmation_replications":c.confirmation_replications,"bounded_score_hoeffding_half_width":confirmation_half_width(&c),"validity":"bounded_score_hoeffding_no_optional_stopping"},"final_test":{"winner_only":true,"mean_completion_error":final_score,"mean_objective_score":final_objective_score,"completed_count":final_results.iter().filter(|r|r["outcome"]=="complete").count(),"failed_count":final_failures,"censored_count":final_censored,"attempt_count":final_results.len()},"training_diagnostics":training_diagnostics(&c),"next_tasks":next_tasks}),
    )
}

fn validate_result(task: &Value, result: &Value, c: &Validated) -> Result<(), String> {
    if result.get("schema_version").and_then(Value::as_str) != Some(ATTEMPT_SCHEMA) {
        return Err(err("unsupported result schema"));
    }
    for key in [
        "task_id",
        "study_hash",
        "candidate_id",
        "replication_id",
        "split",
        "seed_schedule_id",
        "params",
    ] {
        if task.get(key) != result.get(key) {
            return Err(err(format!("result identity mismatch: {key}")));
        }
    }
    let outcome = string(field(result, "outcome")?, "outcome")?;
    if !["complete", "invalid", "failed", "censored", "missing"].contains(&outcome) {
        return Err(err("unknown result outcome"));
    }
    for key in ["completion_error", "transit_error"] {
        if !result
            .get(key)
            .is_some_and(|x| x.is_null() || x.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0))
        {
            return Err(err(format!("invalid {key}")));
        }
    }
    for key in [
        "eligible_count",
        "censored_count",
        "missing_count",
        "failed_count",
    ] {
        u64v(field(result, key)?, key)?;
    }
    let counts = [
        "eligible_count",
        "censored_count",
        "missing_count",
        "failed_count",
    ]
    .map(|k| u64v(&result[k], k))
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    let count_sum = counts.iter().try_fold(0u64, |a, n| {
        a.checked_add(*n)
            .ok_or_else(|| err("result count overflow"))
    })?;
    let expected = task["records"]
        .as_array()
        .ok_or_else(|| err("task records missing"))?;
    if count_sum != expected.len() as u64 {
        return Err(err("result counts do not partition task records"));
    }
    if task["valid_candidate"] == false && outcome != "invalid" {
        return Err(err("invalid candidate cannot produce a complete result"));
    }
    let records = result
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| err("result records must be an array"))?;
    let mut actual = BTreeSet::new();
    let mut derived = [0u64; 4]; // eligible, censored, missing, failed
    let mut completion_errors = Vec::new();
    let mut transit_errors = Vec::new();
    for record in records {
        let case = string(field(record, "case_id")?, "result case_id")?;
        if !actual.insert(case.to_owned()) {
            return Err(err("duplicate result case record"));
        }
        let expected_row = expected
            .iter()
            .find(|r| r["case_id"] == case)
            .ok_or_else(|| err("unexpected result case"))?;
        if let Some(observed) = record.get("observed_tick") {
            if observed != &expected_row["end_tick"] {
                return Err(err("result observation differs from clipped trace"));
            }
        }
        let row_outcome = record
            .get("outcome")
            .and_then(Value::as_str)
            .ok_or_else(|| err("result case outcome missing"))?;
        if !["complete", "censored", "missing", "failed", "invalid"].contains(&row_outcome) {
            return Err(err("unknown case outcome"));
        }
        let expected_censored = expected_row["censored"].as_bool() == Some(true);
        let observed_flag = record.get("observed_censored").and_then(Value::as_bool);
        if observed_flag.is_some_and(|flag| flag != expected_censored) {
            return Err(err("observed censor flag differs from trace"));
        }
        if !["failed", "invalid", "missing"].contains(&row_outcome) && observed_flag.is_none() {
            return Err(err("successful case lacks observed_censored flag"));
        }
        if !["failed", "invalid", "missing"].contains(&row_outcome) {
            for key in [
                "predicted_tick",
                "observed_tick",
                "study_service_seed",
                "native_events",
                "predicted_transit_ticks",
            ] {
                if record.get(key).is_none() {
                    return Err(err(format!("successful case missing {key}")));
                }
            }
            for key in ["study_service_seed", "native_events"] {
                u64v(&record[key], key)?;
            }
            for key in ["predicted_tick", "predicted_transit_ticks"] {
                if !record[key].is_null() {
                    let s = string(&record[key], key)?;
                    let n = s
                        .parse::<u64>()
                        .map_err(|_| err(format!("invalid {key}")))?;
                    if n > MAX_EXACT_TICK || n.to_string() != s {
                        return Err(err(format!("invalid canonical {key}")));
                    }
                }
            }
        }
        let observed_transit = expected_row["transit_ticks"].as_u64();
        let predicted_transit = if ["failed", "invalid", "missing"].contains(&row_outcome) {
            None
        } else {
            parse_tick_string(
                &record["predicted_transit_ticks"],
                "predicted_transit_ticks",
            )?
        };
        let bucket = if ["failed", "invalid"].contains(&row_outcome) {
            3
        } else if expected_censored || row_outcome == "censored" {
            1
        } else if row_outcome == "missing" {
            2
        } else {
            let predicted = parse_tick_string(&record["predicted_tick"], "predicted_tick")?;
            let observed = expected_row["end_tick"]
                .as_u64()
                .ok_or_else(|| err("eligible trace row lacks observed completion"))?;
            let Some(predicted) = predicted else {
                return Err(err("completed prediction has null predicted_tick"));
            };
            if c.transit_weight > 0.0 && (observed_transit.is_none() || predicted_transit.is_none())
            {
                2
            } else {
                completion_errors.push(predicted.abs_diff(u128::from(observed)) as f64);
                if let (Some(observed), Some(predicted)) = (observed_transit, predicted_transit) {
                    transit_errors.push(predicted.abs_diff(u128::from(observed)) as f64);
                }
                0
            }
        };
        derived[bucket] += 1;
    }
    let expected_cases: BTreeSet<String> = expected
        .iter()
        .map(|r| r["case_id"].as_str().unwrap().to_owned())
        .collect();
    if actual != expected_cases {
        return Err(err("result case inventory mismatch"));
    }
    if derived != [counts[0], counts[1], counts[2], counts[3]] {
        return Err(err(
            "aggregate counts differ from exclusive case outcome partition",
        ));
    }
    let expected_outcome = if task["valid_candidate"] == false {
        "invalid"
    } else if derived[3] > 0 {
        "failed"
    } else if derived[1] > 0 {
        "censored"
    } else if derived[2] > 0 || derived[0] == 0 {
        "missing"
    } else {
        "complete"
    };
    if outcome != expected_outcome {
        return Err(err("outer outcome differs from case outcome partition"));
    }
    let expected_completion = (!completion_errors.is_empty()).then(|| mean(&completion_errors));
    let expected_transit = (!transit_errors.is_empty()).then(|| mean(&transit_errors));
    assert_metric_matches(result, "completion_error", expected_completion)?;
    assert_metric_matches(result, "transit_error", expected_transit)?;
    Ok(())
}

fn parse_tick_string(v: &Value, name: &str) -> Result<Option<u128>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let s = string(v, name)?;
    let n = s
        .parse::<u128>()
        .map_err(|_| err(format!("invalid {name}")))?;
    if n > u128::from(MAX_EXACT_TICK) || n.to_string() != s {
        return Err(err(format!("invalid canonical {name}")));
    }
    Ok(Some(n))
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().map(|x| *x / xs.len() as f64).sum::<f64>()
}

fn assert_metric_matches(result: &Value, key: &str, expected: Option<f64>) -> Result<(), String> {
    let actual = finite_metric(result, key)?;
    match (actual, expected) {
        (None, None) => Ok(()),
        (Some(a), Some(b)) if (a - b).abs() <= 1e-9 * b.abs().max(1.0) => Ok(()),
        _ => Err(err(format!(
            "{key} differs from successful per-case records"
        ))),
    }
}

fn task_score(result: &Value, c: &Validated) -> Result<f64, String> {
    let records = result["records"]
        .as_array()
        .ok_or_else(|| err("result records"))?;
    let total = records.len();
    if total == 0 {
        return Err(err("task has no records"));
    }
    let eligible = u64v(&result["eligible_count"], "eligible_count")? as usize;
    let completion = finite_metric(result, "completion_error")?;
    let transit = finite_metric(result, "transit_error")?;
    if eligible > 0 && c.completion_weight > 0.0 && completion.is_none() {
        return Err(err("eligible case lacks required completion metric"));
    }
    if eligible > 0 && c.transit_weight > 0.0 && transit.is_none() {
        return Err(err("eligible case lacks required transit metric"));
    }
    let mut observed = 0.0;
    for (weight, error) in [
        (c.completion_weight, completion.unwrap_or(0.0)),
        (c.transit_weight, transit.unwrap_or(0.0)),
    ] {
        let scaled = error / c.scale_ticks as f64;
        if scaled > 0.0 && weight >= (c.failure_penalty - observed) / scaled {
            observed = c.failure_penalty;
            break;
        }
        observed += weight * scaled;
        if !observed.is_finite() || observed >= c.failure_penalty {
            observed = c.failure_penalty;
            break;
        }
    }
    let eligible_fraction = eligible as f64 / total as f64;
    // Algebraically equals the eligible/uncovered weighted mean, but remains finite
    // even when the declared penalty is near f64::MAX.
    Ok(c.failure_penalty - eligible_fraction * (c.failure_penalty - observed))
}
fn finite_metric(result: &Value, key: &str) -> Result<Option<f64>, String> {
    match field(result, key)? {
        Value::Null => Ok(None),
        v => {
            let n = v
                .as_f64()
                .ok_or_else(|| err(format!("{key} must be finite numeric or null")))?;
            if n.is_finite() && n >= 0.0 {
                Ok(Some(n))
            } else {
                Err(err(format!("invalid {key}")))
            }
        }
    }
}
fn confirmation_precision(summaries: &[Value], c: &Validated) -> bool {
    // The independent fixed confirmation sample's bounded Hoeffding width. Scores
    // are capped at failure_penalty; all candidates receive the same fixed budget.
    confirmation_half_width(c) <= c.target_half_width
        && summaries
            .iter()
            .filter(|s| s["status"] != "invalid_candidate")
            .all(|s| s["confirmation_objective_mean"].as_f64().is_some())
}

fn confirmation_half_width(c: &Validated) -> f64 {
    let n = c.confirmation_replications as f64;
    if n == 0.0 {
        f64::INFINITY
    } else {
        let factor = ((2.0_f64.ln() - c.alpha.ln()) / (2.0 * n)).sqrt();
        if factor >= 1.0 {
            c.failure_penalty
        } else {
            c.failure_penalty * factor
        }
    }
}

#[cfg(test)]
#[path = "c5_study_tests.rs"]
mod tests;
