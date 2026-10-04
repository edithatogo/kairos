//! C4.1 fixture and candidate-report contract checks.
//!
//! These tests validate the fixture protocol and a report comparator. They do
//! not execute or claim conformance for a production metric implementation.
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const FIXTURE_PATH: &str = "conformance/c41/fixtures.json";
const CANDIDATE_SCHEMA: &str = "c41.candidate.v1";
const RESULT_KEYS: &[&str] = &[
    "status",
    "w1",
    "ks_d",
    "unit",
    "scale_ticks",
    "algorithm_version",
    "counts",
    "precision",
    "weighting",
    "warnings",
    "p_value",
    "diagnostic",
];

fn fixture_bytes() -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(FIXTURE_PATH),
    )
    .expect("C4.1 fixture file must be integrated at conformance/c41/fixtures.json")
}
fn fixtures() -> Value {
    serde_json::from_slice(&fixture_bytes()).expect("fixture JSON parses")
}
fn fixture_sha() -> String {
    Sha256::digest(fixture_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn obj<'a>(v: &'a Value, where_: &str) -> &'a Map<String, Value> {
    v.as_object()
        .unwrap_or_else(|| panic!("{where_} must be an object"))
}
fn strv<'a>(v: &'a Value, where_: &str) -> &'a str {
    v.as_str()
        .unwrap_or_else(|| panic!("{where_} must be a string"))
}
fn rational(v: &Value, where_: &str) -> f64 {
    let s = strv(v, where_);
    let mut p = s.split('/');
    let n: f64 = p
        .next()
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("{where_} rational numerator"));
    let d: f64 = p
        .next()
        .map(|x| x.parse().expect("rational denominator"))
        .unwrap_or(1.0);
    assert!(p.next().is_none() && d != 0.0, "{where_} rational form");
    n / d
}
fn expected_algorithm_versions(fx: &Value) -> BTreeSet<String> {
    fx["cases"]
        .as_array()
        .expect("cases array")
        .iter()
        .map(|c| {
            strv(
                &c["expected"]["algorithm_version"],
                "expected algorithm_version",
            )
            .to_owned()
        })
        .collect()
}

