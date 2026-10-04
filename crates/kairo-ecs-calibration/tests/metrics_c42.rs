//! Actual C4.2 metric producer for the frozen C4.1 synthetic fixtures.
//!
//! This adapter deliberately consumes only fixture IDs and `input`. Frozen
//! expected values are read by the independent comparator in `metrics_c41.rs`.

#[path = "../src/metrics.rs"]
mod metrics;

use metrics::{MetricRequest, MetricStatus, Precision};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, process::Command};

const FIXTURE_PATH: &str = "conformance/c41/fixtures.json";
const CANDIDATE_SCHEMA: &str = "c41.candidate.v1";
const PRODUCER_API: &str =
    "tests/metrics_c42.rs::produce_candidate_report -> src/metrics.rs::compare";

fn fixture_bytes() -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(FIXTURE_PATH),
    )
    .expect("frozen C4.1 fixtures are present")
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn object<'a>(value: &'a Value, where_: &str) -> &'a Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{where_} must be an object"))
}

fn string<'a>(value: &'a Value, where_: &str) -> &'a str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{where_} must be a string"))
}

fn input_values<'a>(input: &'a Value, key: &str) -> Vec<Option<&'a str>> {
    input[key]
        .as_array()
        .unwrap_or_else(|| panic!("input.{key} must be an array"))
        .iter()
        .map(|v| v.as_str())
        .collect()
}

fn optional_weights<'a>(input: &'a Value, key: &str) -> Option<Vec<&'a str>> {
    input.get(key).map(|v| {
        v.as_array()
            .unwrap_or_else(|| panic!("input.{key} must be an array"))
            .iter()
            .map(|x| string(x, key))
            .collect()
    })
}

fn status_name(status: &MetricStatus) -> &'static str {
    match status {
        MetricStatus::Computed => "ok",
        MetricStatus::Empty => "empty",
        MetricStatus::InsufficientData => "insufficient_data",
        MetricStatus::Invalid => "invalid",
    }
}

fn precision_name(precision: &Precision) -> &'static str {
    match precision {
        Precision::NotApplicable => "not_applicable",
        Precision::ExactOffsets => "exact_offsets",
        Precision::Rejected => "rejected",
    }
}

fn metric_number(value: Option<f64>) -> Value {
    value
        .filter(|x| x.is_finite())
        .map_or(Value::Null, |x| json!(x))
}

fn string_count(value: usize) -> Value {
    json!(value.to_string())
}

fn json_count(value: &Value, key: &str, default: usize) -> Value {
    value
        .get(key)
        .cloned()
        .unwrap_or_else(|| json!(default.to_string()))
}

fn sorted_algorithm_versions(fixtures: &Value) -> Vec<String> {
    let versions: BTreeSet<String> = fixtures["cases"]
        .as_array()
        .expect("fixture cases")
        .iter()
        .map(|case| string(&case["input"]["algorithm_version"], "algorithm_version").to_owned())
        .collect();
    versions.into_iter().collect()
}

fn compare_input(input: &Value) -> metrics::MetricResult {
    let reference = input_values(input, "reference");
    let simulation = input_values(input, "candidate");
    let reference_weights = optional_weights(input, "reference_weights");
    let simulation_weights = optional_weights(input, "candidate_weights");
    let algorithm_version = string(&input["algorithm_version"], "algorithm_version");
    let origin = input.get("origin").and_then(Value::as_str);
    let scale_ticks = string(&input["scale_ticks"], "scale_ticks");
    metrics::compare(&MetricRequest {
        reference: &reference,
        simulation: &simulation,
        reference_weights: reference_weights.as_deref(),
        simulation_weights: simulation_weights.as_deref(),
        algorithm_version,
        origin,
        scale_ticks,
    })
}

fn raw_count(input: &Value, key: &str) -> usize {
    input[key]
        .as_array()
        .unwrap_or_else(|| panic!("input.{key} must be an array"))
        .len()
}

