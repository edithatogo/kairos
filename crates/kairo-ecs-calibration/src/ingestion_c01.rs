//! Private exact-byte C-01 qualification for the experimental ingestion path.
use crate::{
    ingestion_normalize::Limits,
    ingestion_pipeline::{fingerprint, ingest, PipelineConfig},
    ingestion_sort::SortLimits,
    ingestion_validate::{ValidationBounds, ValidationMode, ValidationPolicy},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const POPULATIONS: [(&str, &str); 4] = [
    ("events", "events.ndjson"),
    ("quarantine", "quarantine.ndjson"),
    ("exclusions", "exclusions.ndjson"),
    ("outcomes", "outcomes.ndjson"),
];
const PROFILES: [&str; 3] = ["long-valid", "wide-valid", "long-quarantine"];
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct Profile {
    name: &'static str,
    template: Value,
    rows: Vec<Value>,
    validation_mode: ValidationMode,
    expected: Expected,
}

#[derive(Clone, Copy)]
struct Expected {
    source_rows: u64,
    candidates: u64,
    accepted: u64,
    excluded: u64,
    events: u64,
    quarantined: u64,
    outcomes: u64,
}

struct Baseline {
    populations: BTreeMap<String, Vec<u8>>,
    semantic: Value,
    manifest: Value,
    source_hash: String,
    source_bytes: u64,
}

struct CachedExecution {
    profile: String,
    ordered_ndjson: Vec<u8>,
    template: Vec<u8>,
    frozen_config: Vec<u8>,
    policy: Vec<u8>,
    evidence: Vec<u8>,
    limits: (usize, usize, usize),
    execution_id: String,
    receipt: Value,
}

fn fresh_dir(parent: &Path, stem: &str) -> PathBuf {
    fs::create_dir_all(parent).unwrap();
    for _ in 0..128 {
        let n = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!("{stem}-{}-{n}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("cannot create private C-01 directory: {e}"),
        }
    }
    panic!("cannot allocate private C-01 directory")
}

fn clock(second: u64) -> Value {
    json!({"raw":format!("2020-01-01T00:00:{second:02}Z"),"representation":"RFC3339","precision":"second","lineage":{"status":"observed"}})
}

fn base_template(shape: &str, bindings: Vec<Value>) -> Value {
    json!({"profile_version":"c11.synthetic-map-v1","dataset_id":"synthetic-c01","mapping_version":"synthetic-map-v1","origin_utc":"2020-01-01T00:00:00Z","case_key_field":"case","source_family":"fixture","shape":shape,"event_kind_field":"kind","resource_key_field":"resource","location_key_field":"location","event_bindings":bindings,"rows":[]})
}

fn long_binding_rows() -> (Value, Vec<Value>) {
    let bindings = [
        ("arrival", "ARRIVAL", 0),
        ("bed_entered", "BED_ENTERED", 1),
        ("physical_departure", "PHYSICAL_DEPARTURE", 2),
    ]
    .into_iter()
    .map(|(kind, ty, rank)| json!({"source_event_type":ty,"kind":kind,"rank":rank.to_string(),"occurrence_index":0,"occurrence_field":"at","key_field":"id","order_field":"seq"}))
    .collect();
    let template = base_template("long", bindings);
    let mut rows = Vec::new();
    for (case, times) in [("Z", [0, 1, 4]), ("A", [2, 4, 7])] {
        for (kind, second) in ["arrival", "bed_entered", "physical_departure"]
            .into_iter()
            .zip(times)
        {
            let mut row = json!({"case":case,"id":format!("{case}-{kind}"),"seq":rows.len(),"kind":kind,"at":clock(second),"resource":"bed-1","location":"zone-1","qualification_marker":"stable"});
            if kind == "physical_departure" {
                row["outcome"] = outcome(times[0], second, case == "A");
            }
            rows.push(row);
        }
    }
    rows.push(json!({"case":"E","id":"excluded-missing","seq":6,"kind":"arrival","at":null,"resource":"bed-1","location":"zone-1","qualification_marker":"stable"}));
    (template, rows)
}

fn wide_binding_rows() -> (Value, Vec<Value>) {
    let bindings: Vec<Value> = [
        ("arrival", "ARRIVAL", 0),
        ("bed_entered", "BED_ENTERED", 1),
        ("physical_departure", "PHYSICAL_DEPARTURE", 2),
    ]
    .into_iter()
    .map(|(kind, ty, rank)| {
        json!({"source_event_type":ty,"kind":kind,"rank":rank.to_string(),"occurrence_index":0,"occurrence_field":format!("{kind}_at"),"key_field":format!("{kind}_id"),"order_field":format!("{kind}_seq")})
    })
    .collect();
    let template = base_template("wide", bindings);
    let mut rows = Vec::new();
    for (case, times) in [("Z", [0, 1, 4]), ("A", [2, 4, 7]), ("E", [u64::MAX; 3])] {
        let mut row = json!({"case":case,"resource":"bed-1","location":"zone-1","qualification_marker":"stable"});
        for (index, kind) in ["arrival", "bed_entered", "physical_departure"]
            .into_iter()
            .enumerate()
        {
            row[format!("{kind}_id")] = json!(format!("{case}-{kind}"));
            row[format!("{kind}_seq")] = json!(index as u64);
            row[format!("{kind}_at")] = if case == "E" {
                Value::Null
            } else {
                clock(times[index])
            };
        }
        if case != "E" {
            row["outcome"] = outcome(times[0], times[2], case == "A");
        }
        rows.push(row);
    }
    (template, rows)
}

fn outcome(start: u64, end: u64, censored: bool) -> Value {
    json!({"endpoint":"departure","risk_start":clock(start),"last_observed":clock(end),"event_clock":if censored{Value::Null}else{clock(end)},"censor_cause":if censored{"window_end"}else{"departed"},"censor_status":if censored{"right"}else{"not_censored"},"lineage":{"status":"observed"}})
}

fn profiles() -> Vec<Profile> {
    let (long_template, long_rows) = long_binding_rows();
    let (wide_template, wide_rows) = wide_binding_rows();
    let mut quarantine_rows = long_rows.clone();
    quarantine_rows[1]["at"] = clock(8);
    vec![
        Profile {
            name: "long-valid",
            template: long_template,
            rows: long_rows,
            validation_mode: ValidationMode::Strict,
            expected: Expected {
                source_rows: 7,
                candidates: 7,
                accepted: 6,
                excluded: 1,
                events: 6,
                quarantined: 0,
                outcomes: 2,
            },
        },
        Profile {
            name: "wide-valid",
            template: wide_template,
            rows: wide_rows,
            validation_mode: ValidationMode::Strict,
            expected: Expected {
                source_rows: 3,
                candidates: 9,
                accepted: 6,
                excluded: 3,
                events: 6,
                quarantined: 0,
                outcomes: 2,
            },
        },
        Profile {
            name: "long-quarantine",
            template: long_binding_rows().0,
            rows: quarantine_rows,
            validation_mode: ValidationMode::Quarantine,
            expected: Expected {
                source_rows: 7,
                candidates: 7,
                accepted: 6,
                excluded: 1,
                events: 3,
                quarantined: 3,
                outcomes: 2,
            },
        },
    ]
}

fn build_evidence() -> Value {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let commit = std::process::Command::new("git")
        .args(["-C", cwd.to_str().unwrap(), "rev-parse", "HEAD"])
        .output()
        .expect("git available");
    assert!(commit.status.success());
    let rustc =
        std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .arg("--version")
            .output()
            .expect("rustc available");
    assert!(rustc.status.success());
    let lock_hash = fingerprint(&cwd.join("../../Cargo.lock")).unwrap().0;
    json!({"engine_commit":String::from_utf8(commit.stdout).unwrap().trim(),"cargo_lock_sha256":lock_hash,"toolchain":String::from_utf8(rustc.stdout).unwrap().trim(),"features":[],"scope":"private C-01 synthetic qualification"})
}

fn config(
    evidence: &Value,
    profile: &Profile,
    chunk_rows: usize,
    run_rows: usize,
    run_bytes: usize,
) -> PipelineConfig {
    PipelineConfig {
        evidence: evidence.clone(),
        normalization: Limits {
            max_chunk_rows: chunk_rows,
            max_chunk_bytes: 1 << 20,
            max_identities: 100,
        },
        sorting: SortLimits {
            max_run_rows: run_rows,
            max_run_bytes: run_bytes,
            max_record_bytes: 1 << 16,
            merge_fan_in: 2,
        },
        validation: ValidationPolicy {
            mode: profile.validation_mode,
            declared_kinds: BTreeSet::from([
                "arrival".into(),
                "bed_entered".into(),
                "physical_departure".into(),
            ]),
            required_kinds: vec![
                "arrival".into(),
                "bed_entered".into(),
                "physical_departure".into(),
            ],
            precedence: vec![
                ("arrival".into(), "bed_entered".into()),
                ("bed_entered".into(), "physical_departure".into()),
            ],
            occupancy_pairs: vec![("bed_entered".into(), "physical_departure".into())],
            capacities: BTreeMap::from([(("bed-1".into(), "zone-1".into()), 1)]),
            window: Some((0, 10_000_000_000)),
            bounds: ValidationBounds {
                max_cases: 100,
                max_state_entries: 1000,
                max_record_bytes: 1 << 16,
            },
        },
    }
}

fn frozen_policy(profile: &Profile) -> Value {
    json!({
        "mode": format!("{:?}", profile.validation_mode),
        "declared_kinds": ["arrival", "bed_entered", "physical_departure"],
        "required_kinds": ["arrival", "bed_entered", "physical_departure"],
        "precedence": [["arrival", "bed_entered"], ["bed_entered", "physical_departure"]],
        "occupancy_pairs": [["bed_entered", "physical_departure"]],
        "capacities": [{"resource":"bed-1", "location":"zone-1", "capacity":1}],
        "window_ns": ["0", "10000000000"],
        "bounds": {"max_cases":100, "max_state_entries":1000, "max_record_bytes":65536}
    })
}

fn write_json(path: &Path, value: &Value) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    serde_json::to_writer(&mut file, value).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
}

