//! Bounded two-pass validation of sorted C0 trace-event NDJSON.
//!
//! This module is private experimental ingestion code. It preserves input
//! records verbatim in its valid/quarantine outputs and never infers missing
//! event or interval data.

use super::ingestion_normalize::order_key;
use super::trace_order::TraceOrderKeyV1;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationMode {
    Strict,
    Quarantine,
    Exploratory,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ValidationBounds {
    pub(crate) max_cases: usize,
    pub(crate) max_state_entries: usize,
    pub(crate) max_record_bytes: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ValidationPolicy {
    pub(crate) mode: ValidationMode,
    pub(crate) declared_kinds: BTreeSet<String>,
    pub(crate) required_kinds: Vec<String>,
    pub(crate) precedence: Vec<(String, String)>,
    pub(crate) occupancy_pairs: Vec<(String, String)>,
    pub(crate) capacities: BTreeMap<(String, String), u32>,
    pub(crate) window: Option<(u128, u128)>,
    pub(crate) bounds: ValidationBounds,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ValidationReport {
    pub(crate) input_events: u64,
    pub(crate) valid_events: u64,
    pub(crate) quarantined_events: u64,
    pub(crate) invalid_cases: u64,
    pub(crate) censored_cases: u64,
    pub(crate) reasons: BTreeMap<String, u64>,
    pub(crate) resource_feasible: bool,
    pub(crate) input_sha256: String,
}

type ResourceLocation = (String, String);
type PairKey = (String, u32, String, String, String);

#[derive(Default)]
struct OccurrenceState {
    relevant_kinds: BTreeMap<String, (u64, u128)>,
}

#[derive(Default)]
struct PairState {
    enter: Option<u128>,
    exit: Option<u128>,
}

#[derive(Default)]
struct CaseState {
    occurrences: BTreeMap<u32, OccurrenceState>,
    reasons: BTreeSet<String>,
    censored: bool,
}

struct ScanState {
    cases: BTreeMap<String, CaseState>,
    pairs: BTreeMap<PairKey, PairState>,
    active: BTreeMap<ResourceLocation, BTreeMap<PairKey, String>>,
    state_entries: usize,
    report: ValidationReport,
}

impl ScanState {
    fn new() -> Self {
        Self {
            cases: BTreeMap::new(),
            pairs: BTreeMap::new(),
            active: BTreeMap::new(),
            state_entries: 0,
            report: ValidationReport {
                resource_feasible: true,
                ..ValidationReport::default()
            },
        }
    }

    fn reserve(&mut self, count: usize, limit: usize) -> Result<(), String> {
        self.state_entries = self
            .state_entries
            .checked_add(count)
            .ok_or("validation state count overflow")?;
        if self.state_entries > limit {
            return Err(format!("validation state limit exceeded ({limit})"));
        }
        Ok(())
    }

    fn mark(&mut self, case: &str, reason: &str, censored: bool) {
        let state = self.cases.entry(case.to_owned()).or_default();
        if state.reasons.insert(reason.to_owned()) {
            *self.report.reasons.entry(reason.to_owned()).or_default() += 1;
        }
        state.censored |= censored;
        if reason.contains("occupancy")
            || reason == "missing_resource_key"
            || reason == "missing_location_key"
            || reason == "missing_or_invalid_capacity"
        {
            self.report.resource_feasible = false;
        }
    }
}

/// Validate a sorted trace-event NDJSON file, then route complete cases.
///
/// Both output paths must be absent and reside on the same filesystem as their
/// temporary siblings. Strict mode never publishes output files. The two final
/// hard-link publications are individually exclusive, not an atomic pair.
pub(crate) fn validate_file(
    input: &Path,
    valid: &Path,
    quarantine: &Path,
    policy: &ValidationPolicy,
) -> Result<ValidationReport, String> {
    validate_policy(policy)?;
    if valid == quarantine || valid == input || quarantine == input {
        return Err("input and output paths must be distinct".into());
    }
    if valid.exists() || quarantine.exists() {
        return Err("output path already exists".into());
    }

    let mut scan = ScanState::new();
    let mut first_hash = Sha256::new();
    let mut first_bytes = 0u64;
    let mut tick_group: Vec<Value> = Vec::new();
    let mut group_tick: Option<u128> = None;
    let mut previous_key: Option<TraceOrderKeyV1> = None;
    let first = File::open(input).map_err(|e| format!("open input: {e}"))?;
    let mut reader = BufReader::new(first);
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = read_bounded_line(&mut reader, &mut line, policy.bounds.max_record_bytes)
            .map_err(|e| format!("read input: {e}"))?;
        if n == 0 {
            break;
        }
        if !line.ends_with(b"\n") {
            return Err("input contains a truncated NDJSON record".into());
        }
        first_hash.update(&line);
        first_bytes = first_bytes
            .checked_add(n as u64)
            .ok_or("input byte count overflow")?;
        let record_bytes = line.strip_suffix(b"\n").unwrap_or(&line);
        if record_bytes.len() > policy.bounds.max_record_bytes {
            return Err("input record exceeds max_record_bytes".into());
        }
        if record_bytes.is_empty() {
            return Err("blank NDJSON record".into());
        }
        let record: Value = serde_json::from_slice(record_bytes)
            .map_err(|e| format!("invalid NDJSON record: {e}"))?;
        let key = order_key(&record)?;
        if previous_key
            .as_ref()
            .is_some_and(|previous| key < *previous)
        {
            return Err("input records are not sorted by the C0 order tuple".into());
        }
        previous_key = Some(key);
        let parsed = parse_event(&record, policy)?;
        if let Some(prev) = group_tick {
            if parsed.tick < prev {
                return Err("input records are not sorted by relative_ticks".into());
            }
            if parsed.tick != prev {
                process_tick(&mut scan, &tick_group, prev, policy)?;
                tick_group.clear();
                group_tick = Some(parsed.tick);
            }
        } else {
            group_tick = Some(parsed.tick);
        }
        if tick_group.len() >= policy.bounds.max_state_entries {
            return Err("same-tick validation state limit exceeded".into());
        }
        tick_group.push(record);
        scan.report.input_events = scan
            .report
            .input_events
            .checked_add(1)
            .ok_or("input event count overflow")?;
        register_case_event(&mut scan, &parsed, policy)?;
    }
    if let Some(tick) = group_tick {
        process_tick(&mut scan, &tick_group, tick, policy)?;
    }
    finish_pairs(&mut scan, policy);
    finish_cases(&mut scan, policy)?;
    scan.report.input_sha256 = hex(&first_hash.finalize());
    if policy.mode == ValidationMode::Strict && scan.report.invalid_cases > 0 {
        return Err(format!(
            "strict validation rejected input: {} invalid case(s)",
            scan.report.invalid_cases
        ));
    }

    let valid_stage = stage_path(valid)?;
    let quarantine_stage = stage_path(quarantine)?;
    let _stage_cleanup = StageCleanup(valid_stage.clone(), quarantine_stage.clone());
    let mut valid_writer = staged_writer(&valid_stage)?;
    let mut quarantine_writer = staged_writer(&quarantine_stage)?;
    let second = File::open(input).map_err(|e| format!("reopen input: {e}"))?;
    let mut reader = BufReader::new(second);
    let mut second_hash = Sha256::new();
    let mut second_bytes = 0u64;
    scan.report.valid_events = 0;
    scan.report.quarantined_events = 0;
    let mut second_line = Vec::new();
    loop {
        second_line.clear();
        let n = read_bounded_line(
            &mut reader,
            &mut second_line,
            policy.bounds.max_record_bytes,
        )
        .map_err(|e| format!("reread input: {e}"))?;
        if n == 0 {
            break;
        }
        if !second_line.ends_with(b"\n") {
            return Err("input changed to a truncated NDJSON record".into());
        }
        second_hash.update(&second_line);
        second_bytes = second_bytes
            .checked_add(n as u64)
            .ok_or("second-pass byte count overflow")?;
        let bytes = second_line.strip_suffix(b"\n").unwrap_or(&second_line);
        let record: Value = match serde_json::from_slice(bytes) {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "input changed or became invalid between passes: {e}"
                ));
            }
        };
        let case = record
            .get("case_key")
            .and_then(Value::as_str)
            .ok_or("missing case_key during second pass")?;
        let bad = scan.cases.get(case).is_some_and(|c| !c.reasons.is_empty());
        if bad {
            scan.report.quarantined_events = scan
                .report
                .quarantined_events
                .checked_add(1)
                .ok_or("quarantine count overflow")?;
        } else {
            scan.report.valid_events = scan
                .report
                .valid_events
                .checked_add(1)
                .ok_or("valid count overflow")?;
        }
        let writer = if bad {
            &mut quarantine_writer
        } else {
            &mut valid_writer
        };
        writer
            .write_all(&second_line)
            .map_err(|e| format!("write staged output: {e}"))?;
    }
    valid_writer
        .flush()
        .map_err(|e| format!("flush valid stage: {e}"))?;
    quarantine_writer
        .flush()
        .map_err(|e| format!("flush quarantine stage: {e}"))?;
    drop(valid_writer);
    drop(quarantine_writer);
    if second_bytes != first_bytes || hex(&second_hash.finalize()) != scan.report.input_sha256 {
        return Err("input changed between validation and routing passes".into());
    }

    scan.report.invalid_cases = scan
        .cases
        .values()
        .filter(|c| !c.reasons.is_empty())
        .count() as u64;
    scan.report.censored_cases = scan.cases.values().filter(|c| c.censored).count() as u64;

    publish_exclusive(&valid_stage, valid)?;
    if let Err(e) = publish_exclusive(&quarantine_stage, quarantine) {
        let _ = fs::remove_file(valid);
        return Err(e);
    }
    Ok(scan.report)
}