fn primary_dispositions(input: &Value, result: &metrics::MetricResult) -> Map<String, Value> {
    if let Some(dispositions) = input
        .get("counts")
        .and_then(|counts| counts.get("primary_dispositions"))
    {
        return object(dispositions, "counts.primary_dispositions").clone();
    }

    let raw_reference = raw_count(input, "reference");
    let raw_candidate = raw_count(input, "candidate");
    let raw_total = raw_reference + raw_candidate;
    let rejected = matches!(&result.status, MetricStatus::Invalid);
    let missing = if rejected {
        0
    } else {
        input_values(input, "reference")
            .iter()
            .chain(input_values(input, "candidate").iter())
            .filter(|value| value.is_none())
            .count()
    };
    let observed = if rejected {
        0
    } else {
        raw_total.saturating_sub(missing)
    };
    let mut counts = Map::new();
    for (key, value) in [
        ("censored", 0),
        ("failed", 0),
        ("infeasible", 0),
        ("missing", missing),
        ("observed", observed),
        ("rejected_input", if rejected { raw_total } else { 0 }),
    ] {
        counts.insert(key.to_owned(), json!(value.to_string()));
    }
    counts
}

fn diagnostic(input: &Value, result: &metrics::MetricResult) -> Value {
    let mut diagnostic = Map::new();
    diagnostic.insert(
        "raw_input_counts".into(),
        json!({
            "candidate": raw_count(input, "candidate").to_string(),
            "reference": raw_count(input, "reference").to_string()
        }),
    );
    diagnostic.insert(
        "primary_dispositions".into(),
        Value::Object(primary_dispositions(input, result)),
    );

    if matches!(&result.status, MetricStatus::Invalid) {
        diagnostic.insert(
            "attempted_support_counts".into(),
            json!({
                "candidate": raw_count(input, "candidate").to_string(),
                "reference": raw_count(input, "reference").to_string()
            }),
        );
    }

    if let Some(counts) = input.get("counts") {
        diagnostic.insert(
            "censor_subtypes".into(),
            counts
                .get("censor_subtypes")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        if let Some(value) = counts.get("overlap_counts") {
            diagnostic.insert("overlap_counts".into(), value.clone());
        }
    }

    if let Some(grouped) = input.get("grouped_observations") {
        let group_variable = string(&grouped["group_variable"], "group_variable");
        let compared_variable = string(&grouped["compared_variable"], "compared_variable");
        let mut groups = Vec::new();
        for group in grouped["prespecified_groups"]
            .as_array()
            .expect("prespecified_groups")
        {
            let group_id = group.as_str().expect("group is a string");
            let mut group_reference = Vec::<Option<&str>>::new();
            let mut group_simulation = Vec::<Option<&str>>::new();
            for (side, target) in [
                ("reference", &mut group_reference),
                ("candidate", &mut group_simulation),
            ] {
                for row in grouped[side].as_array().expect("group rows") {
                    if row[group_variable].as_str() == Some(group_id) {
                        target.push(row[compared_variable].as_str());
                    }
                }
            }
            let grouped_result = metrics::compare(&MetricRequest {
                reference: &group_reference,
                simulation: &group_simulation,
                reference_weights: None,
                simulation_weights: None,
                algorithm_version: "empirical_equal.v1",
                origin: None,
                scale_ticks: string(&input["scale_ticks"], "scale_ticks"),
            });
            groups.push(json!({
                "group": group_id,
                "eligible_candidate": grouped_result.simulation_count.to_string(),
                "eligible_reference": grouped_result.reference_count.to_string(),
                "status": status_name(&grouped_result.status),
                "w1": grouped_metric_string(grouped_result.w1),
                "ks_d": grouped_metric_string(grouped_result.ks_d)
            }));
        }
        diagnostic.insert("group_variable".into(), json!(group_variable));
        diagnostic.insert("compared_variable".into(), json!(compared_variable));
        diagnostic.insert("groups".into(), Value::Array(groups));
    }
    Value::Object(diagnostic)
}

fn grouped_metric_string(value: Option<f64>) -> Value {
    match value.filter(|v| v.is_finite()) {
        None => Value::Null,
        Some(value) if value.fract() == 0.0 => json!(format!("{value:.0}")),
        Some(value) => json!(value.to_string()),
    }
}

fn produce_case(case: &Value) -> Value {
    let input = &case["input"];
    let result = compare_input(input);
    let raw_total = raw_count(input, "candidate") + raw_count(input, "reference");
    let coverage_counts = input.get("counts");
    let eligible = coverage_counts
        .and_then(|c| c.get("eligible"))
        .cloned()
        .unwrap_or_else(|| json!(raw_total.to_string()));
    let excluded = json_count(coverage_counts.unwrap_or(&Value::Null), "excluded", 0);
    let cohort_total = coverage_counts
        .and_then(|c| c.get("eligible"))
        .and_then(Value::as_str)
        .and_then(|e| e.parse::<u128>().ok())
        .zip(
            coverage_counts
                .and_then(|c| c.get("excluded"))
                .and_then(Value::as_str)
                .and_then(|e| e.parse::<u128>().ok()),
        )
        .map(|(a, b)| {
            json!(a
                .checked_add(b)
                .expect("coverage total fits u128")
                .to_string())
        })
        .unwrap_or_else(|| eligible.clone());

    let mut counts = Map::new();
    counts.insert("candidate".into(), string_count(result.simulation_count));
    counts.insert("cohort_total".into(), cohort_total);
    counts.insert("eligible".into(), eligible);
    counts.insert("excluded".into(), excluded);
    counts.insert("reference".into(), string_count(result.reference_count));
    counts.insert("unmatched".into(), json!("0"));
    counts.insert("unmatched_estimand".into(), json!("not_applicable"));

    let warnings = input
        .get("coverage_warnings")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let output = json!({
        "status": status_name(&result.status),
        "w1": metric_number(result.w1),
        "ks_d": metric_number(result.ks_d),
        "unit": string(&input["unit"], "unit"),
        "scale_ticks": string(&input["scale_ticks"], "scale_ticks"),
        "algorithm_version": string(&input["algorithm_version"], "algorithm_version"),
        "counts": Value::Object(counts),
        "precision": precision_name(&result.precision),
        "weighting": if input.get("candidate_weights").is_some() || input.get("reference_weights").is_some() {
            "explicit"
        } else {
            "equal"
        },
        "warnings": warnings,
        "p_value": Value::Null,
        "diagnostic": diagnostic(input, &result)
    });
    json!({"id": string(&case["id"], "id"), "result": output})
}

fn rust_toolchain() -> String {
    if let Ok(version) = std::env::var("C42_TOOLCHAIN") {
        return version;
    }
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("rustc is available for producer provenance");
    assert!(output.status.success(), "read rustc version");
    String::from_utf8(output.stdout)
        .expect("rustc version output is utf8")
        .trim()
        .to_owned()
}

fn producer_commit(write_report: bool) -> String {
    if let Ok(commit) = std::env::var("C42_PRODUCER_COMMIT") {
        assert!(
            is_commit(&commit),
            "C42_PRODUCER_COMMIT must be forty hex digits"
        );
        return commit;
    }
    assert!(
        !write_report,
        "set C42_PRODUCER_COMMIT to the committed report producer source"
    );
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git is available for test provenance");
    assert!(output.status.success(), "read producer commit from git");
    let commit = String::from_utf8(output.stdout)
        .expect("git commit is utf8")
        .trim()
        .to_owned();
    assert!(
        is_commit(&commit),
        "producer commit must be forty hex digits"
    );
    commit
}

fn is_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn produce_candidate_report(fixtures: &Value, fixture_hash: &str, write_report: bool) -> Value {
    let cases = fixtures["cases"]
        .as_array()
        .expect("fixture cases array")
        .iter()
        .map(produce_case)
        .collect::<Vec<_>>();
    let algorithm_versions = sorted_algorithm_versions(fixtures);
    let mut source_bytes = include_bytes!("metrics_c42.rs").to_vec();
    source_bytes.extend_from_slice(include_bytes!("../src/metrics.rs"));
    let report = json!({
        "schema_version": CANDIDATE_SCHEMA,
        "fixture_sha256": fixture_hash,
        "provenance": {
            "kind": "runtime_candidate",
            "producer_commit": producer_commit(write_report),
            "toolchain": rust_toolchain(),
            "algorithm_versions": algorithm_versions,
            "producer_api": PRODUCER_API,
            "producer_source_sha256": sha256(&source_bytes)
        },
        "cases": cases
    });
    if write_report {
        let path = std::env::var("C42_REPORT_PATH").expect("C42_REPORT_PATH is set");
        let relative_path = PathBuf::from(&path);
        let mut components = relative_path.components();
        assert!(
            !relative_path.is_absolute()
                && matches!(components.next(), Some(std::path::Component::Normal(x)) if x == ".artifacts")
                && matches!(components.next(), Some(std::path::Component::Normal(x)) if x == "c42-report")
                && components.all(|part| matches!(part, std::path::Component::Normal(_))),
            "C42_REPORT_PATH must stay within .artifacts/c42-report"
        );
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut bytes = serde_json::to_vec_pretty(&report).expect("serialize candidate report");
        bytes.push(b'\n');
        let report_path = repo_root.join(relative_path);
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent).expect("create report output directory");
        }
        fs::write(report_path, bytes).expect("write actual C4.2 candidate report");
    }
    report
}

