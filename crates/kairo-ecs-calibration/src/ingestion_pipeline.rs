//! Private C1 ingestion join. Local artifacts contain source identities.
use super::ingestion_normalize::{Counts, Limits, Normalizer};
use super::ingestion_sort::{external_sort, SortLimits};
use super::ingestion_validate::{validate_file, ValidationPolicy};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

pub(crate) struct PipelineConfig {
    pub(crate) normalization: Limits,
    pub(crate) sorting: SortLimits,
    pub(crate) validation: ValidationPolicy,
}
struct BundleGuard {
    path: PathBuf,
    complete: bool,
}
impl Drop for BundleGuard {
    fn drop(&mut self) {
        if !self.complete {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// New private output directory; the manifest is its last completion marker.
/// The outer source adapter supplies raw rows and the exact input files.
pub(crate) fn ingest<I>(
    template: Value,
    chunks: I,
    config: &PipelineConfig,
    inputs: &[PathBuf],
    output: &Path,
) -> Result<Value, String>
where
    I: Iterator<Item = Result<Vec<Value>, String>>,
{
    if inputs.is_empty() {
        return Err("input evidence is required".into());
    }
    let input_evidence: Vec<Value> = inputs
        .iter()
        .map(|p| fingerprint(p).map(|(hash, bytes)| json!({"sha256":hash,"bytes":bytes})))
        .collect::<Result<_, _>>()?;
    let mut normalizer = Normalizer::new(template.clone(), config.normalization)?;
    fs::create_dir(output).map_err(|e| format!("output must be a new private directory: {e}"))?;
    let mut guard = BundleGuard {
        path: output.into(),
        complete: false,
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(output, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let scratch = output.join("scratch");
    fs::create_dir(&scratch).map_err(|e| e.to_string())?;
    let mut events = writer(&output.join("unsorted.ndjson"))?;
    let mut exclusions = writer(&output.join("unsorted-exclusions.ndjson"))?;
    let mut outcomes = writer(&output.join("unsorted-outcomes.ndjson"))?;
    let mut diagnostics = writer(&output.join("diagnostics.ndjson"))?;
    let mut outcome_count = 0u64;
    let mut censor_counts = std::collections::BTreeMap::<String, u64>::new();
    for rows in chunks {
        let mapped = normalizer.push(rows?)?;
        for row in mapped.events {
            write_row(&mut events, &row)?;
        }
        for row in mapped.exclusions {
            write_row(&mut exclusions, &row)?;
        }
        for row in mapped.outcomes {
            outcome_count = outcome_count
                .checked_add(1)
                .ok_or("outcome count overflow")?;
            let status = row["censor_status"]
                .as_str()
                .ok_or("outcome status missing")?
                .to_owned();
            let n = censor_counts.entry(status).or_default();
            *n = n.checked_add(1).ok_or("censor count overflow")?;
            write_row(&mut outcomes, &row)?;
        }
        for row in mapped.diagnostics {
            write_row(&mut diagnostics, &row)?;
        }
    }
    for w in [
        &mut events,
        &mut exclusions,
        &mut outcomes,
        &mut diagnostics,
    ] {
        w.flush().map_err(|e| e.to_string())?;
    }
    drop((events, exclusions, outcomes, diagnostics));
    let counts = normalizer.counts();
    check_counts(counts)?;
    let mut sorted = writer(&output.join("sorted.ndjson"))?;
    let sort = external_sort(
        lines(
            &output.join("unsorted.ndjson"),
            config.sorting.max_record_bytes,
        )?,
        &scratch,
        config.sorting,
        |row| write_row(&mut sorted, &row),
    )?;
    sorted.flush().map_err(|e| e.to_string())?;
    drop(sorted);
    if sort.rows != counts.accepted_units {
        return Err("sorted/event count mismatch".into());
    }
    let validation = validate_file(
        &output.join("sorted.ndjson"),
        &output.join("events.ndjson"),
        &output.join("quarantine.ndjson"),
        &config.validation,
    )?;
    if validation.input_events != counts.accepted_units
        || validation
            .valid_events
            .checked_add(validation.quarantined_events)
            != Some(validation.input_events)
    {
        return Err("validation count mismatch".into());
    }
    for (source, target) in [
        ("unsorted-exclusions.ndjson", "exclusions.ndjson"),
        ("unsorted-outcomes.ndjson", "outcomes.ndjson"),
    ] {
        let mut sink = writer(&output.join(target))?;
        // Auxiliary populations have their own deterministic compact-JSON byte order.
        let records=lines(&output.join(source),config.sorting.max_record_bytes)?.map(|row|row.and_then(|payload|{
            let key=serde_json::to_string(&payload).map_err(|e|e.to_string())?;
            Ok(json!({"relative_ticks":"0","case_key":"auxiliary","occurrence":0,"event_kind_rank":0,"source_event_key":key,"source_order":0,"payload":payload}))
        }));
        external_sort(records, &scratch, config.sorting, |row| {
            write_row(&mut sink, &row["payload"])
        })?;
        sink.flush().map_err(|e| e.to_string())?;
    }
    let mut populations = serde_json::Map::new();
    for (name, file) in [
        ("events", "events.ndjson"),
        ("quarantine", "quarantine.ndjson"),
        ("exclusions", "exclusions.ndjson"),
        ("outcomes", "outcomes.ndjson"),
    ] {
        let (hash, bytes) = fingerprint(&output.join(file))?;
        let rows = count_lines(&output.join(file), config.sorting.max_record_bytes)?;
        populations.insert(
            name.into(),
            json!({"sha256":hash,"bytes":bytes,"rows":rows}),
        );
    }
    if populations["events"]["rows"] != json!(validation.valid_events)
        || populations["quarantine"]["rows"] != json!(validation.quarantined_events)
        || populations["exclusions"]["rows"] != json!(counts.excluded_units)
        || populations["outcomes"]["rows"] != json!(outcome_count)
    {
        return Err("artifact population counts do not reconcile".into());
    }
    // Verify exact source files did not change while their decoded rows were ingested.
    for (path, prior) in inputs.iter().zip(&input_evidence) {
        let (hash, bytes) = fingerprint(path)?;
        if prior != &json!({"sha256":hash,"bytes":bytes}) {
            return Err("input evidence changed during ingestion".into());
        }
    }
    let manifest = json!({"manifest_version":"c1.ingestion.v1","privacy":"local-only; publish only reviewed synthetic evidence","serialization":"serde-json-btree-ndjson-v1","logical_schema":"calibration-v1","physical_schema_version":2,"dataset_id":template["dataset_id"],"mapping_version":template["mapping_version"],"mapping_sha256":normalizer.mapping_hash(),"inputs":input_evidence,"origin_utc":template["origin_utc"],"tick_resolution":"1ns","rounding":"reject unrepresentable precision; preserve source precision","ordering":"c0.six-field-v1","source_rows":counts.source_rows,"candidate_units":counts.candidate_units,"mapper_accepted_units":counts.accepted_units,"mapper_excluded_units":counts.excluded_units,"failed_units":counts.failed_units,"unresolved_units":counts.unresolved_units,"candidate_conservation":true,"cohort_denominator":counts.cohort_denominator,"missing_triage":counts.missing_triage,"outcomes":outcome_count,"censor_status_counts":censor_counts,"validation":{"input_events":validation.input_events,"valid_events":validation.valid_events,"quarantined_events":validation.quarantined_events,"invalid_cases":validation.invalid_cases,"censored_cases":validation.censored_cases,"reasons":validation.reasons,"resource_feasible":validation.resource_feasible,"input_sha256":validation.input_sha256},"sort":{"version":"bounded-c0-runs-v1","rows":sort.rows,"runs":sort.runs,"merge_passes":sort.merge_passes,"max_run_rows":config.sorting.max_run_rows,"max_run_bytes":config.sorting.max_run_bytes,"max_record_bytes":config.sorting.max_record_bytes,"merge_fan_in":config.sorting.merge_fan_in},"normalization_limits":{"max_chunk_rows":config.normalization.max_chunk_rows,"max_chunk_bytes":config.normalization.max_chunk_bytes,"max_identities":config.normalization.max_identities},"populations":populations});
    for name in [
        "unsorted.ndjson",
        "unsorted-exclusions.ndjson",
        "unsorted-outcomes.ndjson",
        "sorted.ndjson",
    ] {
        fs::remove_file(output.join(name)).map_err(|e| e.to_string())?;
    }
    fs::remove_dir(&scratch).map_err(|e| format!("sort scratch not empty: {e}"))?;
    let mut marker = writer(&output.join("manifest.json"))?;
    write_row(&mut marker, &manifest)?;
    marker.flush().map_err(|e| e.to_string())?;
    marker.get_ref().sync_all().map_err(|e| e.to_string())?;
    guard.complete = true;
    Ok(manifest)
}
fn check_counts(c: Counts) -> Result<(), String> {
    if c.accepted_units
        .checked_add(c.excluded_units)
        .and_then(|n| n.checked_add(c.failed_units))
        .and_then(|n| n.checked_add(c.unresolved_units))
        != Some(c.candidate_units)
    {
        return Err("candidate conservation failed".into());
    }
    Ok(())
}
fn writer(path: &Path) -> Result<BufWriter<File>, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(BufWriter::new)
        .map_err(|e| e.to_string())
}
fn write_row(w: &mut BufWriter<File>, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *w, value).map_err(|e| e.to_string())?;
    w.write_all(b"\n").map_err(|e| e.to_string())
}
pub(crate) fn fingerprint(path: &Path) -> Result<(String, u64), String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        bytes = bytes.checked_add(n as u64).ok_or("file size overflow")?;
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}
pub(crate) struct Lines {
    reader: BufReader<File>,
    cap: usize,
    failed: bool,
}
impl Iterator for Lines {
    type Item = Result<Value, String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let mut raw = Vec::new();
        let n = match self
            .reader
            .by_ref()
            .take((self.cap + 2) as u64)
            .read_until(b'\n', &mut raw)
        {
            Ok(n) => n,
            Err(e) => {
                self.failed = true;
                return Some(Err(e.to_string()));
            }
        };
        if n == 0 {
            return None;
        }
        if raw.len() > self.cap + 1 || raw.last() != Some(&b'\n') {
            self.failed = true;
            return Some(Err("oversized or truncated NDJSON record".into()));
        }
        raw.pop();
        let value = serde_json::from_slice(&raw).map_err(|e| e.to_string());
        if value.is_err() {
            self.failed = true;
        }
        Some(value)
    }
}
pub(crate) fn lines(path: &Path, cap: usize) -> Result<Lines, String> {
    if cap == 0 || cap.checked_add(2).is_none() {
        return Err("invalid record byte bound".into());
    }
    Ok(Lines {
        reader: BufReader::new(File::open(path).map_err(|e| e.to_string())?),
        cap,
        failed: false,
    })
}
fn count_lines(path: &Path, cap: usize) -> Result<u64, String> {
    lines(path, cap)?.try_fold(0u64, |n, v| {
        v?;
        n.checked_add(1).ok_or_else(|| "row count overflow".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_validate::{ValidationBounds, ValidationMode};
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "kairos-c13-pipeline-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    fn clock(second: u64) -> Value {
        json!({"raw":format!("2020-01-01T00:00:{second:02}Z"),"representation":"RFC3339","precision":"second","lineage":{"status":"observed"}})
    }
    fn source() -> (Value, Vec<Value>) {
        let bindings:Vec<Value>=[("arrival","ARRIVAL",0),("bed_entered","BED_ENTERED",1),("physical_departure","PHYSICAL_DEPARTURE",2)].into_iter().map(|(kind,ty,rank)|json!({"source_event_type":ty,"kind":kind,"rank":rank.to_string(),"occurrence_index":0,"occurrence_field":"at","key_field":"id","order_field":"seq"})).collect();
        let template = json!({"profile_version":"c11.synthetic-map-v1","dataset_id":"synthetic-c13","mapping_version":"synthetic-map-v1","origin_utc":"2020-01-01T00:00:00Z","case_key_field":"case","source_family":"fixture","shape":"long","event_kind_field":"kind","resource_key_field":"resource","location_key_field":"location","event_bindings":bindings,"rows":[]});
        let mut rows = Vec::new();
        for (case, times) in [("Z", [0, 1, 4]), ("A", [2, 4, 7])] {
            for (kind, second) in ["arrival", "bed_entered", "physical_departure"]
                .into_iter()
                .zip(times)
            {
                let mut row = json!({"case":case,"id":format!("{case}-{kind}"),"seq":rows.len(),"kind":kind,"at":clock(second),"resource":"bed-1","location":"zone-1"});
                if kind == "physical_departure" {
                    row["outcome"] = json!({"endpoint":"departure","risk_start":clock(times[0]),"last_observed":clock(second),"event_clock":if case=="A"{Value::Null}else{clock(second)},"censor_cause":if case=="A"{"window_end"}else{"departed"},"censor_status":if case=="A"{"right"}else{"not_censored"},"lineage":{"status":"observed"}});
                }
                rows.push(row);
            }
        }
        rows.push(json!({"case":"E","id":"excluded-missing","seq":6,"kind":"arrival","at":null}));
        (template, rows)
    }
    fn config(run_rows: usize) -> PipelineConfig {
        PipelineConfig {
            normalization: Limits {
                max_chunk_rows: 100,
                max_chunk_bytes: 1 << 20,
                max_identities: 100,
            },
            sorting: SortLimits {
                max_run_rows: run_rows,
                max_run_bytes: 1 << 20,
                max_record_bytes: 1 << 16,
                merge_fan_in: 2,
            },
            validation: ValidationPolicy {
                mode: ValidationMode::Strict,
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
    #[test]
    fn actual_source_pipeline_hashes_survive_chunks_spills_permutations_and_counts_reconcile() {
        let root = root();
        let (template, rows) = source();
        let input = root.join("source.json");
        fs::write(&input, serde_json::to_vec(&rows).unwrap()).unwrap();
        let mut hashes = Vec::new();
        for (index, (chunk, run, reverse)) in [(1, 1, false), (2, 2, true), (7, 100, false)]
            .into_iter()
            .enumerate()
        {
            let mut physical = rows.clone();
            if reverse {
                physical.reverse();
            }
            let output = root.join(format!("layout-{index}"));
            let chunks = physical.chunks(chunk).map(|r| Ok(r.to_vec()));
            let manifest = ingest(
                template.clone(),
                chunks,
                &config(run),
                std::slice::from_ref(&input),
                &output,
            )
            .unwrap();
            assert_eq!(manifest["source_rows"], 7);
            assert_eq!(manifest["candidate_units"], 7);
            assert_eq!(manifest["mapper_accepted_units"], 6);
            assert_eq!(manifest["mapper_excluded_units"], 1);
            assert_eq!(manifest["outcomes"], 2);
            assert_eq!(manifest["validation"]["valid_events"], 6);
            assert_eq!(manifest["validation"]["resource_feasible"], true);
            assert_eq!(manifest["censor_status_counts"]["right"], 1);
            assert_eq!(manifest["censor_status_counts"]["not_censored"], 1);
            hashes.push(manifest["populations"].clone());
            assert!(!output.join("scratch").exists());
            if index == 0 {
                if let Some(target) = std::env::var_os("KAIROS_C13_CAPTURE_DIR") {
                    let target = PathBuf::from(target);
                    fs::create_dir_all(&target).unwrap();
                    for file in [
                        "events.ndjson",
                        "exclusions.ndjson",
                        "outcomes.ndjson",
                        "manifest.json",
                    ] {
                        fs::copy(output.join(file), target.join(file)).unwrap();
                    }
                    fs::write(
                        target.join("template.json"),
                        serde_json::to_vec(&template).unwrap(),
                    )
                    .unwrap();
                    fs::write(
                        target.join("source.json"),
                        serde_json::to_vec(&rows).unwrap(),
                    )
                    .unwrap();
                }
            }
        }
        assert_eq!(hashes[0], hashes[1]);
        assert_eq!(hashes[0], hashes[2]);
        // No source keys, paths or raw source fields in the local manifest.
        let text = fs::read_to_string(root.join("layout-0/manifest.json")).unwrap();
        assert!(!text.contains("excluded-missing"));
        assert!(!text.contains("source.json"));
        assert!(!text.contains("raw_event"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn errors_remove_only_new_bundle_and_missing_evidence_never_succeeds() {
        let root = root();
        let (template, rows) = source();
        let input = root.join("source.json");
        fs::write(&input, serde_json::to_vec(&rows).unwrap()).unwrap();
        let output = root.join("failed");
        let bad = vec![Ok(vec![rows[0].clone()]), Ok(vec![rows[0].clone()])];
        assert!(ingest(
            template.clone(),
            bad.into_iter(),
            &config(1),
            std::slice::from_ref(&input),
            &output
        )
        .is_err());
        assert!(!output.exists());
        assert!(ingest(
            template.clone(),
            std::iter::empty(),
            &config(1),
            &[],
            &output
        )
        .is_err());
        assert!(!output.exists());
        fs::create_dir(&output).unwrap();
        fs::write(output.join("keep"), b"untouched").unwrap();
        assert!(ingest(
            template,
            rows.chunks(1).map(|r| Ok(r.to_vec())),
            &config(1),
            std::slice::from_ref(&input),
            &output
        )
        .is_err());
        assert_eq!(fs::read(output.join("keep")).unwrap(), b"untouched");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn transported_source_rows_use_the_same_actual_pipeline() {
        let Some(path) = std::env::var_os("KAIROS_C13_SOURCE_ROWS") else {
            return;
        };
        let root = root();
        let path = PathBuf::from(path);
        let rows: Vec<Value> = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let (template, _) = source();
        let output = root.join("reingested");
        let manifest = ingest(
            template,
            rows.chunks(2).map(|r| Ok(r.to_vec())),
            &config(1),
            std::slice::from_ref(&path),
            &output,
        )
        .unwrap();
        let expected: Value = serde_json::from_slice(
            &fs::read(
                std::env::var_os("KAIROS_C13_EXPECTED_MANIFEST")
                    .expect("transport reingestion requires original actual manifest"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["populations"], expected["populations"]);
        fs::remove_dir_all(root).unwrap();
    }
}