#[derive(Clone)]
struct Event {
    case: String,
    occurrence: u32,
    kind: String,
    tick: u128,
    resource: Option<String>,
    location: Option<String>,
}

fn parse_event(value: &Value, policy: &ValidationPolicy) -> Result<Event, String> {
    if value.get("record_type").and_then(Value::as_str) != Some("trace_event.v1")
        || value.get("schema_version").and_then(Value::as_str) != Some("calibration-v1")
    {
        return Err("validator accepts trace_event.v1 calibration-v1 records only".into());
    }
    let case = required_string(value, "case_key")?;
    let kind = required_string(value, "event_kind")?;
    if !policy.declared_kinds.contains(kind) {
        return Err(format!("undeclared event kind {kind}"));
    }
    let occurrence = value
        .get("occurrence")
        .and_then(Value::as_u64)
        .filter(|n| *n <= u32::MAX as u64)
        .ok_or("invalid occurrence")? as u32;
    let tick = value
        .get("relative_ticks")
        .and_then(Value::as_str)
        .and_then(parse_u128)
        .ok_or("relative_ticks must be canonical u128 decimal string")?;
    let resource = nullable_string(value, "resource_key")?;
    let location = nullable_string(value, "location_key")?;
    Ok(Event {
        case: case.to_owned(),
        occurrence,
        kind: kind.to_owned(),
        tick,
        resource,
        location,
    })
}

