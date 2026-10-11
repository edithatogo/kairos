//! Bounded command adapter for the hidden experimental C5 JSON facade.
use crate::c5_workers;
use kairo_ecs_calibration::experimental_c5 as c5;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

const MAX_STUDY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_ATTEMPTS_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn run(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("verify") => verify_command(args),
        Some("replay") => replay_command(args),
        Some("calibrate") => calibrate_command(args),
        Some("compare-runs") => compare_command(args),
        _ => Err("invalid C5 command".into()),
    }
}

fn verify_command(args: &[String]) -> Result<String, String> {
    if args.get(1).map(String::as_str) != Some("trace") {
        return Err(
            "usage: verify trace --study <json> [--scenario <path> --seed-manifest <path>]".into(),
        );
    }
    let flags = parse_flags(
        &args[2..],
        &["--study", "--scenario", "--seed-manifest", "--output"],
        &[],
    )?;
    let study_path = resolve_study_path(&flags)?;
    let study = read_bounded(&study_path, MAX_STUDY_BYTES)?;
    validate_study_json(&study)?;
    let manifest = check_optional_manifests(&flags, &study, &study_path)?;
    let native = c5::verify_trace(&study).map_err(|error| error.to_string())?;
    let native_value = parse_json(&native, "trace verification result")?;
    let provenance = provenance(&study, manifest.as_ref(), "verify-trace")?;
    let result = json!({
        "schema_version": "kairoecs.c5-trace-verification.v1",
        "study_sha256": sha256(study.as_bytes()),
        "manifest_binding_sha256": manifest.as_ref().map(|m| m.binding_hash.clone()),
        "provenance": provenance,
        "result": native_value
    });
    if let Some(output) = flags.get("--output") {
        write_file_no_replace(Path::new(output), &canonical_json(&result)?)?;
    }
    canonical_json(&result)
}

fn replay_command(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(
        &args[1..],
        &[
            "--study",
            "--mode",
            "--scenario",
            "--seed-manifest",
            "--output",
        ],
        &[],
    )?;
    let mode = required_string(&flags, "--mode")?;
    if mode != "shadow" && mode != "free" {
        return Err("--mode must be shadow or free".into());
    }
    let study_path = resolve_study_path(&flags)?;
    let study = read_bounded(&study_path, MAX_STUDY_BYTES)?;
    validate_study_json(&study)?;
    let manifest = check_optional_manifests(&flags, &study, &study_path)?;
    let native = c5::replay(&study, mode).map_err(|error| error.to_string())?;
    let native_value = parse_json(&native, "replay result")?;
    let provenance = provenance(&study, manifest.as_ref(), mode)?;
    let result = json!({
        "schema_version": "kairoecs.c5-replay-result.v1",
        "mode": mode,
        "study_sha256": sha256(study.as_bytes()),
        "manifest_binding_sha256": manifest.as_ref().map(|m| m.binding_hash.clone()),
        "adapter": "synthetic_flow_probe.v1",
        "provenance": provenance,
        "result": native_value
    });
    if let Some(output) = flags.get("--output") {
        let output = PathBuf::from(output);
        create_new_directory(&output)?;
        write_file_no_replace(&output.join("replay.json"), &canonical_json(&result)?)?;
        write_file_no_replace(
            &output.join("provenance.json"),
            &canonical_json(&provenance)?,
        )?;
    }
    canonical_json(&result)
}

#[derive(Clone)]
struct ManifestBinding {
    scenario_bytes: Option<Vec<u8>>,
    seed_bytes: Option<Vec<u8>>,
    binding_hash: String,
}