fn write_rows(path: &Path, rows: &[Value]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    for row in rows {
        serde_json::to_writer(&mut file, row).unwrap();
        file.write_all(b"\n").unwrap();
    }
    file.sync_all().unwrap();
}

fn expected_json(e: Expected) -> Value {
    json!({"source_rows":e.source_rows,"candidate_units":e.candidates,"mapper_accepted_units":e.accepted,"mapper_excluded_units":e.excluded,"events":e.events,"quarantined_events":e.quarantined,"outcomes":e.outcomes})
}

fn semantic(manifest: &Value) -> Value {
    json!({"dataset_id":manifest["dataset_id"],"mapping_version":manifest["mapping_version"],"mapping_sha256":manifest["mapping_sha256"],"source_rows":manifest["source_rows"],"candidate_units":manifest["candidate_units"],"mapper_accepted_units":manifest["mapper_accepted_units"],"mapper_excluded_units":manifest["mapper_excluded_units"],"failed_units":manifest["failed_units"],"unresolved_units":manifest["unresolved_units"],"cohort_denominator":manifest["cohort_denominator"],"missing_triage":manifest["missing_triage"],"outcomes":manifest["outcomes"],"censor_status_counts":manifest["censor_status_counts"],"validation":{"input_events":manifest["validation"]["input_events"],"valid_events":manifest["validation"]["valid_events"],"quarantined_events":manifest["validation"]["quarantined_events"],"invalid_cases":manifest["validation"]["invalid_cases"],"censored_cases":manifest["validation"]["censored_cases"],"reasons":manifest["validation"]["reasons"],"resource_feasible":manifest["validation"]["resource_feasible"]},"populations":manifest["populations"]})
}