fn fixtures_without_oracle(bytes: &[u8]) -> Value {
    let mut fixtures: Value = serde_json::from_slice(bytes).expect("fixture JSON parses");
    for case in fixtures["cases"].as_array_mut().expect("fixture cases") {
        case.as_object_mut().unwrap().remove("expected");
    }
    fixtures
}

#[test]
fn default_actual_producer_executes_all_frozen_cases() {
    let bytes = fixture_bytes();
    let fixtures: Value = serde_json::from_slice(&bytes).expect("fixture JSON parses");
    let write_report = std::env::var_os("C42_REPORT_PATH").is_some();
    let report = produce_candidate_report(&fixtures, &sha256(&bytes), write_report);
    assert_eq!(report["cases"].as_array().unwrap().len(), 42);
    assert_eq!(report["schema_version"], CANDIDATE_SCHEMA);
    assert_eq!(report["fixture_sha256"], sha256(&bytes));
}

#[test]
fn producer_output_is_invariant_to_removed_or_mutated_expected_values() {
    let bytes = fixture_bytes();
    let fixtures: Value = serde_json::from_slice(&bytes).expect("fixture JSON parses");
    let stripped = fixtures_without_oracle(&bytes);
    let mut mutated = stripped.clone();
    for case in mutated["cases"].as_array_mut().unwrap() {
        case["expected"] = json!({"w1":"poison", "counts":{"candidate":"999"}});
    }
    let hash = sha256(&bytes);
    let report_without_expected = produce_candidate_report(&stripped, &hash, false);
    let report_mutated_expected = produce_candidate_report(&mutated, &hash, false);
    assert_eq!(report_without_expected, report_mutated_expected);
    assert_eq!(
        report_without_expected["cases"],
        produce_candidate_report(&fixtures, &hash, false)["cases"]
    );
}