fn register_case_event(
    scan: &mut ScanState,
    event: &Event,
    policy: &ValidationPolicy,
) -> Result<(), String> {
    if !scan.cases.contains_key(&event.case) {
        if scan.cases.len() >= policy.bounds.max_cases {
            return Err("validation max_cases exceeded".into());
        }
        scan.reserve(1, policy.bounds.max_state_entries)?;
        scan.cases.insert(event.case.clone(), CaseState::default());
    }
    let relevant = policy.required_kinds.contains(&event.kind)
        || policy
            .precedence
            .iter()
            .any(|(a, b)| a == &event.kind || b == &event.kind)
        || policy
            .occupancy_pairs
            .iter()
            .any(|(a, b)| a == &event.kind || b == &event.kind);
    let mut add_state = false;
    {
        let case = scan.cases.get_mut(&event.case).expect("inserted above");
        if let Some((start, end)) = policy.window {
            if event.tick < start || event.tick >= end {
                if case.reasons.insert("observation_window_exceeded".into()) {
                    *scan
                        .report
                        .reasons
                        .entry("observation_window_exceeded".into())
                        .or_default() += 1;
                }
                case.censored = true;
            }
        }
        if let std::collections::btree_map::Entry::Vacant(entry) =
            case.occurrences.entry(event.occurrence)
        {
            entry.insert(OccurrenceState::default());
            add_state = true;
        }
        if relevant {
            let occurrence = case
                .occurrences
                .get_mut(&event.occurrence)
                .expect("created above");
            match occurrence.relevant_kinds.get_mut(&event.kind) {
                Some((count, _)) => *count = count.saturating_add(1),
                None => {
                    occurrence
                        .relevant_kinds
                        .insert(event.kind.clone(), (1, event.tick));
                    add_state = true;
                }
            }
        }
    }
    if add_state {
        scan.reserve(1, policy.bounds.max_state_entries)?;
    }
    Ok(())
}