fn population_receipt(dir: &Path) -> (BTreeMap<String, Vec<u8>>, Value) {
    let mut bytes_by_name = BTreeMap::new();
    let mut stats = serde_json::Map::new();
    for (name, file) in POPULATIONS {
        let bytes = fs::read(dir.join(file)).unwrap_or_else(|e| panic!("missing {file}: {e}"));
        let rows = bytes.iter().filter(|b| **b == b'\n').count();
        let (hash, size) = fingerprint(&dir.join(file)).unwrap();
        assert_eq!(size as usize, bytes.len());
        stats.insert(name.into(), json!({"sha256":hash,"bytes":size,"rows":rows}));
        bytes_by_name.insert(name.into(), bytes);
    }
    (bytes_by_name, Value::Object(stats))
}

fn parsed_records(populations: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<Value>> {
    populations
        .iter()
        .map(|(name, bytes)| {
            let text = std::str::from_utf8(bytes).expect("canonical NDJSON is UTF-8");
            let records = text
                .lines()
                .map(|line| serde_json::from_str(line).expect("canonical NDJSON record is JSON"))
                .collect();
            (name.clone(), records)
        })
        .collect()
}

fn semantic_counts(manifest: &Value) -> Value {
    json!({"source_rows":manifest["source_rows"],"candidate_units":manifest["candidate_units"],"mapper_accepted_units":manifest["mapper_accepted_units"],"mapper_excluded_units":manifest["mapper_excluded_units"],"failed_units":manifest["failed_units"],"unresolved_units":manifest["unresolved_units"],"valid_events":manifest["validation"]["valid_events"],"quarantined_events":manifest["validation"]["quarantined_events"],"outcomes":manifest["outcomes"],"validation_reasons":manifest["validation"]["reasons"],"censor_status_counts":manifest["censor_status_counts"]})
}

fn assert_expected(manifest: &Value, e: Expected) {
    assert_eq!(manifest["source_rows"], json!(e.source_rows));
    assert_eq!(manifest["candidate_units"], json!(e.candidates));
    assert_eq!(manifest["mapper_accepted_units"], json!(e.accepted));
    assert_eq!(manifest["mapper_excluded_units"], json!(e.excluded));
    assert_eq!(manifest["failed_units"], json!(0));
    assert_eq!(manifest["unresolved_units"], json!(0));
    assert_eq!(manifest["candidate_conservation"], json!(true));
    assert_eq!(manifest["validation"]["valid_events"], json!(e.events));
    assert_eq!(
        manifest["validation"]["quarantined_events"],
        json!(e.quarantined)
    );
    assert_eq!(manifest["outcomes"], json!(e.outcomes));
    let actual = manifest["populations"].as_object().unwrap();
    for (name, expected_rows) in [
        ("events", e.events),
        ("quarantine", e.quarantined),
        ("exclusions", e.excluded),
        ("outcomes", e.outcomes),
    ] {
        assert_eq!(
            actual[name]["rows"],
            json!(expected_rows),
            "{name} population must be non-vacuous as declared"
        );
    }
}

fn diagnostic_receipt(dir: &Path) -> Value {
    let path = dir.join("diagnostics.ndjson");
    let bytes = fs::read(&path).expect("diagnostics must be retained");
    let rows = bytes.iter().filter(|b| **b == b'\n').count();
    let (sha256, bytes_len) = fingerprint(&path).unwrap();
    json!({"sha256":sha256,"bytes":bytes_len,"rows":rows})
}

fn capture_baseline(
    root: &Path,
    capture: Option<&Path>,
    evidence: &Value,
    profile: &Profile,
) -> Baseline {
    let work = fresh_dir(root, "baseline");
    let input = work.join("source.ndjson");
    write_rows(&input, &profile.rows);
    let output = work.join("output");
    let manifest = ingest(
        profile.template.clone(),
        &config(evidence, profile, 64, 64, 1_048_576),
        std::slice::from_ref(&input),
        &output,
    )
    .unwrap();
    assert_expected(&manifest, profile.expected);
    let (populations, pop_stats) = population_receipt(&output);
    assert_eq!(manifest["populations"], pop_stats);
    let semantic = semantic(&manifest);
    let (source_hash, source_bytes) = fingerprint(&input).unwrap();
    if let Some(base) = capture {
        let dir = base.join(profile.name);
        fs::create_dir(&dir).expect("profile capture directory must be new");
        write_json(&dir.join("template.json"), &profile.template);
        write_json(&dir.join("source.json"), &json!(profile.rows));
        write_json(
            &dir.join("config.json"),
            &json!({"normalization_limits":{"max_chunk_rows":64,"max_chunk_bytes":1048576,"max_identities":100},"sort_limits":{"max_run_rows":64,"max_run_bytes":1048576,"max_record_bytes":65536,"merge_fan_in":2},"validation_mode":format!("{:?}",profile.validation_mode),"validation_policy":frozen_policy(profile),"expected":expected_json(profile.expected),"build_evidence":evidence}),
        );
        for (_, file) in POPULATIONS {
            fs::copy(output.join(file), dir.join(file)).unwrap();
        }
        fs::copy(
            output.join("diagnostics.ndjson"),
            dir.join("diagnostics.ndjson"),
        )
        .unwrap();
        write_json(&dir.join("manifest.json"), &manifest);
        write_json(
            &dir.join("baseline-receipt.json"),
            &json!({"profile":profile.name,"source_sha256":source_hash,"source_bytes":source_bytes,"semantic_counts":semantic_counts(&manifest),"populations":pop_stats,"diagnostics":diagnostic_receipt(&output)}),
        );
    }
    Baseline {
        populations,
        semantic,
        manifest,
        source_hash,
        source_bytes,
    }
}

fn load_captured_profile(capture_root: &Path, expected: &Profile) -> (Profile, Baseline, Vec<u8>) {
    let dir = capture_root.join(expected.name);
    let template_bytes = fs::read(dir.join("template.json")).expect("captured template required");
    let source_json = fs::read(dir.join("source.json")).expect("captured source required");
    let config_bytes = fs::read(dir.join("config.json")).expect("captured config required");
    let manifest_bytes = fs::read(dir.join("manifest.json")).expect("captured manifest required");
    let receipt_bytes =
        fs::read(dir.join("baseline-receipt.json")).expect("captured baseline receipt required");
    assert!(
        !config_bytes.is_empty(),
        "captured validation config cannot be empty"
    );
    assert!(
        source_json.len() <= 1 << 20,
        "captured source JSON exceeds 1MiB"
    );
    let template: Value = serde_json::from_slice(&template_bytes).expect("captured template JSON");
    let rows: Vec<Value> =
        serde_json::from_slice(&source_json).expect("captured source JSON array");
    let config_value: Value = serde_json::from_slice(&config_bytes).expect("captured config JSON");
    let manifest: Value = serde_json::from_slice(&manifest_bytes).expect("captured manifest JSON");
    let receipt: Value =
        serde_json::from_slice(&receipt_bytes).expect("captured baseline receipt JSON");
    assert_eq!(receipt["profile"], json!(expected.name));
    assert_eq!(manifest["manifest_version"], json!("c1.ingestion.v1"));
    assert_eq!(
        template, expected.template,
        "captured template differs from the frozen profile"
    );
    assert_eq!(
        canonical_row_multiset(&rows),
        canonical_row_multiset(&expected.rows)
    );
    assert_eq!(rows.len(), expected.expected.source_rows as usize);
    assert!(rows.len() <= 100 && rows.iter().all(Value::is_object));
    assert_eq!(
        config_value["validation_mode"],
        json!(format!("{:?}", expected.validation_mode))
    );
    if !config_value["validation_policy"].is_null() {
        assert_eq!(config_value["validation_policy"], frozen_policy(expected));
    }
    assert_eq!(config_value["expected"], expected_json(expected.expected));
    assert_eq!(
        config_value["normalization_limits"]["max_chunk_rows"],
        json!(64)
    );
    assert_eq!(
        config_value["normalization_limits"]["max_chunk_bytes"],
        json!(1_048_576)
    );
    assert_eq!(
        config_value["normalization_limits"]["max_identities"],
        json!(100)
    );
    assert_eq!(config_value["sort_limits"]["max_run_rows"], json!(64));
    assert_eq!(
        config_value["sort_limits"]["max_run_bytes"],
        json!(1_048_576)
    );
    assert_eq!(
        config_value["sort_limits"]["max_record_bytes"],
        json!(65_536)
    );
    assert_eq!(config_value["sort_limits"]["merge_fan_in"], json!(2));
    assert_eq!(
        receipt["semantic_counts"],
        semantic_counts(&manifest),
        "captured receipt semantic counts must match its frozen manifest"
    );
    assert_expected(&manifest, expected.expected);

    let mut source_ndjson = Vec::new();
    for row in &rows {
        serde_json::to_writer(&mut source_ndjson, row).unwrap();
        source_ndjson.push(b'\n');
    }
    let source_hash = sha256(&source_ndjson);
    let source_bytes = source_ndjson.len() as u64;
    assert_eq!(receipt["source_sha256"], json!(source_hash));
    assert_eq!(receipt["source_bytes"], json!(source_bytes));

    let (populations, stats) = population_receipt(&dir);
    assert_eq!(manifest["populations"], stats);
    assert_eq!(receipt["populations"], stats);
    let diagnostics = diagnostic_receipt(&dir);
    assert_eq!(receipt["diagnostics"], diagnostics);
    let semantic = semantic(&manifest);
    (
        Profile {
            name: expected.name,
            template,
            rows,
            validation_mode: expected.validation_mode,
            expected: expected.expected,
        },
        Baseline {
            populations,
            semantic,
            manifest,
            source_hash,
            source_bytes,
        },
        config_bytes,
    )
}

fn compare_run(
    root: &Path,
    evidence: &Value,
    profile: &Profile,
    rows: &[Value],
    layout: &Value,
    baseline: &Baseline,
    retain: Option<&Path>,
) -> Value {
    let work = fresh_dir(root, "point");
    let input = work.join("source.ndjson");
    write_rows(&input, rows);
    let chunk = layout["max_chunk_rows"].as_u64().unwrap() as usize;
    let run_rows = layout["max_run_rows"].as_u64().unwrap() as usize;
    let run_bytes = layout["max_run_bytes"].as_u64().unwrap() as usize;
    let output = work.join("output");
    let manifest = ingest(
        profile.template.clone(),
        &config(evidence, profile, chunk, run_rows, run_bytes),
        std::slice::from_ref(&input),
        &output,
    )
    .unwrap();
    assert_expected(&manifest, profile.expected);
    let (populations, stats) = population_receipt(&output);
    assert_eq!(
        parsed_records(&populations),
        parsed_records(&baseline.populations),
        "ordered parsed records differ from baseline at {layout}"
    );
    assert_eq!(
        populations, baseline.populations,
        "canonical NDJSON bytes differ from this profile's actual-ingest baseline at {layout}"
    );
    assert_eq!(
        semantic(&manifest),
        baseline.semantic,
        "semantic accounting differs from baseline at {layout}"
    );
    assert_eq!(manifest["populations"], stats);
    let diagnostics = diagnostic_receipt(&output);
    let input_fingerprint = fingerprint(&input).unwrap();
    let receipt = json!({"profile":profile.name,"layout":layout,"source_sha256":input_fingerprint.0,"source_bytes":input_fingerprint.1,"semantic_counts":semantic_counts(&manifest),"populations":stats,"diagnostics":diagnostics,"sort_report":manifest["sort"],"exact_bytes_equal_baseline":true});
    if let Some(dir) = retain {
        write_json(
            &dir.join(format!(
                "manifest-{}.json",
                layout["point_id"].as_str().unwrap()
            )),
            &manifest,
        );
        write_json(
            &dir.join(format!(
                "receipt-{}.json",
                layout["point_id"].as_str().unwrap()
            )),
            &receipt,
        );
    }
    receipt
}

fn raw_orders(rows: &[Value]) -> Vec<(&'static str, Vec<Value>)> {
    let forward = rows.to_vec();
    let mut reversed = rows.to_vec();
    reversed.reverse();
    vec![("forward", forward), ("reversed", reversed)]
}

fn settings() -> Vec<(usize, usize, usize)> {
    [1usize, 2, 64]
        .into_iter()
        .flat_map(|chunk| {
            [1usize, 2, 64].into_iter().flat_map(move |rows| {
                [4096usize, 1_048_576]
                    .into_iter()
                    .map(move |bytes| (chunk, rows, bytes))
            })
        })
        .collect()
}

fn check_spill_controls(manifest: &Value, settings: (usize, usize, usize), expected: Expected) {
    let report = &manifest["sort"];
    assert_eq!(report["rows"], json!(expected.accepted));
    if settings.1 == 1 && settings.2 == 1_048_576 {
        assert!(
            report["runs"].as_u64().unwrap() > 1,
            "row-triggered spill missing: {report}"
        );
        assert!(
            report["merge_passes"].as_u64().unwrap() > 0,
            "row-triggered merge missing: {report}"
        );
    }
    if settings.1 == 64 && settings.2 == 4096 {
        assert!(
            report["runs"].as_u64().unwrap() > 1,
            "byte-only spill missing with max_run_rows above event count: {report}"
        );
        assert!(
            report["merge_passes"].as_u64().unwrap() > 0,
            "byte-only merge missing: {report}"
        );
    }
    if settings.1 == 64 && settings.2 == 1_048_576 {
        assert_eq!(
            report["runs"],
            json!(1),
            "single-run control must be exactly one run: {report}"
        );
        assert_eq!(
            report["merge_passes"],
            json!(0),
            "single-run control must not merge: {report}"
        );
    }
}

#[test]
fn self_contained_three_profile_cartesian_matrix_has_exact_population_bytes() {
    let root = fresh_dir(&std::env::temp_dir(), "kairos-c01-self");
    let capture = std::env::var_os("KAIROS_C01_CAPTURE_DIR").map(PathBuf::from);
    if let Some(path) = &capture {
        fs::create_dir(path).expect("capture directory must be new");
    }
    let evidence = build_evidence();
    let mut total_points = 0usize;
    let mut top_profiles = Vec::new();
    for profile in profiles() {
        assert!(PROFILES.contains(&profile.name));
        let baseline = capture_baseline(&root, capture.as_deref(), &evidence, &profile);
        let profile_capture = capture.as_ref().map(|p| p.join(profile.name));
        let mut receipts = Vec::new();
        for (order, rows) in raw_orders(&profile.rows) {
            for (chunk, run_rows, run_bytes) in settings() {
                let point_id = format!("{}-{}-{}-{}", order, chunk, run_rows, run_bytes);
                let point = json!({"point_id":point_id,"row_order":order,"max_chunk_rows":chunk,"max_run_rows":run_rows,"max_run_bytes":run_bytes,"merge_fan_in":2});
                let receipt = compare_run(
                    &root,
                    &evidence,
                    &profile,
                    &rows,
                    &point,
                    &baseline,
                    profile_capture.as_deref(),
                );
                check_spill_controls(
                    &json!({"sort":receipt["sort_report"]}),
                    (chunk, run_rows, run_bytes),
                    profile.expected,
                );
                receipts.push(receipt);
                total_points += 1;
            }
        }
        assert_eq!(
            receipts.len(),
            36,
            "each profile must cover both orders and all 18 settings"
        );
        top_profiles.push(json!({"profile":profile.name,"expected":expected_json(profile.expected),"baseline_source_sha256":baseline.source_hash,"baseline_source_bytes":baseline.source_bytes,"baseline_semantic":semantic_counts(&baseline.manifest),"points":receipts}));
    }
    assert_eq!(
        total_points, 108,
        "C-01 matrix must be non-vacuous and complete"
    );
    if let Some(capture) = capture {
        write_json(
            &capture.join("matrix-receipt.json"),
            &json!({"schema_version":"c01.rust-matrix.v1","profiles":top_profiles,"points":total_points}),
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn schema_valid_raw_marker_mutation_is_detected_by_exact_population_oracle() {
    let root = fresh_dir(&std::env::temp_dir(), "kairos-c01-mutation");
    let evidence = build_evidence();
    let profile = profiles().remove(0);
    let baseline = capture_baseline(&root, None, &evidence, &profile);
    let mut changed = profile.rows.clone();
    changed[0]["qualification_marker"] = json!("mutated-but-schema-valid");
    let work = fresh_dir(&root, "mutant");
    let input = work.join("source.ndjson");
    write_rows(&input, &changed);
    let output = work.join("output");
    let manifest = ingest(
        profile.template.clone(),
        &config(&evidence, &profile, 64, 64, 1_048_576),
        std::slice::from_ref(&input),
        &output,
    )
    .unwrap();
    assert_expected(&manifest, profile.expected);
    let (mutant_populations, _) = population_receipt(&output);
    assert_ne!(
        mutant_populations["events"], baseline.populations["events"],
        "mutated raw source marker must alter canonical event bytes"
    );
    assert_ne!(
        manifest["populations"]["events"]["sha256"],
        baseline.manifest["populations"]["events"]["sha256"],
        "mutated source marker must change event SHA-256"
    );
    assert_eq!(manifest["validation"]["valid_events"], json!(6));
    assert_eq!(manifest["validation"]["quarantined_events"], json!(0));
    assert_eq!(
        semantic_counts(&manifest),
        semantic_counts(&baseline.manifest)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires exact physical transport source index and result directory"]
fn actual_transport_matrix() {
    actual_transport_matrix_impl();
}

fn actual_transport_matrix_impl() {
    const MAX_SOURCE_BYTES: u64 = 1 << 20;
    const MAX_SOURCE_ROWS: usize = 100;
    const FORMATS: [&str; 3] = ["ipc_file", "ipc_stream", "parquet"];
    let index_path = PathBuf::from(
        std::env::var_os("KAIROS_C01_TRANSPORT_INDEX").expect("transport index required"),
    );
    let result = PathBuf::from(
        std::env::var_os("KAIROS_C01_RESULT_DIR").expect("transport result directory required"),
    );
    fs::create_dir(&result).expect("transport result directory must be fresh");
    let index_bytes = fs::read(&index_path).expect("read transport index");
    assert!(
        !index_bytes.is_empty() && index_bytes.len() <= MAX_SOURCE_BYTES as usize,
        "transport index must be nonempty and <=1MiB"
    );
    let index: Value = serde_json::from_slice(&index_bytes).expect("valid transport index JSON");
    assert_eq!(index["schema_version"], json!("c01.transport-index.v1"));
    let capture_root = PathBuf::from(index["capture_dir"].as_str().expect("capture_dir required"));
    let capture_root = fs::canonicalize(&capture_root).expect("capture directory exists");
    let index_parent = fs::canonicalize(index_path.parent().expect("index parent required"))
        .expect("transport index parent exists");
    let bundle_root_rel = PathBuf::from(
        index["bundle_root"]
            .as_str()
            .expect("explicit bundle_root allowlist required"),
    );
    assert!(
        !bundle_root_rel.is_absolute()
            && bundle_root_rel
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
        "bundle_root must be a relative non-traversing path"
    );
    let bundle_root = fs::canonicalize(index_parent.join(bundle_root_rel))
        .expect("allowlisted bundle_root exists");
    assert!(
        bundle_root.starts_with(&index_parent),
        "bundle_root escapes index parent"
    );
    let mut expected_index = BTreeSet::new();
    for profile in PROFILES {
        for batch in [1u64, 2] {
            for rowgroup in [1u64, 3] {
                for order in ["forward", "reverse"] {
                    for format in FORMATS {
                        for writer in [1u64, 2, 3] {
                            expected_index.insert(bundle_key(
                                profile, batch, rowgroup, order, format, writer,
                            ));
                        }
                    }
                }
            }
        }
    }
    let bundles = index["bundles"].as_array().expect("bundles array required");
    assert_eq!(
        bundles.len(),
        216,
        "transport index must contain all 216 source bundles"
    );
    let mut seen = BTreeSet::new();
    let expected_profiles = profiles();
    let mut profile_defs = Vec::with_capacity(expected_profiles.len());
    let mut baselines = BTreeMap::new();
    let mut frozen_configs = BTreeMap::new();
    for expected in &expected_profiles {
        let (loaded, baseline, frozen_config) = load_captured_profile(&capture_root, expected);
        baselines.insert(loaded.name, baseline);
        frozen_configs.insert(loaded.name, frozen_config);
        profile_defs.push(loaded);
    }
    let by_name: BTreeMap<&str, Profile> =
        profile_defs.iter().map(|p| (p.name, p.clone())).collect();
    let evidence = build_evidence();
    let work = fresh_dir(&result, "work");
    let executions = result.join("executions");
    fs::create_dir(&executions).unwrap();
    let ledger = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(result.join("matrix-points.ndjson"))
        .unwrap();
    let mut ledger = std::io::BufWriter::new(ledger);
    let mut cache: Vec<CachedExecution> = Vec::new();
    let mut representative_classes: BTreeSet<(String, Vec<u8>)> = BTreeSet::new();
    let mut representatives = Vec::new();
    let mut actual_executions = 0usize;
    let mut point_count = 0usize;
    let mut checked_alias_sources = 0usize;
    let mut distinct_inputs = BTreeSet::new();
    for (bundle_index, bundle) in bundles.iter().enumerate() {
        let profile_name = bundle["profile"].as_str().expect("profile required");
        let profile = by_name.get(profile_name).expect("unsupported profile");
        let batch = bundle["batch_rows"].as_u64().expect("batch_rows required");
        let rowgroup = bundle["row_group_rows"]
            .as_u64()
            .expect("row_group_rows required");
        let order = bundle["row_order"].as_str().expect("row_order required");
        let format = bundle["format"].as_str().expect("format required");
        let writer = bundle["writer_limit"]
            .as_u64()
            .expect("writer_limit required");
        let key = bundle_key(profile_name, batch, rowgroup, order, format, writer);
        assert!(
            seen.insert(key.clone()),
            "duplicate physical source bundle: {key}"
        );
        let path_str = bundle["path"].as_str().expect("bundle path required");
        let relative_source = PathBuf::from(path_str);
        assert!(
            !relative_source.is_absolute()
                && relative_source
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "bundle path must be relative and non-traversing"
        );
        let candidate = bundle_root.join(relative_source);
        let canonical = fs::canonicalize(&candidate).expect("transport source array exists");
        assert!(
            canonical.starts_with(&bundle_root),
            "transport path escapes allowlisted bundle_root"
        );
        let metadata = fs::metadata(&canonical).unwrap();
        assert!(
            metadata.is_file() && metadata.len() <= MAX_SOURCE_BYTES,
            "transport JSON exceeds 1MiB or is not a regular file"
        );
        let raw = fs::read(&canonical).unwrap();
        assert!(
            raw.len() as u64 <= MAX_SOURCE_BYTES,
            "transport JSON grew beyond bound during read"
        );
        let actual_sha = sha256(&raw);
        assert_eq!(
            bundle["sha256"].as_str(),
            Some(actual_sha.as_str()),
            "transport source file hash mismatch"
        );
        let rows: Vec<Value> =
            serde_json::from_slice(&raw).expect("transport source must be a JSON array");
        assert!(
            !rows.is_empty() && rows.len() <= MAX_SOURCE_ROWS,
            "transport source row count is out of bounds"
        );
        assert_eq!(
            bundle["rows"],
            json!(rows.len()),
            "index row count mismatch"
        );
        assert_eq!(
            rows.len(),
            profile.expected.source_rows as usize,
            "source row count differs from declared profile"
        );
        let profile_rows = canonical_row_multiset(&rows);
        assert_eq!(
            profile_rows,
            canonical_row_multiset(&profile.rows),
            "source row multiset differs from baseline profile"
        );
        assert!(
            matches!(order, "forward" | "reverse"),
            "unsupported physical row order"
        );
        let mut ndjson = Vec::new();
        for row in &rows {
            assert!(row.is_object(), "source array rows must be objects");
            serde_json::to_writer(&mut ndjson, row).unwrap();
            ndjson.push(b'\n');
        }
        assert!(
            ndjson.len() <= MAX_SOURCE_BYTES as usize,
            "serialized transport rows exceed 1MiB"
        );
        let ndjson_sha = sha256(&ndjson);
        distinct_inputs.insert((profile_name.to_owned(), ndjson_sha.clone()));
        let template_key = fs::read(capture_root.join(profile_name).join("template.json"))
            .expect("captured template bytes");
        let frozen_config = frozen_configs[profile_name].clone();
        let policy_key = serde_json::to_vec(&frozen_policy(profile))
            .expect("serialize frozen validation policy");
        let evidence_key = serde_json::to_vec(&evidence).unwrap();
        for (chunk, run_rows, run_bytes) in settings() {
            let setting = (chunk, run_rows, run_bytes);
            let cached = cache.iter().find(|entry| {
                entry.profile == profile_name
                    && entry.ordered_ndjson == ndjson
                    && entry.template == template_key
                    && entry.frozen_config == frozen_config
                    && entry.policy == policy_key
                    && entry.evidence == evidence_key
                    && entry.limits == setting
            });
            let (execution_id, receipt, aliased) = if let Some(entry) = cached {
                (entry.execution_id.clone(), entry.receipt.clone(), true)
            } else {
                let id = format!("exec-{actual_executions:04}");
                let source_path = work.join(format!("source-{bundle_index:03}.ndjson"));
                fs::write(&source_path, &ndjson).unwrap();
                let output = work.join(format!("output-{actual_executions:04}"));
                let manifest = ingest(
                    profile.template.clone(),
                    &config(&evidence, profile, chunk, run_rows, run_bytes),
                    std::slice::from_ref(&source_path),
                    &output,
                )
                .unwrap();
                assert_expected(&manifest, profile.expected);
                let baseline = &baselines[profile_name];
                let (actual_pops, actual_stats) = population_receipt(&output);
                assert_eq!(
                    actual_pops, baseline.populations,
                    "actual transport population bytes differ from profile baseline"
                );
                assert_eq!(
                    parsed_records(&actual_pops),
                    parsed_records(&baseline.populations),
                    "ordered parsed transport records differ from baseline"
                );
                assert_eq!(
                    semantic(&manifest),
                    baseline.semantic,
                    "actual transport semantic accounting differs from baseline"
                );
                assert_eq!(
                    manifest["inputs"][0]["sha256"],
                    json!(ndjson_sha),
                    "manifest source SHA must be the exact alias NDJSON hash"
                );
                assert_eq!(manifest["inputs"][0]["bytes"], json!(ndjson.len()));
                assert_eq!(manifest["populations"], actual_stats);
                let receipt = json!({"execution_id":id,"profile":profile_name,"source_ndjson_sha256":ndjson_sha,"source_ndjson_bytes":ndjson.len(),"semantic_counts":semantic_counts(&manifest),"populations":actual_stats,"diagnostics":diagnostic_receipt(&output),"sort_report":manifest["sort"],"exact_bytes_equal_baseline":true});
                write_json(&executions.join(format!("manifest-{id}.json")), &manifest);
                write_json(&executions.join(format!("receipt-{id}.json")), &receipt);
                if representative_classes.insert((profile_name.to_owned(), ndjson.clone())) {
                    let representative = result.join("representatives").join(&id);
                    fs::create_dir_all(&representative).unwrap();
                    for (_, filename) in POPULATIONS {
                        fs::copy(output.join(filename), representative.join(filename))
                            .unwrap_or_else(|error| panic!("retain {filename}: {error}"));
                    }
                    fs::copy(
                        output.join("diagnostics.ndjson"),
                        representative.join("diagnostics.ndjson"),
                    )
                    .unwrap_or_else(|error| panic!("retain diagnostics: {error}"));
                    let provenance = json!({
                        "execution_id":id,
                        "profile":profile_name,
                        "source_ndjson_sha256":ndjson_sha,
                        "source_ndjson_bytes":ndjson.len(),
                        "source_array_sha256":actual_sha,
                        "transport_bundle_path":bundle["path"],
                        "manifest_path":format!("executions/manifest-{id}.json"),
                        "receipt_path":format!("executions/receipt-{id}.json"),
                        "representative_directory":format!("representatives/{id}"),
                        "populations":actual_stats,
                        "diagnostics":diagnostic_receipt(&output)
                    });
                    write_json(&representative.join("representative.json"), &provenance);
                    representatives.push(provenance);
                }
                fs::remove_dir_all(&output).unwrap();
                fs::remove_file(source_path).unwrap();
                actual_executions += 1;
                cache.push(CachedExecution {
                    profile: profile_name.into(),
                    ordered_ndjson: ndjson.clone(),
                    template: template_key.clone(),
                    frozen_config: frozen_config.clone(),
                    policy: policy_key.clone(),
                    evidence: evidence_key.clone(),
                    limits: setting,
                    execution_id: id.clone(),
                    receipt: receipt.clone(),
                });
                (id, receipt, false)
            };
            assert_eq!(
                receipt["source_ndjson_sha256"],
                json!(ndjson_sha),
                "alias source digest must equal actual execution input digest"
            );
            assert_eq!(
                receipt["source_ndjson_bytes"],
                json!(ndjson.len()),
                "alias source byte length must equal actual execution input length"
            );
            if aliased {
                checked_alias_sources += 1;
            }
            let point = json!({"matrix_point":point_count,"bundle_index":bundle_index,"profile":profile_name,"batch_rows":batch,"row_group_rows":rowgroup,"row_order":order,"format":format,"writer_limit":writer,"max_chunk_rows":chunk,"max_run_rows":run_rows,"max_run_bytes":run_bytes,"merge_fan_in":2,"source_array_sha256":actual_sha,"source_ndjson_sha256":ndjson_sha,"execution_id":execution_id,"execution_kind":if aliased{"equivalence_alias"}else{"actual"},"canonical_populations":receipt["populations"],"sort_report":receipt["sort_report"]});
            serde_json::to_writer(&mut ledger, &point).unwrap();
            ledger.write_all(b"\n").unwrap();
            point_count += 1;
        }
    }
    ledger.flush().unwrap();
    assert_eq!(
        seen, expected_index,
        "transport index missing or has unsupported physical points"
    );
    assert_eq!(
        point_count, 3888,
        "must ledger every physical source × Rust configuration point"
    );
    let all_alias_sources_checked = checked_alias_sources == point_count - actual_executions;
    assert!(all_alias_sources_checked);
    write_json(
        &result.join("representative-index.json"),
        &json!({"schema_version":"c01.transport-representatives.v1","representatives":representatives}),
    );
    write_json(
        &result.join("actual-matrix-summary.json"),
        &json!({"schema_version":"c01.transport-rust-matrix.v1","index_sha256":sha256(&index_bytes),"transport_bundles":bundles.len(),"matrix_points":point_count,"actual_executions":actual_executions,"aliases":point_count-actual_executions,"checked_alias_sources":checked_alias_sources,"distinct_profile_input_fingerprints":distinct_inputs.len(),"cache_key":"exact profile + exact ordered NDJSON bytes + frozen template bytes + frozen config bytes + validation policy bytes + chunk/run-row/run-byte limits + same code/toolchain evidence","profiles":PROFILES,"all_alias_source_sha_and_byte_length_match":all_alias_sources_checked}),
    );
    fs::remove_dir_all(work).unwrap();
}

fn bundle_key(
    profile: &str,
    batch: u64,
    rowgroup: u64,
    order: &str,
    format: &str,
    writer: u64,
) -> String {
    format!("{profile}|{batch}|{rowgroup}|{order}|{format}|{writer}")
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn canonical_rows(rows: &[Value]) -> Vec<Vec<u8>> {
    rows.iter()
        .map(|row| serde_json::to_vec(row).unwrap())
        .collect()
}

fn canonical_row_multiset(rows: &[Value]) -> BTreeMap<Vec<u8>, usize> {
    let mut multiset = BTreeMap::new();
    for row in canonical_rows(rows) {
        *multiset.entry(row).or_insert(0) += 1;
    }
    multiset
}