#[test]
fn actual_conditional_metric_exposes_opposite_dependence_and_keeps_empty_group() {
    let bytes = fixture_bytes();
    let fixtures: Value = serde_json::from_slice(&bytes).expect("fixture JSON parses");
    let case = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "conditional-opposite")
        .unwrap();
    let produced = produce_case(case);
    assert_eq!(produced["result"]["w1"], json!(0.0));
    assert_eq!(produced["result"]["ks_d"], json!(0.0));
    assert_eq!(produced["result"]["diagnostic"]["groups"][0]["w1"], "1");
    assert_eq!(produced["result"]["diagnostic"]["groups"][1]["w1"], "1");
    assert_eq!(
        produced["result"]["diagnostic"]["groups"][2]["status"],
        "empty"
    );
    assert_eq!(
        produced["result"]["diagnostic"]["groups"][2]["w1"],
        Value::Null
    );
}

#[test]
fn nulls_remain_missing_while_malformed_and_weight_errors_are_rejected() {
    let bytes = fixture_bytes();
    let fixtures: Value = serde_json::from_slice(&bytes).expect("fixture JSON parses");
    let cases = fixtures["cases"].as_array().unwrap();
    let find = |id: &str| cases.iter().find(|case| case["id"] == id).unwrap();
    let nulls = produce_case(find("null-points"));
    assert_eq!(nulls["result"]["status"], "ok");
    assert_eq!(nulls["result"]["counts"]["reference"], "1");
    assert_eq!(
        nulls["result"]["diagnostic"]["primary_dispositions"]["missing"],
        "1"
    );
    for id in ["malformed-value", "malformed-weight", "bad-weight-length"] {
        let invalid = produce_case(find(id));
        assert_eq!(invalid["result"]["status"], "invalid", "{id}");
        assert_eq!(invalid["result"]["w1"], Value::Null, "{id}");
        assert_ne!(
            invalid["result"]["diagnostic"]["primary_dispositions"]["rejected_input"],
            "0"
        );
    }
}

#[test]
fn optional_report_requires_committed_source_binding() {
    if std::env::var_os("C42_REPORT_PATH").is_none() {
        return;
    }
    assert!(std::env::var("C42_PRODUCER_COMMIT")
        .as_deref()
        .is_ok_and(is_commit));
}