fn process_tick(
    scan: &mut ScanState,
    records: &[Value],
    tick: u128,
    policy: &ValidationPolicy,
) -> Result<(), String> {
    let mut events = Vec::new();
    for record in records {
        events.push(parse_event(record, policy)?);
    }
    let mut starts: BTreeMap<ResourceLocation, Vec<(PairKey, String)>> = BTreeMap::new();
    let mut exits: BTreeMap<ResourceLocation, Vec<(PairKey, String)>> = BTreeMap::new();
    for event in &events {
        for (enter_kind, exit_kind) in &policy.occupancy_pairs {
            if event.kind != *enter_kind && event.kind != *exit_kind {
                continue;
            }
            let Some(resource) = event.resource.clone() else {
                scan.mark(&event.case, "missing_resource_key", false);
                continue;
            };
            let Some(location) = event.location.clone() else {
                scan.mark(&event.case, "missing_location_key", false);
                continue;
            };
            let resource_location = (resource.clone(), location.clone());
            let cap = policy.capacities.get(&resource_location).copied();
            let Some(capacity) = cap.filter(|v| *v > 0) else {
                scan.mark(&event.case, "missing_or_invalid_capacity", false);
                continue;
            };
            let _ = capacity;
            let key: PairKey = (
                event.case.clone(),
                event.occurrence,
                resource,
                location,
                enter_kind.clone(),
            );
            if !scan.pairs.contains_key(&key) {
                scan.reserve(1, policy.bounds.max_state_entries)?;
            }
            let pair = scan.pairs.entry(key.clone()).or_default();
            if event.kind == *enter_kind {
                if pair.enter.replace(tick).is_some() {
                    scan.mark(&event.case, "duplicate_occupancy_enter", false);
                }
                starts
                    .entry(resource_location)
                    .or_default()
                    .push((key, event.case.clone()));
            } else {
                if pair.exit.replace(tick).is_some() {
                    scan.mark(&event.case, "duplicate_occupancy_exit", false);
                }
                exits
                    .entry(resource_location)
                    .or_default()
                    .push((key, event.case.clone()));
            }
        }
    }
    // Release all positive-duration occupants at this tick before admissions.
    for (resource, rows) in exits {
        for (key, case) in rows {
            let start = scan.pairs.get(&key).and_then(|p| p.enter);
            match start {
                Some(start) if start < tick => {
                    if let Some(active) = scan.active.get_mut(&resource) {
                        active.remove(&key);
                    }
                }
                Some(start) if start == tick => { /* zero interval: no occupancy */ }
                Some(_) => scan.mark(&case, "reversed_occupancy_interval", false),
                None => scan.mark(&case, "unmatched_occupancy_exit", false),
            }
        }
    }
    let mut by_resource: BTreeMap<ResourceLocation, Vec<(PairKey, String)>> = BTreeMap::new();
    for (resource, rows) in starts {
        for (key, case) in rows {
            let end = scan.pairs.get(&key).and_then(|p| p.exit);
            if end == Some(tick) {
                continue;
            }
            if end.is_some_and(|end| end < tick) {
                scan.mark(&case, "reversed_occupancy_interval", false);
                continue;
            }
            by_resource
                .entry(resource.clone())
                .or_default()
                .push((key, case));
        }
    }
    for (resource, admissions) in by_resource {
        let capacity = policy.capacities.get(&resource).copied().unwrap_or(0);
        let active_cases: Vec<String> = scan
            .active
            .get(&resource)
            .into_iter()
            .flat_map(|active| active.values().cloned())
            .collect();
        let projected = active_cases
            .len()
            .checked_add(admissions.len())
            .ok_or("occupancy count overflow")?;
        if capacity == 0 {
            for (_, case) in &admissions {
                scan.mark(case, "missing_or_invalid_capacity", false);
            }
            continue;
        }
        if projected > capacity as usize {
            scan.report.resource_feasible = false;
            let involved: BTreeSet<String> = active_cases
                .into_iter()
                .chain(admissions.iter().map(|(_, c)| c.clone()))
                .collect();
            for case in involved {
                scan.mark(&case, "resource_overcapacity", false);
            }
        }
        let active = scan.active.entry(resource).or_default();
        for (key, case) in admissions {
            active.insert(key, case);
        }
    }
    Ok(())
}

fn finish_pairs(scan: &mut ScanState, policy: &ValidationPolicy) {
    let pairs: Vec<(PairKey, Option<u128>, Option<u128>)> = scan
        .pairs
        .iter()
        .map(|(k, p)| (k.clone(), p.enter, p.exit))
        .collect();
    for (key, enter, exit) in pairs {
        match (enter, exit) {
            (None, _) => scan.mark(&key.0, "unmatched_occupancy_exit", false),
            (Some(_), None) => scan.mark(&key.0, "open_occupancy_interval", false),
            (Some(a), Some(b)) if b < a => scan.mark(&key.0, "reversed_occupancy_interval", false),
            (Some(a), Some(b))
                if a == b && policy.occupancy_pairs.iter().any(|(e, _)| key.4 == *e) => {}
            _ => {}
        }
    }
}