fn validate_fixture(fx: &Value) {
    assert_eq!(
        strv(&fx["schema_version"], "schema_version"),
        "c41.fixtures.v1"
    );
    assert!(fx["provenance"].is_object(), "fixture provenance required");
    let cases = fx["cases"].as_array().expect("cases array");
    assert!(
        cases.len() >= 30,
        "C4.1 fixture coverage requires at least 30 cases"
    );
    let mut ids = BTreeSet::new();
    for c in cases {
        let id = strv(&c["id"], "id");
        assert!(ids.insert(id), "duplicate fixture id {id}");
        assert!(!strv(&c["family"], "family").is_empty());
        assert!(c["input"].is_object() && c["expected"].is_object());
        let e = &c["expected"];
        assert!(matches!(
            strv(&e["status"], "status"),
            "ok" | "empty" | "invalid" | "insufficient_data"
        ));
        for k in [
            "unit",
            "scale_ticks",
            "algorithm_version",
            "precision",
            "weighting",
        ] {
            assert!(!strv(&e[k], k).is_empty(), "{k} required");
        }
        assert!(matches!(
            strv(&e["precision"], "precision"),
            "exact_offsets" | "rejected" | "not_applicable"
        ));
        assert!(matches!(
            strv(&e["weighting"], "weighting"),
            "equal" | "explicit"
        ));
        assert!(e["counts"].is_object() && e["warnings"].is_array());
        assert!(e["p_value"].is_null(), "C4.1 does not produce p-values");
        for m in ["w1", "ks_d"] {
            if !e[m].is_null() {
                let n = rational(&e[m], m);
                assert!(n.is_finite(), "finite rational expected");
            }
        }
        if e["status"] != "ok" {
            assert!(
                e["w1"].is_null() && e["ks_d"].is_null(),
                "non-ok cases have null metrics"
            );
        }
    }
}
fn close(got: &Value, want: &Value, key: &str, scale_ticks: &Value) -> bool {
    if want.is_null() {
        return got.is_null();
    }
    let Some(g) = got.as_f64() else { return false };
    let w = rational(want, key);
    let tol = if key == "w1" {
        1e-12 * rational(scale_ticks, "scale_ticks").max(1.0)
    } else {
        1e-12
    };
    g.is_finite()
        && if key == "w1" {
            g >= 0.0
        } else {
            (0.0..=1.0).contains(&g)
        }
        && (g - w).abs() <= tol
}
fn compare_report(fx: &Value, report: &Value, bytes_hash: &str) -> Result<(), String> {
    validate_fixture(fx);
    let r = obj(report, "candidate report");
    if r.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != ["schema_version", "fixture_sha256", "provenance", "cases"]
            .into_iter()
            .collect()
    {
        return Err("candidate report keys".into());
    }
    if strv(
        r.get("schema_version").unwrap_or(&Value::Null),
        "schema_version",
    ) != CANDIDATE_SCHEMA
    {
        return Err("schema_version".into());
    }
    if r.get("fixture_sha256").and_then(Value::as_str) != Some(bytes_hash) {
        return Err("fixture_sha256".into());
    }
    let p = obj(r.get("provenance").ok_or("provenance")?, "provenance");
    for k in ["producer_commit", "toolchain"] {
        if p.get(k)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            return Err(format!("provenance.{k}"));
        }
    }
    let av = p
        .get("algorithm_versions")
        .ok_or("provenance.algorithm_versions")?;
    if av != &json!(expected_algorithm_versions(fx)) {
        return Err("provenance.algorithm_versions".into());
    }
    let cases = r.get("cases").and_then(Value::as_array).ok_or("cases")?;
    let mut seen = BTreeSet::new();
    if cases.len() != fx["cases"].as_array().unwrap().len() {
        return Err("case membership length".into());
    }
    for c in cases {
        let case_obj = obj(c, "case");
        if case_obj.keys().map(String::as_str).collect::<BTreeSet<_>>()
            != ["id", "result"].into_iter().collect()
        {
            return Err("case keys".into());
        }
        let id = c.get("id").and_then(Value::as_str).ok_or("case id")?;
        if !seen.insert(id.to_owned()) {
            return Err("duplicate case id".into());
        }
        let Some(expected_case) = fx["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
        else {
            return Err("extra case id".into());
        };
        let result = c.get("result").ok_or("result")?;
        let got = obj(result, "result");
        let want = obj(&expected_case["expected"], "expected");
        for k in got.keys() {
            if !RESULT_KEYS.contains(&k.as_str()) {
                return Err(format!("extra result key {k}"));
            }
        }
        for k in [
            "status",
            "unit",
            "scale_ticks",
            "algorithm_version",
            "counts",
            "precision",
            "weighting",
            "warnings",
            "p_value",
        ] {
            if got.get(k) != want.get(k) {
                return Err(format!("metadata {id}.{k}"));
            }
        }
        for k in ["w1", "ks_d"] {
            if !close(
                got.get(k).ok_or(k)?,
                want.get(k).ok_or(k)?,
                k,
                &expected_case["expected"]["scale_ticks"],
            ) {
                return Err(format!("numeric {id}.{k}"));
            }
        }
        if got.get("diagnostic") != want.get("diagnostic") {
            return Err(format!("diagnostic {id}"));
        }
    }
    if seen.len() != fx["cases"].as_array().unwrap().len() {
        return Err("missing case".into());
    }
    Ok(())
}
fn compare_runtime_report(fx: &Value, report: &Value, hash: &str) -> Result<(), String> {
    let p = report
        .get("provenance")
        .and_then(Value::as_object)
        .ok_or("provenance")?;
    if p.get("kind").and_then(Value::as_str) != Some("runtime_candidate") {
        return Err("runtime gate rejects comparator mocks".into());
    }
    let commit = p
        .get("producer_commit")
        .and_then(Value::as_str)
        .ok_or("producer_commit")?;
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("runtime producer requires exact commit".into());
    }
    if p.get("producer_api")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .is_none()
    {
        return Err("runtime producer API binding required".into());
    }
    compare_report(fx, report, hash)
}
#[test]
fn runtime_gate_rejects_mock_and_missing_producer_binding() {
    let fx = fixtures();
    let mock = mock_report(&fx);
    let hash = fixture_sha();
    assert!(compare_runtime_report(&fx, &mock, &hash).is_err());
    let mut unbound = mock.clone();
    unbound["provenance"]["kind"] = json!("runtime_candidate");
    unbound["provenance"]["producer_commit"] = json!("a".repeat(40));
    assert!(compare_runtime_report(&fx, &unbound, &hash).is_err());
}
fn mock_report(fx: &Value) -> Value {
    let cases: Vec<Value> = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let mut result = c["expected"].clone();
            for k in ["w1", "ks_d"] {
                if !result[k].is_null() {
                    result[k] = json!(rational(&result[k], k));
                }
            }
            json!({"id": c["id"], "result": result})
        })
        .collect();
    json!({"schema_version": CANDIDATE_SCHEMA, "fixture_sha256": fixture_sha(), "provenance": {"kind":"comparator_mock", "producer_commit":"MOCK-COMPARATOR-SELF-TEST", "toolchain":"MOCK", "algorithm_versions": expected_algorithm_versions(fx)}, "cases": cases})
}
#[test]
fn fixture_protocol_and_coverage() {
    validate_fixture(&fixtures());
}
#[test]
fn reference_expected_values_are_exact_rationals_and_protocol_coherent() {
    let fx = fixtures();
    let mut families = BTreeSet::new();
    for c in fx["cases"].as_array().unwrap() {
        families.insert(strv(&c["family"], "family").to_owned());
        let e = &c["expected"];
        if e["status"] == "ok" {
            assert!(
                !e["w1"].is_null() && !e["ks_d"].is_null(),
                "ok case has both metrics"
            );
        }
        if e["precision"] == "rejected" {
            assert!(e["w1"].is_null() && e["ks_d"].is_null());
        }
    }
    let tail = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["family"] == "tail_w1_180_d_1_5")
        .expect("prespecified tail W1/KS fixture");
    assert_eq!(tail["expected"]["w1"], "180");
    assert_eq!(tail["expected"]["ks_d"], "1/5");
    for required in [
        "shift",
        "identical",
        "ties",
        "weighted",
        "duration_scaling",
        "empty",
        "invalid",
        "coverage",
        "large_common_origin",
        "precision_rejection",
    ] {
        assert!(
            families.iter().any(|f| f.contains(required)),
            "missing required family {required}"
        );
    }
}
#[test]
fn comparator_accepts_explicitly_labeled_mock_only() {
    let fx = fixtures();
    let mock = mock_report(&fx);
    assert_eq!(
        mock["provenance"]["producer_commit"],
        "MOCK-COMPARATOR-SELF-TEST"
    );
    compare_report(&fx, &mock, &fixture_sha()).expect("mock comparator self-test");
}
#[test]
fn comparator_rejects_contract_mutations() {
    let fx = fixtures();
    let good = mock_report(&fx);
    let hash = fixture_sha();
    let mut cases = Vec::new();
    macro_rules! bad {
        ($edit:expr) => {{
            let mut x = good.clone();
            $edit(&mut x);
            cases.push(x);
        }};
    }
    bad!(|x: &mut Value| x["schema_version"] = json!("wrong"));
    bad!(|x: &mut Value| x["fixture_sha256"] = json!("00"));
    bad!(|x: &mut Value| x["provenance"]["producer_commit"] = json!(""));
    bad!(|x: &mut Value| x["provenance"]["algorithm_versions"] = json!({}));
    bad!(|x: &mut Value| x["provenance"]["algorithm_versions"] = json!(["wrong"]));
    bad!(|x: &mut Value| x["cases"][0]["result"]["unit"] = json!("wrong"));
    bad!(|x: &mut Value| x["cases"][0]["result"]["algorithm_version"] = json!("wrong"));
    bad!(|x: &mut Value| x["cases"][0]["result"]["precision"] = json!("wrong"));
    bad!(|x: &mut Value| x["cases"][0]["result"]["weighting"] = json!("wrong"));
    bad!(|x: &mut Value| x["cases"][0]["result"]["warnings"] = json!(["wrong"]));
    bad!(|x: &mut Value| x["cases"][0]["result"]["counts"] = json!({"wrong":"1"}));
    bad!(|x: &mut Value| x["cases"][0]["result"]["p_value"] = json!(0.5));
    bad!(|x: &mut Value| x["cases"][0]["result"]["unexpected"] = json!(1));
    bad!(|x: &mut Value| x["cases"][0]["id"] = x["cases"][1]["id"].clone());
    bad!(|x: &mut Value| {
        let _ = x["cases"].as_array_mut().unwrap().pop();
    });
    bad!(|x: &mut Value| x["cases"][0]["result"]["w1"] = json!(f64::NAN));
    bad!(|x: &mut Value| x["cases"][0]["result"]["ks_d"] = json!(2.0));
    bad!(|x: &mut Value| x["cases"][0]["result"]["w1"] = json!(-1.0));
    for bad_report in cases {
        assert!(
            compare_report(&fx, &bad_report, &hash).is_err(),
            "mutated candidate unexpectedly accepted"
        );
    }
}
#[test]
fn comparator_enforces_numeric_tolerance_boundaries_and_domains() {
    let fx = fixtures();
    let good = mock_report(&fx);
    let hash = fixture_sha();
    let tail_index = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["family"] == "tail_w1_180_d_1_5")
        .unwrap();
    let w1 = rational(&fx["cases"][tail_index]["expected"]["w1"], "w1");
    let d = rational(&fx["cases"][tail_index]["expected"]["ks_d"], "ks_d");
    let scale = rational(
        &fx["cases"][tail_index]["expected"]["scale_ticks"],
        "scale_ticks",
    )
    .max(1.0);
    let wt = 1e-12 * scale;
    let dt = 1e-12;
    let mut inside = good.clone();
    inside["cases"][tail_index]["result"]["w1"] = json!(w1 + wt * 0.5);
    inside["cases"][tail_index]["result"]["ks_d"] = json!(d + dt * 0.5);
    assert!(
        compare_report(&fx, &inside, &hash).is_ok(),
        "values within tolerance accepted"
    );
    for (key, value) in [("w1", w1 + wt * 1.1), ("ks_d", d + dt * 1.1)] {
        let mut outside = good.clone();
        outside["cases"][tail_index]["result"][key] = json!(value);
        assert!(
            compare_report(&fx, &outside, &hash).is_err(),
            "{key} beyond tolerance rejected"
        );
    }
    for (key, value) in [("w1", -1e-15), ("ks_d", -1e-15), ("ks_d", 1.0 + 1e-15)] {
        let mut invalid = good.clone();
        invalid["cases"][tail_index]["result"][key] = json!(value);
        assert!(
            compare_report(&fx, &invalid, &hash).is_err(),
            "{key} outside domain rejected"
        );
    }
}
#[test]
fn comparator_rejects_omitted_metadata_and_wrong_algorithm_set() {
    let fx = fixtures();
    let good = mock_report(&fx);
    let hash = fixture_sha();
    let tail_index = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["family"] == "tail_w1_180_d_1_5")
        .unwrap();
    let censored_index = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["expected"].get("diagnostic").is_some())
        .unwrap();
    let mut status = good.clone();
    status["cases"][tail_index]["result"]["status"] = json!("empty");
    let mut null = good.clone();
    null["cases"][tail_index]["result"]["w1"] = Value::Null;
    let mut diagnostic = good.clone();
    diagnostic["cases"][censored_index]["result"]
        .as_object_mut()
        .unwrap()
        .remove("diagnostic");
    let mut counts = good.clone();
    counts["cases"][tail_index]["result"]
        .as_object_mut()
        .unwrap()
        .remove("counts");
    let mut algorithms = good.clone();
    algorithms["provenance"]["algorithm_versions"] = json!(["empirical_equal.v1"]);
    let mut top_extra = good.clone();
    top_extra["unexpected"] = json!(true);
    let mut case_extra = good.clone();
    case_extra["cases"][tail_index]["unexpected"] = json!(true);
    for report in [
        status, null, diagnostic, counts, algorithms, top_extra, case_extra,
    ] {
        assert!(compare_report(&fx, &report, &hash).is_err());
    }
}

#[test]
fn comparator_rejects_fixture_mutations() {
    let good = fixtures();
    for mutate in 0..5 {
        let mut fx = good.clone();
        match mutate {
            0 => fx["schema_version"] = json!("wrong"),
            1 => fx["cases"][0]["id"] = fx["cases"][1]["id"].clone(),
            2 => fx["cases"][0]["expected"]["p_value"] = json!(0.1),
            3 => fx["cases"][0]["expected"]["precision"] = json!("unknown"),
            _ => fx["cases"][0]["expected"]["w1"] = json!("NaN"),
        }
        assert!(std::panic::catch_unwind(|| validate_fixture(&fx)).is_err());
    }
}
#[test]
#[ignore = "C4.2 runtime producer is not implemented; candidate report required"]
fn c42_candidate_report() {
    let path = std::env::var("C41_CANDIDATE_REPORT")
        .expect("set C41_CANDIDATE_REPORT to an actual C4.2 candidate report");
    let bytes = fs::read(path).expect("read actual candidate report");
    let report: Value = serde_json::from_slice(&bytes).expect("candidate report JSON");
    compare_runtime_report(&fixtures(), &report, &fixture_sha())
        .expect("C4.2 candidate report conforms");
}