fn calibrate_command(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(
        &args[1..],
        &[
            "--study",
            "--scenario",
            "--seed-manifest",
            "--output",
            "--resume",
        ],
        &["--workers", "--stop-after"],
    )?;
    let study_path = resolve_study_path(&flags)?;
    let study = read_bounded(&study_path, MAX_STUDY_BYTES)?;
    validate_study_json(&study)?;
    let manifest = check_optional_manifests(&flags, &study, &study_path)?;
    let trace = c5::verify_trace(&study).map_err(|error| error.to_string())?;
    let _trace_value = parse_json(&trace, "trace verification result")?;
    let workers = parse_bounded_usize(flags.get("--workers"), "--workers", 1, 64, 1)?;
    let stop_after = parse_optional_usize(flags.get("--stop-after"), "--stop-after", usize::MAX)?;
    let output = required_path(&flags, "--output")?;
    if let Some(resume) = flags.get("--resume") {
        validate_path_components(&output, false)?;
        let resume = checked_existing_dir(Path::new(resume))?;
        let output_abs = fs::canonicalize(&output).map_err(|e| e.to_string())?;
        if resume != output_abs {
            return Err("--resume must name the existing --output run directory".into());
        }
    } else {
        create_new_directory(&output)?;
    }

    let provenance = provenance(&study, manifest.as_ref(), "calibrate")?;
    let provenance_bytes = canonical_json(&provenance)?;
    let binding = sha256(provenance_bytes.as_bytes());
    let provenance_path = output.join("provenance.json");
    if provenance_path.exists() {
        let existing = read_bounded(&provenance_path, MAX_MANIFEST_BYTES)?;
        if existing != provenance_bytes {
            return Err("existing run provenance does not match this study/build".into());
        }
    } else {
        write_file_no_replace(&provenance_path, &provenance_bytes)?;
    }

    let plan_text = c5::plan(&study).map_err(|error| error.to_string())?;
    let plan = parse_json(&plan_text, "study plan")?;
    let initial_tasks = array_field(&plan, "tasks")?.to_vec();
    let study_hash = string_field(&plan, "study_hash")?.to_owned();
    let initial_fresh = count_missing_shards(&initial_tasks, &output.join("initial-shards"))?;
    let initial = c5_workers::execute(
        &initial_tasks,
        workers,
        &output.join("initial-shards"),
        &binding,
        stop_after,
        |task| {
            let task_json = serde_json::to_string(task).map_err(|e| e.to_string())?;
            let result = c5::evaluate(&study, &task_json).map_err(|e| e.to_string())?;
            parse_json(&result, "evaluation result")
        },
    )
    .map_err(|error| format!("initial task execution: {error}"))?;
    if !initial.complete {
        return canonical_json(&json!({
            "schema_version":"kairoecs.c5-calibration-progress.v1",
            "status":"incomplete",
            "phase":"initial",
            "study_hash":study_hash,
            "completed_tasks":initial.results.len(),
            "total_tasks":initial_tasks.len(),
            "resume":"calibrate --study <same-study> --output <same-dir> --resume <same-dir>"
        }));
    }
    let mut attempts: Vec<Value> = initial
        .results
        .into_iter()
        .map(|item| item.result)
        .collect();
    let initial_summary_text = c5::summarize(
        &study,
        &serde_json::to_string(&attempts).map_err(|e| e.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let initial_summary = parse_json(&initial_summary_text, "initial study summary")?;
    write_identical_or_new(
        &output.join("summary-initial.json"),
        &canonical_json(&initial_summary)?,
    )?;
    let final_tasks = array_field(&initial_summary, "next_tasks")?.to_vec();
    if !final_tasks.is_empty() {
        let remaining_stop_after = stop_after.map(|limit| limit.saturating_sub(initial_fresh));
        let final_execution = c5_workers::execute(
            &final_tasks,
            workers,
            &output.join("final-shards"),
            &binding,
            remaining_stop_after,
            |task| {
                let task_json = serde_json::to_string(task).map_err(|e| e.to_string())?;
                let result = c5::evaluate(&study, &task_json).map_err(|e| e.to_string())?;
                parse_json(&result, "evaluation result")
            },
        )
        .map_err(|error| format!("final task execution: {error}"))?;
        if !final_execution.complete {
            return canonical_json(&json!({
                "schema_version":"kairoecs.c5-calibration-progress.v1",
                "status":"incomplete",
                "phase":"final_test",
                "study_hash":study_hash,
                "completed_tasks":final_execution.results.len(),
                "total_tasks":final_tasks.len(),
                "remaining_fresh_task_budget":remaining_stop_after,
                "resume":"calibrate --study <same-study> --output <same-dir> --resume <same-dir>"
            }));
        }
        attempts.extend(final_execution.results.into_iter().map(|item| item.result));
    }
    let summary_text = c5::summarize(
        &study,
        &serde_json::to_string(&attempts).map_err(|e| e.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let summary = parse_json(&summary_text, "study summary")?;
    write_identical_or_new(&output.join("summary.json"), &canonical_json(&summary)?)?;
    let receipt = json!({
        "schema_version":"kairoecs.c5-calibration-run.v1",
        "status": if summary.get("complete").and_then(Value::as_bool)==Some(true) {"complete"} else {"incomplete"},
        "study_hash":study_hash,
        "summary_sha256":sha256(canonical_json(&summary)?.as_bytes()),
        "provenance_sha256":binding,
        "summary":summary
    });
    write_identical_or_new(&output.join("run.json"), &canonical_json(&receipt)?)?;
    canonical_json(&receipt)
}

fn compare_command(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(&args[1..], &["--left", "--right", "--output"], &[])?;
    let left = checked_existing_dir(&required_path(&flags, "--left")?)?;
    let right = checked_existing_dir(&required_path(&flags, "--right")?)?;
    let left_prov = read_json_file(&left.join("provenance.json"), MAX_MANIFEST_BYTES)?;
    let right_prov = read_json_file(&right.join("provenance.json"), MAX_MANIFEST_BYTES)?;
    for field in [
        "schema_version",
        "study_sha256",
        "scenario_sha256",
        "seed_manifest_sha256",
        "manifest_binding_sha256",
        "executable_sha256",
        "source_commit",
        "source_dirty",
        "cargo_lock_sha256",
        "toolchain",
        "features",
        "adapter",
        "mode",
    ] {
        if left_prov.get(field) != right_prov.get(field) {
            return Err(format!("incompatible run provenance field `{field}`"));
        }
    }
    let left_summary = read_json_file(&left.join("summary.json"), MAX_ATTEMPTS_BYTES)?;
    let right_summary = read_json_file(&right.join("summary.json"), MAX_ATTEMPTS_BYTES)?;
    if left_summary.get("complete").and_then(Value::as_bool) != Some(true)
        || right_summary.get("complete").and_then(Value::as_bool) != Some(true)
    {
        return Err("compare-runs requires two complete calibration summaries".into());
    }
    validate_complete_bundle(&left, &left_prov, &left_summary)?;
    validate_complete_bundle(&right, &right_prov, &right_summary)?;
    let report = json!({
        "schema_version":"kairoecs.c5-run-comparison.v1",
        "compatible":true,
        "study_sha256":left_prov["study_sha256"],
        "left_summary_sha256":sha256(canonical_json(&left_summary)?.as_bytes()),
        "right_summary_sha256":sha256(canonical_json(&right_summary)?.as_bytes()),
        "left":left_summary,
        "right":right_summary
    });
    let encoded = canonical_json(&report)?;
    if let Some(path) = flags.get("--output") {
        write_file_no_replace(Path::new(path), &encoded)?;
    }
    Ok(encoded)
}

fn validate_complete_bundle(dir: &Path, provenance: &Value, summary: &Value) -> Result<(), String> {
    if provenance.get("schema_version").and_then(Value::as_str) != Some("kairoecs.c5-provenance.v1")
    {
        return Err(format!("{} has unsupported C5 provenance", dir.display()));
    }
    let run = read_json_file(&dir.join("run.json"), MAX_ATTEMPTS_BYTES)?;
    let summary_hash = sha256(canonical_json(summary)?.as_bytes());
    let provenance_hash = sha256(canonical_json(provenance)?.as_bytes());
    if run.get("status").and_then(Value::as_str) != Some("complete")
        || run.get("schema_version").and_then(Value::as_str)
            != Some("kairoecs.c5-calibration-run.v1")
        || run.get("summary_sha256").and_then(Value::as_str) != Some(summary_hash.as_str())
        || run.get("provenance_sha256").and_then(Value::as_str) != Some(provenance_hash.as_str())
        || run.get("summary") != Some(summary)
        || run.get("study_hash").and_then(Value::as_str)
            != summary.get("study_hash").and_then(Value::as_str)
    {
        return Err(format!(
            "{} run receipt does not match its artifacts",
            dir.display()
        ));
    }
    Ok(())
}

fn check_optional_manifests(
    flags: &BTreeMap<String, String>,
    study: &str,
    study_path: &Path,
) -> Result<Option<ManifestBinding>, String> {
    let scenario_path = flags.get("--scenario");
    let seed_path = flags.get("--seed-manifest");
    if scenario_path.is_some() != seed_path.is_some() {
        return Err("--scenario and --seed-manifest must be supplied together".into());
    }
    let Some(scenario_path) = scenario_path else {
        return Ok(None);
    };
    let scenario_path = PathBuf::from(scenario_path);
    let seed_path = PathBuf::from(seed_path.expect("paired manifest option"));
    let scenario_bytes = read_bounded(&scenario_path, MAX_MANIFEST_BYTES)?.into_bytes();
    let seed_bytes = read_bounded(&seed_path, MAX_MANIFEST_BYTES)?.into_bytes();
    let scenario = crate::scenario::load_scenario(&scenario_path).map_err(|e| e.to_string())?;
    let seed = crate::scenario::load_seed_manifest(&seed_path).map_err(|e| e.to_string())?;
    crate::scenario::validate_scenario_and_seed(&scenario, &seed).map_err(|e| e.to_string())?;
    let study_value = parse_json(study, "study")?;
    let root_seed = study_value
        .get("root_seed")
        .and_then(Value::as_u64)
        .ok_or("study root_seed must be UInt64")?;
    let replications = study_value
        .pointer("/budget/replications")
        .and_then(Value::as_u64)
        .ok_or("study budget.replications must be UInt64")?;
    if scenario.base_seed != root_seed || seed.base_seed != root_seed {
        return Err("legacy manifest base_seed does not match study root_seed".into());
    }
    if u64::from(scenario.replications) != replications {
        return Err("legacy scenario replications do not match study budget.replications".into());
    }
    if let Some(reference) = crate::scenario::calibration_study_reference(&scenario_path)? {
        let resolved = resolve_manifest_reference(&scenario_path, &reference)?;
        if fs::canonicalize(&resolved).map_err(|e| e.to_string())?
            != fs::canonicalize(study_path).map_err(|e| e.to_string())?
        {
            return Err("scenario calibration_study conflicts with --study".into());
        }
    }
    let binding_hash = sha256(&[scenario_bytes.as_slice(), seed_bytes.as_slice()].concat());
    Ok(Some(ManifestBinding {
        scenario_bytes: Some(scenario_bytes),
        seed_bytes: Some(seed_bytes),
        binding_hash,
    }))
}

fn provenance(
    study: &str,
    manifests: Option<&ManifestBinding>,
    mode: &str,
) -> Result<Value, String> {
    let executable = std::env::current_exe().map_err(|e| format!("current executable: {e}"))?;
    let executable_hash = sha256_file(&executable)?;
    Ok(json!({
        "schema_version":"kairoecs.c5-provenance.v1",
        "study_sha256":sha256(study.as_bytes()),
        "scenario_sha256":manifests.and_then(|m|m.scenario_bytes.as_ref()).map(|b|sha256(b)),
        "seed_manifest_sha256":manifests.and_then(|m|m.seed_bytes.as_ref()).map(|b|sha256(b)),
        "manifest_binding_sha256":manifests.map(|m|m.binding_hash.clone()),
        "executable_sha256":executable_hash,
        "package_version":env!("CARGO_PKG_VERSION"),
        "source_commit":option_env!("KAIROS_C5_SOURCE_COMMIT").unwrap_or("unavailable"),
        "cargo_lock_sha256":sha256(include_bytes!("../../../Cargo.lock")),
        "toolchain":option_env!("KAIROS_C5_TOOLCHAIN").unwrap_or("unavailable"),
        "source_dirty":option_env!("KAIROS_C5_SOURCE_DIRTY").unwrap_or("unavailable"),
        "features":["flow"],
        "adapter":"synthetic_flow_probe.v1",
        "mode":mode
    }))
}

fn validate_study_json(study: &str) -> Result<(), String> {
    let value = parse_json(study, "study")?;
    if value.get("schema_version").and_then(Value::as_str) != Some("kairoecs.calibration.study.v1")
    {
        return Err("unsupported or missing study schema_version".into());
    }
    if value
        .get("study_id")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("study_id must be a nonempty string".into());
    }
    Ok(())
}

fn parse_flags(
    args: &[String],
    paths: &[&str],
    scalars: &[&str],
) -> Result<BTreeMap<String, String>, String> {
    let mut flags = BTreeMap::new();
    let allowed: Vec<&str> = paths.iter().chain(scalars.iter()).copied().collect();
    let mut i = 0;
    while i < args.len() {
        let key = args[i].as_str();
        if !allowed.contains(&key) {
            return Err(format!("unknown argument `{key}`"));
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for `{key}`"))?;
        if value.starts_with("--") {
            return Err(format!("missing value for `{key}`"));
        }
        if flags.insert(key.to_owned(), value.clone()).is_some() {
            return Err(format!("duplicate argument `{key}`"));
        }
        i += 2;
    }
    Ok(flags)
}

fn required_path(flags: &BTreeMap<String, String>, key: &str) -> Result<PathBuf, String> {
    flags
        .get(key)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required flag `{key}`"))
}
fn resolve_study_path(flags: &BTreeMap<String, String>) -> Result<PathBuf, String> {
    if let Some(study) = flags.get("--study") {
        return Ok(PathBuf::from(study));
    }
    let scenario = flags
        .get("--scenario")
        .ok_or("missing required flag `--study` (or scenario calibration_study reference)")?;
    if !flags.contains_key("--seed-manifest") {
        return Err("--scenario and --seed-manifest must be supplied together".into());
    }
    let scenario_path = PathBuf::from(scenario);
    let _scenario_bytes = read_bounded(&scenario_path, MAX_MANIFEST_BYTES)?;
    let reference = crate::scenario::calibration_study_reference(&scenario_path)?
        .ok_or("missing required flag `--study` and scenario calibration_study reference")?;
    resolve_manifest_reference(&scenario_path, &reference)
}
fn required_string<'a>(flags: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    flags
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required flag `{key}`"))
}
fn parse_bounded_usize(
    value: Option<&String>,
    name: &str,
    min: usize,
    max: usize,
    default: usize,
) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be an integer"))?;
    if !(min..=max).contains(&parsed) {
        return Err(format!("{name} must be in {min}..={max}"));
    }
    Ok(parsed)
}
fn parse_optional_usize(
    value: Option<&String>,
    name: &str,
    max: usize,
) -> Result<Option<usize>, String> {
    let Some(value) = value else { return Ok(None) };
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be an integer"))?;
    if parsed == 0 || parsed > max {
        return Err(format!("{name} must be in 1..={max}"));
    }
    Ok(Some(parsed))
}

fn read_bounded(path: &Path, max: u64) -> Result<String, String> {
    validate_path_components(path, false)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() > max {
        return Err(format!(
            "{} must be a regular file no larger than {max} bytes",
            path.display()
        ));
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(metadata.len().min(max) as usize);
    Read::take(file, max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err(format!(
            "{} grew beyond the {max} byte limit",
            path.display()
        ));
    }
    String::from_utf8(bytes).map_err(|e| format!("{} is not UTF-8: {e}", path.display()))
}
fn read_json_file(path: &Path, max: u64) -> Result<Value, String> {
    parse_json(&read_bounded(path, max)?, &path.display().to_string())
}
fn parse_json(text: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("invalid {label} JSON: {e}"))
}
fn canonical_json(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}
fn sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finalize())
}
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("study plan missing array `{key}`"))
}
fn count_missing_shards(tasks: &[Value], output: &Path) -> Result<usize, String> {
    if !output.exists() {
        return Ok(tasks.len());
    }
    validate_path_components(output, false)?;
    let metadata = fs::symlink_metadata(output).map_err(|e| e.to_string())?;
    if !metadata.file_type().is_dir() {
        return Err(format!(
            "{} is not a regular shard directory",
            output.display()
        ));
    }
    let mut missing = 0usize;
    for task in tasks {
        let task_id = task
            .get("task_id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && id.is_ascii())
            .ok_or("planned task has invalid task_id")?;
        let path = output.join(format!("task-{}.json", sha256(task_id.as_bytes())));
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => return Err(format!("unexpected shard entry type: {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing += 1,
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Ok(missing)
}
fn string_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("study plan missing string `{key}`"))
}

fn create_new_directory(path: &Path) -> Result<(), String> {
    validate_path_components(path, true)?;
    fs::create_dir(path)
        .map_err(|e| format!("output directory must be new: {}: {e}", path.display()))
}
fn checked_existing_dir(path: &Path) -> Result<PathBuf, String> {
    validate_path_components(path, false)?;
    let m = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !m.file_type().is_dir() {
        return Err(format!("{} is not a regular directory", path.display()));
    }
    fs::canonicalize(path).map_err(|e| e.to_string())
}
fn validate_path_components(path: &Path, allow_missing_leaf: bool) -> Result<(), String> {
    let abs = absolute_lexical(path)?;
    let mut current = PathBuf::new();
    for component in abs.components() {
        match component {
            // A Windows prefix is incomplete until its root separator arrives.
            // Metadata on e.g. `\\?\C:` is interpreted as a drive-relative
            // path and fails even when the eventual canonical path is valid.
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                continue;
            }
            Component::RootDir => current.push(component.as_os_str()),
            Component::Normal(v) => current.push(v),
            Component::ParentDir => {
                return Err("parent-directory path components are rejected".into());
            }
            _ => continue,
        }
        match fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!(
                    "symlink path component rejected: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(e)
                if allow_missing_leaf
                    && e.kind() == std::io::ErrorKind::NotFound
                    && current == abs =>
            {
                return Ok(());
            }
            Err(e) => return Err(format!("{}: {e}", current.display())),
        }
    }
    Ok(())
}
fn absolute_lexical(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path))
    }
}
fn write_file_no_replace(path: &Path, contents: &str) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    validate_path_components(path, true)?;
    let parent = path.parent().ok_or("artifact path must have a parent")?;
    let name = path
        .file_name()
        .ok_or("artifact path must have a filename")?;
    let mut temp_path = None;
    let mut f = None;
    for _ in 0..16 {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!(
            ".{}.tmp.{}.{}",
            name.to_string_lossy(),
            std::process::id(),
            nonce
        );
        let candidate = parent.join(temp_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temp_path = Some(candidate);
                f = Some(file);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("{}: {error}", candidate.display())),
        }
    }
    let temp_path = temp_path.ok_or("unable to allocate unique artifact temporary")?;
    let mut f = f.ok_or("artifact temporary handle missing")?;
    if let Err(error) = f.write_all(contents.as_bytes()).and_then(|()| f.sync_all()) {
        let _ = fs::remove_file(&temp_path);
        return Err(error.to_string());
    }
    drop(f);
    if let Err(error) = fs::hard_link(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("{}: {error}", path.display()));
    }
    fs::remove_file(&temp_path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn write_identical_or_new(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        let current = read_bounded(path, MAX_ATTEMPTS_BYTES)?;
        if current != contents {
            return Err(format!("existing artifact differs: {}", path.display()));
        }
        Ok(())
    } else {
        write_file_no_replace(path, contents)
    }
}
fn resolve_manifest_reference(manifest: &Path, reference: &str) -> Result<PathBuf, String> {
    let rel = Path::new(reference);
    if rel.is_absolute() {
        return Err("scenario calibration_study must be relative".into());
    }
    let base = manifest.parent().ok_or("scenario manifest has no parent")?;
    let base = fs::canonicalize(base).map_err(|e| e.to_string())?;
    let unresolved = base.join(rel);
    validate_path_components(&unresolved, false)?;
    let candidate = fs::canonicalize(unresolved).map_err(|e| e.to_string())?;
    if !candidate.starts_with(&base) {
        return Err("scenario calibration_study escapes manifest directory".into());
    }
    validate_path_components(&candidate, false)?;
    Ok(candidate)
}

#[cfg(all(test, windows))]
mod windows_path_validation_tests {
    use super::{validate_path_components, Component};
    use std::fs;
    use std::path::Path;

    #[test]
    fn accepts_canonicalized_verbatim_path() {
        let executable = std::env::current_exe().expect("current executable");
        let canonical = fs::canonicalize(executable).expect("canonical executable path");
        assert!(canonical.to_string_lossy().starts_with("\\\\?\\"));
        assert!(matches!(
            canonical.components().next(),
            Some(Component::Prefix(_))
        ));
        validate_path_components(Path::new(&canonical), false)
            .expect("canonicalized verbatim paths must pass component validation");
    }
}