fn finish_cases(scan: &mut ScanState, policy: &ValidationPolicy) -> Result<(), String> {
    // Precompute the required anchors once. Rebuilding this set per occurrence
    // multiplied temporary allocations by the number of occurrences.
    let mut required: BTreeSet<&str> = policy.required_kinds.iter().map(String::as_str).collect();
    for (before, after) in &policy.precedence {
        required.insert(before);
        required.insert(after);
    }

    let occurrence_count = scan.cases.values().try_fold(0usize, |sum, case| {
        sum.checked_add(case.occurrences.len())
            .ok_or("validation occurrence count overflow")
    })?;
    let checks_per_occurrence = required
        .len()
        .checked_add(policy.precedence.len())
        .ok_or("validation work count overflow")?;
    let validation_work = occurrence_count
        .checked_mul(checks_per_occurrence)
        .ok_or("validation work count overflow")?;
    if validation_work > policy.bounds.max_state_entries {
        return Err(format!(
            "validation work limit exceeded ({})",
            policy.bounds.max_state_entries
        ));
    }

    // Each case can contribute at most one count for each diagnostic class.
    // Aggregate flags per case, so memory remains O(number of cases), not
    // O(number of occurrences × number of policy edges).
    let ScanState { cases, report, .. } = scan;
    for case in cases.values_mut() {
        let (mut missing_anchor, mut duplicate_anchor, mut precedence_violation) =
            (false, false, false);
        for occurrence in case.occurrences.values() {
            for &kind in &required {
                match occurrence.relevant_kinds.get(kind) {
                    None => missing_anchor = true,
                    Some((count, _)) if *count != 1 => duplicate_anchor = true,
                    _ => {}
                }
            }
            for (before, after) in &policy.precedence {
                if let (Some((_, a)), Some((_, b))) = (
                    occurrence.relevant_kinds.get(before),
                    occurrence.relevant_kinds.get(after),
                ) {
                    if a > b {
                        precedence_violation = true;
                    }
                }
            }
        }
        for (reason, present) in [
            ("missing_required_anchor", missing_anchor),
            ("duplicate_required_anchor", duplicate_anchor),
            ("precedence_violation", precedence_violation),
        ] {
            if present && case.reasons.insert(reason.into()) {
                *report.reasons.entry(reason.into()).or_default() += 1;
            }
        }
    }
    report.invalid_cases = cases.values().filter(|c| !c.reasons.is_empty()).count() as u64;
    report.censored_cases = cases.values().filter(|c| c.censored).count() as u64;
    if policy.mode == ValidationMode::Exploratory {
        // Capacity conflicts are reported but remain in the valid stream.
        for case in cases.values_mut() {
            case.reasons.remove("resource_overcapacity");
        }
        report.invalid_cases = cases.values().filter(|c| !c.reasons.is_empty()).count() as u64;
    }
    Ok(())
}

fn validate_policy(policy: &ValidationPolicy) -> Result<(), String> {
    let b = policy.bounds;
    if b.max_cases == 0 || b.max_state_entries == 0 || b.max_record_bytes == 0 {
        return Err("validation bounds must be nonzero".into());
    }
    if b.max_record_bytes.checked_add(2).is_none() {
        return Err("max_record_bytes is too large".into());
    }
    if policy.declared_kinds.len() > b.max_state_entries
        || policy.required_kinds.len() > b.max_state_entries
        || policy.precedence.len() > b.max_state_entries
        || policy.occupancy_pairs.len() > b.max_state_entries
        || policy.capacities.len() > b.max_state_entries
    {
        return Err("policy exceeds max_state_entries".into());
    }
    if policy.declared_kinds.is_empty() || policy.declared_kinds.iter().any(|k| k.trim().is_empty())
    {
        return Err("declared_kinds must be nonempty names".into());
    }
    if policy
        .required_kinds
        .iter()
        .any(|k| !policy.declared_kinds.contains(k))
    {
        return Err("required kind is undeclared".into());
    }
    let mut edges = BTreeMap::<&str, Vec<&str>>::new();
    for (a, b) in &policy.precedence {
        if !policy.declared_kinds.contains(a) || !policy.declared_kinds.contains(b) {
            return Err("precedence edge references undeclared kind".into());
        }
        edges.entry(a).or_default().push(b);
    }
    let mut indegree: BTreeMap<&str, usize> = policy
        .declared_kinds
        .iter()
        .map(|k| (k.as_str(), 0))
        .collect();
    let mut unique_edges = BTreeSet::new();
    for (a, b) in &policy.precedence {
        if !unique_edges.insert((a, b)) {
            return Err("duplicate precedence edge".into());
        }
        let count = indegree
            .get_mut(b.as_str())
            .ok_or("undeclared edge endpoint")?;
        *count = count.checked_add(1).ok_or("precedence count overflow")?;
    }
    let mut ready: BTreeSet<&str> = indegree
        .iter()
        .filter_map(|(k, n)| (*n == 0).then_some(*k))
        .collect();
    let mut visited = 0usize;
    while let Some(kind) = ready.pop_first() {
        visited = visited.checked_add(1).ok_or("precedence count overflow")?;
        for after in edges.get(kind).into_iter().flatten() {
            let n = indegree.get_mut(after).ok_or("undeclared edge endpoint")?;
            *n = n.checked_sub(1).ok_or("precedence count underflow")?;
            if *n == 0 {
                ready.insert(after);
            }
        }
    }
    if visited != policy.declared_kinds.len() {
        return Err("precedence graph contains a cycle".into());
    }
    let mut occupied_kinds = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    for (enter, exit) in &policy.occupancy_pairs {
        if !pairs.insert((enter, exit))
            || !occupied_kinds.insert(enter)
            || !occupied_kinds.insert(exit)
            || enter == exit
            || !policy.declared_kinds.contains(enter)
            || !policy.declared_kinds.contains(exit)
        {
            return Err("invalid occupancy pair kinds".into());
        }
    }
    if policy.capacities.values().any(|c| *c == 0) {
        return Err("capacities must be positive".into());
    }
    if policy.window.is_some_and(|(s, e)| s >= e) {
        return Err("observation window must be nonempty [start,end)".into());
    }
    Ok(())
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("missing or blank {key}"))
}
fn nullable_string(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(format!("{key} must be string or null")),
    }
}
fn parse_u128(s: &str) -> Option<u128> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    s.parse().ok()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn stage_path(target: &Path) -> Result<PathBuf, String> {
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("create output parent: {e}"))?;
    let name = target
        .file_name()
        .ok_or("output path has no filename")?
        .to_string_lossy();
    Ok(parent.join(format!(".{name}.{}.stage", std::process::id())))
}
fn staged_writer(path: &Path) -> Result<BufWriter<File>, String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create staged output: {e}"))?;
    Ok(BufWriter::new(file))
}
fn cleanup_stage(a: &Path, b: &Path) {
    let _ = fs::remove_file(a);
    let _ = fs::remove_file(b);
}
struct StageCleanup(PathBuf, PathBuf);
impl Drop for StageCleanup {
    fn drop(&mut self) {
        cleanup_stage(&self.0, &self.1);
    }
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    output: &mut Vec<u8>,
    limit: usize,
) -> io::Result<usize> {
    output.clear();
    let cap = limit
        .checked_add(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "record limit overflow"))?;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(output.len());
        }
        let count = available
            .iter()
            .position(|b| *b == b'\n')
            .map_or(available.len(), |i| i + 1);
        if output.len().checked_add(count).is_none_or(|n| n > cap) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "record exceeds configured byte bound",
            ));
        }
        let complete = available[count - 1] == b'\n';
        output.extend_from_slice(&available[..count]);
        reader.consume(count);
        if complete {
            return Ok(output.len());
        }
    }
}
fn publish_exclusive(stage: &Path, target: &Path) -> Result<(), String> {
    fs::hard_link(stage, target).map_err(|e| format!("publish output exclusively: {e}"))?;
    fs::remove_file(stage).map_err(|e| format!("remove staged output: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "c13-validate-{}-{n}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    fn event(
        case: &str,
        occ: u32,
        kind: &str,
        tick: u128,
        resource: Option<&str>,
        location: Option<&str>,
    ) -> Value {
        json!({"record_type":"trace_event.v1","schema_version":"calibration-v1","dataset_id":"synthetic","mapping_version":"m1","case_key":case,"source_event_key":format!("{case}-{occ}-{kind}-{tick}"),"occurrence":occ,"event_kind":kind,"event_kind_rank":1,"relative_ticks":tick.to_string(),"source_order":tick as u64,"occurrence_time":{"relative_ticks":tick.to_string()},"source_recorded_time":null,"message_created_time":null,"time_lineage":{},"resource_key":resource,"actor_key":null,"location_key":location,"disposition":"realized","knowledge_availability":{"status":"unknown","available_at":null},"raw_event":{},"quality_flags":[]})
    }
    fn policy(mode: ValidationMode) -> ValidationPolicy {
        ValidationPolicy {
            mode,
            declared_kinds: ["arrive", "leave", "begin", "finish"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            required_kinds: vec!["arrive".into()],
            precedence: vec![],
            occupancy_pairs: vec![("begin".into(), "finish".into())],
            capacities: [(("bed".into(), "ward".into()), 1)].into_iter().collect(),
            window: None,
            bounds: ValidationBounds {
                max_cases: 20,
                max_state_entries: 200,
                max_record_bytes: 8192,
            },
        }
    }
    fn write_input(dir: &Path, rows: &[Value]) -> PathBuf {
        let p = dir.join("input.ndjson");
        let mut f = File::create(&p).unwrap();
        for r in rows {
            writeln!(f, "{}", serde_json::to_string(r).unwrap()).unwrap();
        }
        p
    }
    fn run(dir: &Path, rows: &[Value], p: &ValidationPolicy) -> Result<ValidationReport, String> {
        let input = write_input(dir, rows);
        validate_file(
            &input,
            &dir.join("valid.ndjson"),
            &dir.join("quarantine.ndjson"),
            p,
        )
    }

    #[test]
    fn interleaved_cases_and_equal_precedence_are_valid() {
        let dir = temp();
        let mut p = policy(ValidationMode::Quarantine);
        p.precedence = vec![("arrive".into(), "leave".into())];
        let rows = vec![
            event("a", 0, "arrive", 1, None, None),
            event("b", 0, "arrive", 1, None, None),
            event("a", 0, "leave", 2, None, None),
            event("b", 0, "leave", 2, None, None),
        ];
        let r = run(&dir, &rows, &p).unwrap();
        assert_eq!(r.valid_events, 4);
        assert_eq!(r.invalid_cases, 0);
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn reversed_precedence_quarantines_whole_interleaved_case() {
        let dir = temp();
        let mut p = policy(ValidationMode::Quarantine);
        p.precedence = vec![("arrive".into(), "leave".into())];
        let rows = vec![
            event("a", 0, "leave", 1, None, None),
            event("b", 0, "arrive", 1, None, None),
            event("a", 0, "arrive", 2, None, None),
            event("b", 0, "leave", 2, None, None),
        ];
        let r = run(&dir, &rows, &p).unwrap();
        assert_eq!(r.invalid_cases, 1);
        assert_eq!(r.quarantined_events, 2);
        assert_eq!(r.valid_events, 2);
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn touching_intervals_are_feasible_and_overlaps_quarantine_both() {
        let d = temp();
        let p = policy(ValidationMode::Quarantine);
        let touching = vec![
            event("a", 0, "arrive", 0, None, None),
            event("a", 0, "begin", 0, Some("bed"), Some("ward")),
            event("a", 0, "finish", 2, Some("bed"), Some("ward")),
            event("b", 0, "arrive", 2, None, None),
            event("b", 0, "begin", 2, Some("bed"), Some("ward")),
            event("b", 0, "finish", 4, Some("bed"), Some("ward")),
        ];
        let r = run(&d, &touching, &p).unwrap();
        assert!(r.resource_feasible);
        assert_eq!(r.invalid_cases, 0);
        let _ = fs::remove_dir_all(&d);
        let d = temp();
        let overlap = vec![
            event("a", 0, "arrive", 0, None, None),
            event("a", 0, "begin", 0, Some("bed"), Some("ward")),
            event("b", 0, "arrive", 1, None, None),
            event("b", 0, "begin", 1, Some("bed"), Some("ward")),
            event("a", 0, "finish", 3, Some("bed"), Some("ward")),
            event("b", 0, "finish", 4, Some("bed"), Some("ward")),
        ];
        let r = run(&d, &overlap, &p).unwrap();
        assert!(!r.resource_feasible);
        assert_eq!(r.invalid_cases, 2);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn zero_duration_occupancy_is_allowed_and_does_not_consume_capacity() {
        let d = temp();
        let p = policy(ValidationMode::Quarantine);
        let rows = vec![
            event("a", 0, "arrive", 0, None, None),
            event("a", 0, "begin", 2, Some("bed"), Some("ward")),
            event("a", 0, "finish", 2, Some("bed"), Some("ward")),
            event("b", 0, "arrive", 2, None, None),
            event("b", 0, "begin", 2, Some("bed"), Some("ward")),
            event("b", 0, "finish", 3, Some("bed"), Some("ward")),
        ];
        let r = run(&d, &rows, &p).unwrap();
        assert!(r.resource_feasible);
        assert_eq!(r.invalid_cases, 0);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn same_tick_exit_releases_before_c0_earlier_admission() {
        let d = temp();
        let p = policy(ValidationMode::Quarantine);
        // At tick 5 case `a` sorts before `z`; the entering case appears first in
        // C0 presentation order. Capacity semantics must still release `z` first.
        let rows = vec![
            event("z", 0, "arrive", 0, None, None),
            event("z", 0, "begin", 0, Some("bed"), Some("ward")),
            event("a", 0, "arrive", 5, None, None),
            event("a", 0, "begin", 5, Some("bed"), Some("ward")),
            event("z", 0, "finish", 5, Some("bed"), Some("ward")),
            event("a", 0, "finish", 8, Some("bed"), Some("ward")),
        ];
        let r = run(&d, &rows, &p).unwrap();
        assert!(r.resource_feasible);
        assert_eq!(r.invalid_cases, 0);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn strict_never_publishes_and_window_quarantines_whole_case() {
        let d = temp();
        let mut p = policy(ValidationMode::Strict);
        p.window = Some((2, 5));
        let input = write_input(&d, &[event("a", 0, "arrive", 5, None, None)]);
        let valid = d.join("valid");
        let quarantine = d.join("quarantine");
        assert!(validate_file(&input, &valid, &quarantine, &p).is_err());
        assert!(!valid.exists());
        assert!(!quarantine.exists());
        let _ = fs::remove_dir_all(&d);
        let d = temp();
        let mut p = policy(ValidationMode::Quarantine);
        p.window = Some((2, 5));
        let rows = vec![
            event("a", 0, "arrive", 2, None, None),
            event("a", 0, "leave", 5, None, None),
        ];
        let r = run(&d, &rows, &p).unwrap();
        assert_eq!(r.censored_cases, 1);
        assert_eq!(r.quarantined_events, 2);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn bad_policy_caps_cycles_and_oversize_records_reject_without_outputs() {
        let d = temp();
        let mut p = policy(ValidationMode::Quarantine);
        p.precedence = vec![
            ("arrive".into(), "leave".into()),
            ("leave".into(), "arrive".into()),
        ];
        let input = write_input(&d, &[event("a", 0, "arrive", 1, None, None)]);
        assert!(validate_file(&input, &d.join("v"), &d.join("q"), &p).is_err());
        p.precedence.clear();
        p.bounds.max_cases = 0;
        assert!(validate_file(&input, &d.join("v"), &d.join("q"), &p).is_err());
        p.bounds.max_cases = 20;
        p.bounds.max_record_bytes = 8;
        assert!(validate_file(&input, &d.join("v"), &d.join("q"), &p).is_err());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn runtime_case_state_and_partial_line_bounds_fail_closed() {
        let d = temp();
        let mut p = policy(ValidationMode::Quarantine);
        p.bounds.max_cases = 1;
        let input = write_input(
            &d,
            &[
                event("a", 0, "arrive", 1, None, None),
                event("b", 0, "arrive", 2, None, None),
            ],
        );
        assert!(validate_file(&input, &d.join("v"), &d.join("q"), &p).is_err());

        p.bounds.max_cases = 10;
        p.bounds.max_state_entries = 1;
        let input = write_input(&d, &[event("a", 0, "arrive", 1, None, None)]);
        assert!(validate_file(&input, &d.join("v"), &d.join("q"), &p).is_err());

        let partial = d.join("partial.ndjson");
        fs::write(&partial, b"{\"incomplete\":true}").unwrap();
        p.bounds.max_state_entries = 10;
        assert!(validate_file(&partial, &d.join("v"), &d.join("q"), &p).is_err());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn finalization_work_limit_rejects_before_materializing_occurrence_edge_violations() {
        let d = temp();
        let mut p = policy(ValidationMode::Quarantine);
        p.required_kinds = vec!["arrive".into()];
        p.precedence = vec![("begin".into(), "finish".into())];
        p.bounds.max_state_entries = 15;
        let rows: Vec<Value> = (0..6)
            .map(|occ| event("case", occ, "arrive", occ as u128, None, None))
            .collect();
        let input = write_input(&d, &rows);
        let error = validate_file(&input, &d.join("valid"), &d.join("quarantine"), &p)
            .expect_err("policy-comparison work exceeds the explicit bound");
        assert!(error.contains("validation work limit exceeded"));
        assert!(!d.join("valid").exists());
        assert!(!d.join("quarantine").exists());
        let _ = fs::remove_dir_all(d);
    }
}
