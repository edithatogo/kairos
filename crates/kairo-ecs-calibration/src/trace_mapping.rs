//! Private experimental consumer for c11.synthetic-map-v1 raw JSON requests.
use super::trace_order::{EventKindRank, TraceOrderKeyV1};
use chrono::{DateTime, Datelike, FixedOffset, SecondsFormat, Utc};
use kairo_ecs_arrow::trace_time::{
    normalize_occurrence, normalize_timestamp, relative_ticks, ClockRole, Lineage,
    NormalizedTimestamp, SourcePrecision, TemporalError, TimeValue, TimestampInput,
    TraceTimeResult,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// Consumes the raw request only; expected fixtures are used by tests outside this function.
pub(crate) fn map(request: &Value) -> Value {
    let result = match map_inner(request) {
        Ok(v) => v,
        Err(reason) => failure_result(request, &reason),
    };
    #[cfg(test)]
    capture_actual(request, &result).expect("C1 mapper actual-output capture must be durable");
    result
}
fn failure_result(request: &Value, detail: &str) -> Value {
    let rows = request.get("rows").and_then(Value::as_array);
    let source_rows = rows.map_or(0, Vec::len);
    let bindings = request.get("event_bindings").and_then(Value::as_array);
    let mut candidates = 0usize;
    if let (Some(rows), Some(bindings)) = (rows, bindings) {
        if request.get("shape").and_then(Value::as_str) == Some("wide") {
            candidates = rows.len().saturating_mul(bindings.len())
        } else if request.get("shape").and_then(Value::as_str) == Some("long") {
            if let Some(kf) = request.get("event_kind_field").and_then(Value::as_str) {
                for row in rows {
                    if let Some(k) = row.get(kf).and_then(Value::as_str) {
                        candidates = candidates.saturating_add(
                            bindings
                                .iter()
                                .filter(|b| b.get("kind").and_then(Value::as_str) == Some(k))
                                .count(),
                        )
                    }
                }
            }
        }
    }
    json!({"classification":"failed","records":[],"outcomes":[],"diagnostics":[{"reason":"invalid_mapping","detail":detail}],"accounting":{"source_rows":source_rows,"candidate_units":candidates,"accepted_units":0,"excluded_units":0,"failed_units":candidates,"unresolved_units":0,"candidate_conservation":true,"cohort_denominator":source_rows,"missing_triage":0}})
}

fn map_inner(req: &Value) -> Result<Value, String> {
    let obj = req.as_object().ok_or("invalid envelope")?;
    let s = |k: &str| {
        obj.get(k)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("missing or invalid {k}"))
    };
    if s("profile_version")? != "c11.synthetic-map-v1" {
        return Err("unsupported profile".into());
    }
    for k in [
        "dataset_id",
        "mapping_version",
        "case_key_field",
        "source_family",
    ] {
        if s(k)?.trim().is_empty() {
            return Err(format!("blank {k}"));
        }
    }
    let shape = s("shape")?;
    if shape != "wide" && shape != "long" {
        return Err("invalid shape".into());
    }
    let origin = parse_rfc(s("origin_utc")?).map_err(|e| format!("invalid origin: {e}"))?;
    let origin_ns = ns(origin)?;
    let bindings = obj
        .get("event_bindings")
        .and_then(Value::as_array)
        .ok_or("invalid event_bindings")?;
    if bindings.is_empty() {
        return Err("empty event_bindings".into());
    }
    let mut bs = Vec::new();
    let mut kinds = BTreeSet::new();
    let mut ranks = BTreeSet::new();
    for v in bindings {
        let b = v.as_object().ok_or("binding must be object")?;
        let strf = |k: &str| {
            b.get(k)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("binding missing {k}"))
        };
        let kind = strf("kind")?.to_owned();
        let rank = strf("rank")?.to_owned();
        if kind.trim().is_empty() || !canon_uint(&rank) {
            return Err("invalid kind/rank".into());
        }
        if !kinds.insert(kind.clone()) || !ranks.insert(rank.clone()) {
            return Err("duplicate kind/rank".into());
        }
        let source_event_type = optstr(b, "source_event_type");
        let f = Binding {
            kind,
            rank,
            occurrence_index: b
                .get("occurrence_index")
                .and_then(Value::as_u64)
                .filter(|v| *v <= u32::MAX as u64)
                .ok_or("invalid occurrence_index")? as u32,
            occurrence: strf("occurrence_field")?.into(),
            key: strf("key_field")?.into(),
            order: strf("order_field")?.into(),
            recorded: optstr(b, "recorded_field"),
            message: optstr(b, "message_field"),
            knowledge: optstr(b, "knowledge_field"),
            source_event_type,
            movement: optstr(b, "movement_evidence_field"),
            interval: optstr(b, "interval_field"),
        };
        bs.push(f);
    }
    let rows = obj
        .get("rows")
        .and_then(Value::as_array)
        .ok_or("invalid rows")?;
    let mut candidates = Vec::new();
    for (ri, row) in rows.iter().enumerate() {
        if !row.is_object() {
            return Err("row must be object".into());
        }
        let selected: Vec<&Binding> = if shape == "wide" {
            bs.iter().collect()
        } else {
            let f = s("event_kind_field")?;
            let k = row
                .get(f)
                .and_then(Value::as_str)
                .ok_or("missing long event_kind")?;
            vec![bs
                .iter()
                .find(|b| b.kind == k)
                .ok_or("unknown long event_kind")?]
        };
        for b in selected {
            candidates.push((ri, row, b));
        }
    }
    let n = candidates.len();
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    // Identity errors are checked as a dataset class before any clock processing.
    for (ri, row, b) in &candidates {
        let k = row[&b.key].as_str();
        if k.is_none_or(|s| s.trim().is_empty()) {
            issues.push(json!({"reason":"missing_source_event_key","row_index":ri,"event_kind":b.kind,"raw_row":row}));
        }
    }
    if issues.is_empty() {
        for (ri, row, b) in &candidates {
            let k = row[&b.key].as_str().unwrap();
            if !seen.insert(k.to_owned()) {
                issues.push(json!({"reason":"duplicate_source_event_key","row_index":ri,"event_kind":b.kind,"raw_row":row}));
            }
        }
    }
    if issues.is_empty() {
        for (ri, row, b) in &candidates {
            let ro = row.as_object().unwrap();
            if ro
                .get(s("case_key_field")?)
                .and_then(Value::as_str)
                .is_none_or(|v| v.trim().is_empty())
            {
                issues.push(json!({"reason":"invalid_mapping","detail":"missing or blank case_key","row_index":ri,"raw_row":row}));
            }
            if ro.get(&b.order).and_then(Value::as_u64).is_none() {
                issues.push(json!({"reason":"invalid_mapping","detail":"source_order must be UInt64","row_index":ri,"raw_row":row}));
            }
        }
    }
    if !issues.is_empty() {
        return Ok(result(
            rows.len(),
            n,
            0,
            0,
            n,
            0,
            vec![],
            vec![],
            issues,
            "failed",
        ));
    }
    let mut ordered: Vec<(TraceOrderKeyV1, Value)> = Vec::new();
    let mut exclusions = Vec::new();
    let mut outcomes = Vec::new();
    let mut diagnostics = Vec::new();
    let (mut accepted, mut excluded, mut unresolved) = (0, 0, 0);
    for (ri, row, b) in candidates {
        let ro = row.as_object().unwrap();
        let case = ro
            .get(s("case_key_field")?)
            .and_then(Value::as_str)
            .unwrap();
        let key = ro[&b.key].as_str().unwrap();
        let order = ro.get(&b.order).and_then(Value::as_u64).unwrap();
        let source_type = obj
            .get("source_event_type_field")
            .and_then(Value::as_str)
            .and_then(|f| row.get(f))
            .and_then(Value::as_str)
            .or(b.source_event_type.as_deref());
        if let Some(g) = semantic_guard(s("source_family")?, b, source_type, row)
            .or_else(|| {
                source_type
                    .filter(|v| v.trim().is_empty())
                    .map(|_| "missing explicit source_event_type")
            })
            .or_else(|| {
                source_type
                    .is_none()
                    .then_some("missing explicit source_event_type")
            })
        {
            unresolved += 1;
            diagnostics.push(json!({"reason":"unverified_mapping","detail":g,"row_index":ri,"raw_row":row,"mapping_status":"unverified"}));
            continue;
        }
        if let Some(f) = &b.interval {
            if let Some(iv) = ro.get(f).and_then(Value::as_object) {
                if let Some(status) = interval_status(iv) {
                    diagnostics.push(json!({"reason":status,"row_index":ri,"event_kind":b.kind,"raw_interval":iv}));
                }
            }
        }
        let mut quality = Vec::new();
        let mut rawtimes = Map::new();
        let mut lineage = Map::new();
        let occurrence = ro.get(&b.occurrence).cloned().unwrap_or(Value::Null);
        rawtimes.insert("occurrence".into(), occurrence.clone());
        if let Some(l) = occurrence.get("lineage") {
            lineage.insert("occurrence".into(), l.clone());
        }
        for (label, field) in [
            ("source_recorded_time", &b.recorded),
            ("message_created_time", &b.message),
            ("available_at", &b.knowledge),
        ] {
            let mut raw = field
                .as_ref()
                .and_then(|f| ro.get(f))
                .cloned()
                .unwrap_or(Value::Null);
            if label == "available_at" {
                raw = raw.get("available_at").cloned().unwrap_or(Value::Null);
            }
            rawtimes.insert(label.into(), raw.clone());
            if let Some(l) = raw.get("lineage") {
                lineage.insert(label.into(), l.clone());
            }
        }
        let occ = match normalized_clock(
            &occurrence,
            origin_ns,
            ClockRole::Occurrence,
            s("mapping_version")?,
        ) {
            Ok(Some(v)) => v,
            Ok(None) => {
                excluded += 1;
                exclusions.push(exclusion(
                    req,
                    row,
                    b,
                    case,
                    key,
                    order,
                    "missing_required_time",
                    "occurrence time is absent",
                    rawtimes,
                    lineage,
                ));
                continue;
            }
            Err(e) => {
                excluded += 1;
                exclusions.push(exclusion(
                    req, row, b, case, key, order, e.code, &e.detail, rawtimes, lineage,
                ));
                continue;
            }
        };
        lineage.insert("occurrence".into(), occ.lineage.clone());
        let mut optional = Map::new();
        let mut failure: Option<ClockFailure> = None;
        for (label, field, role) in [
            (
                "source_recorded_time",
                &b.recorded,
                ClockRole::SourceRecorded,
            ),
            (
                "message_created_time",
                &b.message,
                ClockRole::MessageCreated,
            ),
            ("available_at", &b.knowledge, ClockRole::KnowledgeAvailable),
        ] {
            let raw = field
                .as_ref()
                .and_then(|f| ro.get(f))
                .map(|v| {
                    if label == "available_at" {
                        v.get("available_at").cloned().unwrap_or(Value::Null)
                    } else {
                        v.clone()
                    }
                })
                .unwrap_or(Value::Null);
            if raw.is_null() {
                optional.insert(label.into(), Value::Null);
                quality.push(format!("optional_clock_absent:{label}"));
                continue;
            }
            match normalized_clock(&raw, origin_ns, role, s("mapping_version")?) {
                Ok(Some(v)) => {
                    lineage.insert(label.into(), v.lineage.clone());
                    optional.insert(
                        label.into(),
                        json!({"time_value":v.json(),"lineage":v.lineage}),
                    );
                }
                Ok(None) => {
                    optional.insert(label.into(), Value::Null);
                }
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        if let Some(e) = failure {
            excluded += 1;
            exclusions.push(exclusion(
                req, row, b, case, key, order, e.code, &e.detail, rawtimes, lineage,
            ));
            continue;
        }
        let before_unresolved = unresolved;
        let knowledge = knowledge_value(
            ro,
            b,
            obj,
            &optional,
            &mut unresolved,
            &mut diagnostics,
            ri,
            row,
        )?;
        if unresolved > before_unresolved {
            continue;
        }
        let raw_event = raw_event(req, row, b, obj);
        let mv = s("mapping_version")?;
        let time_lineage = json!({"occurrence":occ.lineage,"source_recorded":optional.get("source_recorded_time").and_then(|v|v.get("lineage")).cloned().unwrap_or_else(||unknown_lineage(mv)),"message_created":optional.get("message_created_time").and_then(|v|v.get("lineage")).cloned().unwrap_or_else(||unknown_lineage(mv))});
        let record = json!({"record_type":"trace_event.v1","schema_version":"calibration-v1","dataset_id":s("dataset_id")?,"mapping_version":mv,"case_key":case,"source_event_key":key,"occurrence":b.occurrence_index,"event_kind":b.kind,"event_kind_rank":rank_number(&b.rank),"relative_ticks":occ.ticks.to_string(),"source_order":order,"occurrence_time":occ.json(),"source_recorded_time":optional.get("source_recorded_time").and_then(|v|v.get("time_value")).cloned().unwrap_or(Value::Null),"message_created_time":optional.get("message_created_time").and_then(|v|v.get("time_value")).cloned().unwrap_or(Value::Null),"time_lineage":time_lineage,"resource_key":fieldval(ro,obj,"resource_key_field"),"actor_key":fieldval(ro,obj,"actor_key_field"),"location_key":fieldval(ro,obj,"location_key_field"),"disposition":disposition(ro,obj),"knowledge_availability":knowledge,"raw_event":raw_event,"quality_flags":quality});
        let rank = EventKindRank::from_canonical_decimal(&b.rank).map_err(|_| "invalid rank")?;
        ordered.push((
            TraceOrderKeyV1 {
                relative_ticks: occ.ticks,
                case_key: case.into(),
                occurrence: b.occurrence_index,
                event_kind_rank: rank,
                source_event_key: key.into(),
                source_order: order,
            },
            record,
        ));
        accepted += 1;
    }
    // Outcomes are a distinct case/endpoint population, emitted once per raw source row.
    for row in rows {
        let ro = row.as_object().unwrap();
        let case = ro
            .get(s("case_key_field")?)
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .ok_or("missing/blank outcome case key")?;
        if let Some(out) = outcome(ro, req, case)? {
            outcomes.push(out);
        }
    }
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    let mut records: Vec<Value> = ordered.into_iter().map(|v| v.1).collect();
    records.extend(exclusions);
    let class = if excluded == 0 && unresolved == 0 {
        "accepted"
    } else {
        "partially_excluded"
    };
    let mut out = result(
        rows.len(),
        n,
        accepted,
        excluded,
        0,
        unresolved,
        records,
        outcomes,
        diagnostics,
        class,
    );
    let cohort_field = obj.get("cohort_field").and_then(Value::as_str);
    let triage_field = obj.get("triage_field").and_then(Value::as_str);
    let denominator = cohort_field
        .map(|f| {
            rows.iter()
                .filter(|r| r.get(f) == Some(&Value::Bool(true)))
                .count()
        })
        .unwrap_or(rows.len());
    let missing = match (cohort_field, triage_field) {
        (Some(c), Some(t)) => rows
            .iter()
            .filter(|r| r.get(c) == Some(&Value::Bool(true)) && r.get(t).is_none_or(Value::is_null))
            .count(),
        _ => 0,
    };
    out["accounting"]["cohort_denominator"] = json!(denominator);
    out["accounting"]["missing_triage"] = json!(missing);
    Ok(out)
}

#[derive(Clone)]
struct Binding {
    kind: String,
    rank: String,
    occurrence_index: u32,
    occurrence: String,
    key: String,
    order: String,
    recorded: Option<String>,
    message: Option<String>,
    knowledge: Option<String>,
    source_event_type: Option<String>,
    movement: Option<String>,
    interval: Option<String>,
}
fn rank_number(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or_else(|_| json!(s))
}
fn optstr(o: &Map<String, Value>, k: &str) -> Option<String> {
    o.get(k).and_then(Value::as_str).map(str::to_owned)
}
fn canon_uint(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
}
fn ns(d: DateTime<Utc>) -> Result<i128, String> {
    (d.timestamp() as i128)
        .checked_mul(1_000_000_000)
        .and_then(|x| x.checked_add(d.timestamp_subsec_nanos() as i128))
        .ok_or("origin arithmetic overflow".into())
}
fn parse_rfc(s: &str) -> Result<DateTime<Utc>, String> {
    if !s.contains('T') {
        return Err("date-only input".into());
    }
    let b = s.as_bytes();
    if !s.is_ascii()
        || b.len() < 20
        || b.get(4) != Some(&b'-')
        || b.get(7) != Some(&b'-')
        || b.get(10) != Some(&b'T')
        || b.get(13) != Some(&b':')
        || b.get(16) != Some(&b':')
        || !b[..4]
            .iter()
            .chain(&b[5..7])
            .chain(&b[8..10])
            .chain(&b[11..13])
            .chain(&b[14..16])
            .chain(&b[17..19])
            .all(u8::is_ascii_digit)
    {
        return Err("noncanonical RFC3339 spelling".into());
    }
    if b[17..19] == *b"60" {
        return Err("leap second unsupported".into());
    }
    let mut i = 19;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1
        }
        if i == start {
            return Err("empty fractional seconds".into());
        }
        if i - start > 9 && b[start + 9..i].iter().any(|v| *v != b'0') {
            return Err("subnanosecond precision".into());
        }
    }
    let zone = &s[i..];
    if zone != "Z" {
        if zone.len() != 6
            || !matches!(zone.as_bytes()[0], b'+' | b'-')
            || zone.as_bytes()[3] != b':'
            || !zone.as_bytes()[1..3]
                .iter()
                .chain(&zone.as_bytes()[4..6])
                .all(u8::is_ascii_digit)
        {
            return Err("explicit offset required".into());
        }
        if zone == "-00:00" {
            return Err("unknown offset".into());
        }
    }
    let d = DateTime::<FixedOffset>::parse_from_rfc3339(s)
        .map_err(|_| "invalid RFC3339 calendar or clock".to_string())?
        .with_timezone(&Utc);
    if !(1..=9999).contains(&d.year()) {
        return Err("calendar range".into());
    }
    Ok(d)
}
fn precision(s: &str) -> Option<SourcePrecision> {
    Some(match s {
        "nanosecond" => SourcePrecision::Nanosecond,
        "microsecond" => SourcePrecision::Microsecond,
        "millisecond" => SourcePrecision::Millisecond,
        "second" => SourcePrecision::Second,
        "minute" => SourcePrecision::Minute,
        "other" => SourcePrecision::Coarse,
        "unknown" => SourcePrecision::Unknown,
        _ => return None,
    })
}
fn lineage(v: &Value, mapping_version: &str) -> Result<(Lineage, Value), String> {
    let status = v
        .get("status")
        .and_then(Value::as_str)
        .ok_or("missing lineage status")?;
    if v.get("mapping_version")
        .and_then(Value::as_str)
        .is_some_and(|m| m != mapping_version)
    {
        return Err("lineage mapping_version mismatch".into());
    }
    for field in ["evidence_ref", "derivation"] {
        if v.get(field)
            .is_some_and(|x| !x.is_null() && x.as_str().is_none())
        {
            return Err(format!("lineage {field} must be string or null"));
        }
    }
    let l = match status {
        "observed" => Lineage::Observed,
        "derived" => Lineage::Derived,
        "defaulted" => Lineage::Defaulted,
        "unknown" => Lineage::Unknown,
        _ => return Err("invalid lineage status".into()),
    };
    Ok((
        l,
        json!({"status":status,"mapping_version":mapping_version,"evidence_ref":v.get("evidence_ref").cloned().unwrap_or(Value::Null),"derivation":v.get("derivation").cloned().unwrap_or(Value::Null)}),
    ))
}
fn unknown_lineage(mapping_version: &str) -> Value {
    json!({"status":"unknown","mapping_version":mapping_version,"evidence_ref":null,"derivation":null})
}
fn normalized_clock(
    v: &Value,
    origin: i128,
    role: ClockRole,
    mapping_version: &str,
) -> Result<Option<Norm>, ClockFailure> {
    if v.is_null() {
        return Ok(None);
    }
    let o = v
        .as_object()
        .ok_or(ClockFailure::new("invalid_mapping", "clock must be object"))?;
    let raw = o.get("raw").cloned().unwrap_or(Value::Null);
    if raw.as_str().is_none_or(str::is_empty) {
        return Err(ClockFailure::new(
            "invalid_mapping",
            "raw clock value must be a nonempty string",
        ));
    }
    let precs = o
        .get("precision")
        .and_then(Value::as_str)
        .ok_or(ClockFailure::new(
            "invalid_mapping",
            "missing declared precision",
        ))?;
    let p = precision(precs).ok_or(ClockFailure::new(
        "invalid_mapping",
        "unsupported precision token",
    ))?;
    let (lin, lineage_json) = lineage(
        o.get("lineage")
            .ok_or(ClockFailure::new("invalid_mapping", "missing lineage"))?,
        mapping_version,
    )
    .map_err(|e| ClockFailure::new("invalid_mapping", e))?;
    let rep = o
        .get("representation")
        .and_then(Value::as_str)
        .unwrap_or("");
    let value = match rep {
        "RFC3339" => {
            if p == SourcePrecision::Minute
                || p == SourcePrecision::Coarse
                || p == SourcePrecision::Unknown
            {
                return Err(ClockFailure::new(
                    "date_only_or_coarse_precision",
                    "raw point clock precision is coarse",
                ));
            }
            let s = raw.as_str().ok_or(ClockFailure::new(
                "invalid_mapping",
                "RFC3339 raw must be string",
            ))?;
            let d = parse_rfc(s).map_err(|e| {
                ClockFailure::new(
                    if e.contains("subnanosecond") {
                        "sub_nanosecond"
                    } else if e.contains("offset") {
                        "missing_timezone"
                    } else if e.contains("range") {
                        "overflow"
                    } else if e.contains("date-only") {
                        "date_only_or_coarse_precision"
                    } else {
                        "invalid_mapping"
                    },
                    e,
                )
            })?;
            TimeValue::ResolvedUtc(ns(d).map_err(|e| ClockFailure::new("overflow", e))?)
        }
        "relative_integer" => {
            if p == SourcePrecision::Minute
                || p == SourcePrecision::Coarse
                || p == SourcePrecision::Unknown
            {
                return Err(ClockFailure::new(
                    "date_only_or_coarse_precision",
                    "integer clock precision is coarse",
                ));
            }
            let s = raw.as_str().ok_or(ClockFailure::new(
                "invalid_mapping",
                "integer raw must be canonical string",
            ))?;
            if s.is_empty()
                || s.trim() != s
                || s.starts_with('+')
                || (s.starts_with('0') && s.len() > 1)
                || s.starts_with("-0")
            {
                return Err(ClockFailure::new(
                    "invalid_mapping",
                    "noncanonical signed integer",
                ));
            }
            let n = s
                .parse::<i128>()
                .map_err(|_| ClockFailure::new("overflow", "integer outside i128"))?;
            let mul = match o.get("unit").and_then(Value::as_str).unwrap_or("") {
                "s" => 1_000_000_000i128,
                "ms" => 1_000_000,
                "us" => 1_000,
                "ns" => 1,
                _ => return Err(ClockFailure::new("invalid_mapping", "invalid integer unit")),
            };
            let delta = n.checked_mul(mul).ok_or(ClockFailure::new(
                "overflow",
                "unit multiplication overflow",
            ))?;
            let utc = origin
                .checked_add(delta)
                .ok_or(ClockFailure::new("overflow", "origin addition overflow"))?;
            TimeValue::ResolvedUtc(utc)
        }
        "classified" => {
            if o.get("classification_provenance").and_then(Value::as_str)
                != Some("synthetic_classifier_control")
            {
                return Err(ClockFailure::new(
                    "invalid_mapping",
                    "classified clock lacks explicit synthetic classifier provenance",
                ));
            }
            match o
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("")
            {
                "fold" => TimeValue::AmbiguousLocal,
                "gap" => TimeValue::NonexistentLocal,
                _ => TimeValue::UnresolvedLocal,
            }
        }
        _ => {
            return Err(ClockFailure::new(
                "invalid_mapping",
                "unsupported representation",
            ))
        }
    };
    let lineage_raw = lineage_json;
    let ti = TimestampInput {
        role,
        lineage: Some(lin),
        precision: p,
        value,
    };
    let mut normalized = match role {
        ClockRole::Occurrence => match normalize_occurrence(origin, Some(ti)) {
            TraceTimeResult::Accepted(t) => Norm::from_occ(t, raw.clone(), precs),
            TraceTimeResult::Excluded(e) => return Err(ClockFailure::from(e)),
        },
        _ => match normalize_timestamp(origin, ti) {
            TraceTimeResult::Accepted(t) => Norm::from_time(t, raw.clone(), precs),
            TraceTimeResult::Excluded(e) => return Err(ClockFailure::from(e)),
        },
    };
    let dt = utc_text(normalized.utc_ns).ok_or(ClockFailure::new(
        "overflow",
        "normalized timestamp outside calendar range",
    ))?;
    let _ = relative_ticks(origin, normalized.utc_ns).map_err(ClockFailure::from)?;
    normalized.utc = dt;
    normalized.lineage = lineage_raw;
    normalized.offset = if rep == "RFC3339" {
        offset_zone(&raw)
    } else {
        Value::Null
    };
    Ok(Some(normalized))
}
#[derive(Clone)]
struct Norm {
    utc: String,
    raw: Value,
    precision: String,
    lineage: Value,
    utc_ns: i128,
    ticks: u128,
    offset: Value,
}
impl Norm {
    fn from_occ(t: kairo_ecs_arrow::trace_time::NormalizedOccurrence, raw: Value, p: &str) -> Self {
        Self {
            utc: String::new(),
            raw,
            precision: p.into(),
            lineage: json!({"status":lineage_name(t.lineage)}),
            utc_ns: t.utc_nanoseconds,
            ticks: t.relative_ticks,
            offset: Value::Null,
        }
    }
    fn from_time(t: NormalizedTimestamp, raw: Value, p: &str) -> Self {
        Self {
            utc: String::new(),
            raw,
            precision: p.into(),
            lineage: json!({"status":lineage_name(t.lineage)}),
            utc_ns: t.utc_nanoseconds,
            ticks: t.relative_ticks,
            offset: Value::Null,
        }
    }
    fn json(&self) -> Value {
        json!({"raw":self.raw,"source_precision":self.precision,"source_offset_or_zone":self.offset,"utc":self.utc,"relative_ticks":self.ticks.to_string(),"tick_resolution":"1ns","lineage":self.lineage})
    }
}
fn lineage_name(l: Lineage) -> &'static str {
    match l {
        Lineage::Observed => "observed",
        Lineage::Derived => "derived",
        Lineage::Defaulted => "defaulted",
        Lineage::Unknown => "unknown",
    }
}
fn utc_text(ns: i128) -> Option<String> {
    let sec = i64::try_from(ns.div_euclid(1_000_000_000)).ok()?;
    let n = ns.rem_euclid(1_000_000_000) as u32;
    let dt = DateTime::<Utc>::from_timestamp(sec, n)?;
    if !(1..=9999).contains(&dt.year()) {
        return None;
    }
    Some(dt.to_rfc3339_opts(SecondsFormat::AutoSi, true))
}
fn offset_zone(raw: &Value) -> Value {
    let Some(s) = raw.as_str() else {
        return Value::Null;
    };
    if s.ends_with('Z') {
        return json!("Z");
    }
    let Some(i) = s[11..].find(['+', '-']).map(|n| n + 11) else {
        return Value::Null;
    };
    json!(&s[i..])
}
struct ClockFailure {
    code: &'static str,
    detail: String,
}
impl ClockFailure {
    fn new(c: &'static str, d: impl Into<String>) -> Self {
        Self {
            code: c,
            detail: d.into(),
        }
    }
    fn from(e: TemporalError) -> Self {
        let c = match e {
            TemporalError::PreOrigin => "pre_origin",
            TemporalError::Overflow => "overflow",
            TemporalError::MissingOccurrence => "missing_required_time",
            TemporalError::UnresolvedLocal => "other",
            TemporalError::AmbiguousLocal => "ambiguous_dst_fold",
            TemporalError::NonexistentLocal => "nonexistent_dst_gap",
            TemporalError::CoarsePrecision => "date_only_or_coarse_precision",
            TemporalError::SubNanosecond => "sub_nanosecond",
            TemporalError::ClockRoleMismatch => "invalid_mapping",
            TemporalError::UnsupportedMovement => "invalid_mapping",
            TemporalError::MissingLineage => "invalid_mapping",
            TemporalError::ReversedInterval => "invalid_mapping",
        };
        Self::new(c, e.to_string())
    }
}
fn semantic_guard(
    src: &str,
    b: &Binding,
    source_type: Option<&str>,
    row: &Value,
) -> Option<&'static str> {
    if src == "FHIR_AU" && b.recorded.as_deref() == Some("meta.lastUpdated") {
        return Some("meta.lastUpdated cannot establish source-recorded time");
    }
    if src == "HL7_V2" && b.occurrence == "MSH-7" {
        return Some("MSH-7 cannot establish occurrence time");
    }
    if ["ADT", "HL7_V2"].contains(&src)
        && matches!(b.kind.as_str(), "physical_departure" | "bed_entered")
        && source_type == Some("ADT^A08")
        && b.movement
            .as_ref()
            .and_then(|f| row.get(f))
            .is_none_or(Value::is_null)
    {
        return Some("ADT^A08 alone cannot establish physical movement");
    }
    if src == "OMOP" && b.occurrence == "visit_end_datetime" {
        let lin = row
            .get(&b.occurrence)
            .and_then(|v| v.get("lineage"))
            .and_then(Value::as_object);
        if lin.is_none_or(|o| {
            o.get("evidence_ref")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
                && o.get("derivation")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
        }) {
            return Some("OMOP visit end lacks physical-departure lineage");
        }
    }
    None
}
fn interval_status(iv: &Map<String, Value>) -> Option<&'static str> {
    let start = canon_u128(iv.get("start_ticks")?.as_str()?)?;
    let ev = iv.get("end_ticks")?;
    if ev.is_null() {
        return Some("open_location_interval");
    }
    let end = canon_u128(ev.as_str()?)?;
    match kairo_ecs_arrow::trace_time::validate_location_interval(
        start,
        Some(end),
        kairo_ecs_arrow::trace_time::IntervalRule::OpenOrForward,
    ) {
        TraceTimeResult::Accepted(()) => None,
        TraceTimeResult::Excluded(_) => Some("reversed_interval"),
    }
}
fn canon_u128(s: &str) -> Option<u128> {
    if !canon_uint(s) {
        return None;
    }
    s.parse().ok()
}
fn raw_event(req: &Value, row: &Value, b: &Binding, obj: &Map<String, Value>) -> Value {
    let idfield = obj.get("source_record_id_field").and_then(Value::as_str);
    json!({"source_family":req["source_family"],"source_event_type":obj.get("source_event_type_field").and_then(Value::as_str).and_then(|f|row.get(f)).and_then(Value::as_str).or(b.source_event_type.as_deref()),"source_record_id":idfield.and_then(|f|row.get(f)).and_then(Value::as_str).map(|v|json!(v)).unwrap_or(Value::Null),"source_fields":row})
}
fn fieldval(row: &Map<String, Value>, req: &Map<String, Value>, fk: &str) -> Value {
    req.get(fk)
        .and_then(Value::as_str)
        .and_then(|f| row.get(f))
        .and_then(Value::as_str)
        .map(|v| json!(v))
        .unwrap_or(Value::Null)
}
fn disposition(row: &Map<String, Value>, req: &Map<String, Value>) -> Value {
    match fieldval(row, req, "disposition_field").as_str() {
        Some("provisional") => json!("provisional"),
        Some("realized") => json!("realized"),
        _ => json!("unknown"),
    }
}
fn exclusion(
    req: &Value,
    row: &Value,
    b: &Binding,
    _case: &str,
    key: &str,
    _order: u64,
    reason: &str,
    detail: &str,
    raw_times: Map<String, Value>,
    lineage_map: Map<String, Value>,
) -> Value {
    let reason = match reason {
        "ambiguous_dst_fold" | "ambiguous_local_time" => "ambiguous_dst_fold",
        "nonexistent_dst_gap" | "nonexistent_local_time" => "nonexistent_dst_gap",
        "sub_nanosecond" | "subnanosecond_precision" => "sub_nanosecond",
        "overflow" | "temporal_overflow" => "overflow",
        "pre_origin" => "pre_origin",
        "missing_timezone" => "missing_timezone",
        "missing_required_time" => "missing_required_time",
        "date_only_or_coarse_precision" => "date_only_or_coarse_precision",
        "invalid_mapping" | "missing_lineage" => "invalid_mapping",
        _ => "other",
    };
    let raw = raw_times
        .into_iter()
        .map(|(k, v)| {
            (k, {
                let raw = if v.is_null() {
                    Value::Null
                } else {
                    v.get("raw").cloned().unwrap_or(v)
                };
                if raw.is_null() || raw.is_string() {
                    raw
                } else {
                    Value::Null
                }
            })
        })
        .collect::<Map<String, Value>>();
    let src = lineage_map
        .get("occurrence")
        .or_else(|| lineage_map.values().next());
    let status = src
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .filter(|s| ["observed", "derived", "defaulted", "unknown"].contains(s))
        .unwrap_or("unknown");
    let mv = req["mapping_version"].as_str().unwrap_or("unknown");
    let evidence = src
        .and_then(|v| v.get("evidence_ref"))
        .filter(|v| v.is_null() || v.is_string())
        .cloned()
        .unwrap_or(Value::Null);
    let derivation = src
        .and_then(|v| v.get("derivation"))
        .filter(|v| v.is_null() || v.is_string())
        .cloned()
        .unwrap_or(Value::Null);
    let lin = json!({"status":status,"mapping_version":mv,"evidence_ref":evidence,"derivation":derivation});
    json!({"record_type":"trace_exclusion.v1","schema_version":"calibration-v1","dataset_id":req["dataset_id"],"mapping_version":req["mapping_version"],"source_event_key":key,"raw_event":raw_event(req,row,b,req.as_object().unwrap()),"raw_time_values":raw,"exclusion_reason":reason,"lineage":lin,"detail":detail})
}
fn knowledge_value(
    row: &Map<String, Value>,
    b: &Binding,
    req: &Map<String, Value>,
    optional: &Map<String, Value>,
    unresolved: &mut usize,
    diags: &mut Vec<Value>,
    ri: usize,
    rawrow: &Value,
) -> Result<Value, String> {
    let Some(f) = &b.knowledge else {
        return Ok(json!({"status":"unknown","available_at":null}));
    };
    let Some(k) = row.get(f).and_then(Value::as_object) else {
        return Ok(json!({"status":"unknown","available_at":null}));
    };
    let st = k.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let av = optional
        .get("available_at")
        .and_then(|v| v.get("time_value"))
        .cloned()
        .unwrap_or(Value::Null);
    let cutoff = req
        .get("prediction_cutoff")
        .and_then(Value::as_str)
        .and_then(|s| parse_rfc(s).ok());
    let at = av
        .get("utc")
        .and_then(Value::as_str)
        .and_then(|s| parse_rfc(s).ok());
    let eligible = st == "known" && cutoff.zip(at).is_some_and(|(c, a)| a <= c);
    let problem = if !["known", "not_yet_known", "unknown"].contains(&st) {
        Some("invalid knowledge status")
    } else if st == "known" && av.is_null() {
        Some("known feature has no independently supplied available_at")
    } else if st == "known" && cutoff.is_none() {
        Some("known feature requires explicit prediction_cutoff")
    } else {
        None
    };
    if let Some(detail) = problem {
        *unresolved += 1;
        diags.push(json!({"reason":"invalid_mapping","detail":detail,"row_index":ri,"raw_row":rawrow,"knowledge_eligible":false}));
    } else {
        diags.push(json!({"reason":"knowledge_cutoff_evaluated","knowledge_status":st,"knowledge_eligible":eligible,"row_index":ri,"raw_row":rawrow}));
    }
    Ok(
        json!({"status":if ["known","not_yet_known","unknown"].contains(&st){st}else{"unknown"},"available_at":av}),
    )
}
fn outcome(row: &Map<String, Value>, req: &Value, case: &str) -> Result<Option<Value>, String> {
    let Some(o) = row.get("outcome").and_then(Value::as_object) else {
        return Ok(None);
    };
    let reqv = |k: &str| {
        o.get(k)
            .ok_or_else(|| format!("outcome missing explicit {k}"))
    };
    let risk = reqv("risk_start")?;
    let last = reqv("last_observed")?;
    let event = reqv("event_clock")?;
    let cause = reqv("censor_cause")?;
    let status = o
        .get("censor_status")
        .and_then(Value::as_str)
        .ok_or("outcome missing censor_status")?;
    if ![
        "not_censored",
        "left",
        "right",
        "interval",
        "unknown",
        "missing",
    ]
    .contains(&status)
    {
        return Err("unsupported censor status".into());
    }
    let origin = parse_rfc(req["origin_utc"].as_str().ok_or("missing origin")?)
        .map_err(|e| e.to_string())?;
    let origin = ns(origin).map_err(|e| e.to_string())?;
    let mv = req["mapping_version"].as_str().unwrap_or("");
    let norm = |v: &Value| -> Result<Value, String> {
        if v.is_null() {
            return Ok(Value::Null);
        }
        normalized_clock(v, origin, ClockRole::Occurrence, mv)
            .map_err(|e| e.detail)
            .map(|n| n.map(|x| x.json()).unwrap_or(Value::Null))
    };
    let risk = norm(risk)?;
    let last = norm(last)?;
    let ev = norm(event)?;
    let observed = status == "not_censored";
    if status == "right" && last.is_null() {
        return Err("right-censored outcome requires explicit last_observed".into());
    }
    if status == "right" && !event.is_null() {
        return Err("right-censored outcome cannot assert event_clock".into());
    }
    if observed && ev.is_null() {
        return Err("observed endpoint requires event_clock".into());
    }
    if !observed && !ev.is_null() {
        return Err("censored/unresolved endpoint must not assert event_clock".into());
    }
    let (_lin, lin) = lineage(o.get("lineage").ok_or("outcome missing lineage")?, mv)?;
    let endpoint = o
        .get("endpoint")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or("outcome missing endpoint")?;
    Ok(Some(
        json!({"record_type":"outcome_observation.v1","schema_version":"calibration-v1","dataset_id":req["dataset_id"],"case_key":case,"endpoint":endpoint,"risk_start":risk,"last_observed":last,"event_observed":observed,"event_time":if observed{ev}else{Value::Null},"event_cause":if observed{cause.clone()}else{Value::Null},"censor_status":status,"censor_reason":if status=="right"||status=="left"||status=="interval"{cause.clone()}else{Value::Null},"cluster_ids":o.get("cluster_ids").cloned().unwrap_or(json!([])),"lineage":lin}),
    ))
}
#[cfg(test)]
fn capture_actual(request: &Value, result: &Value) -> std::io::Result<()> {
    use std::{
        collections::BTreeMap,
        fs,
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };
    static CAPTURE: OnceLock<Mutex<BTreeMap<Vec<u8>, (Value, Value)>>> = OnceLock::new();
    let Ok(path) = std::env::var("KAIROS_C11_MAPPER_OUTFILE") else {
        return Ok(());
    };
    let key = serde_json::to_vec(request).map_err(std::io::Error::other)?;
    let capture = CAPTURE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut guard = capture
        .lock()
        .map_err(|_| std::io::Error::other("capture mutex poisoned"))?;
    guard.insert(key, (request.clone(), result.clone()));
    let cases: Vec<Value> = guard
        .values()
        .map(|(request, result)| json!({"request":request,"result":result}))
        .collect();
    let bytes = serde_json::to_vec_pretty(&cases).map_err(std::io::Error::other)?;
    let target = PathBuf::from(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?
    }
    let mut temp = target.clone();
    temp.set_extension("json.tmp");
    fs::write(&temp, bytes)?;
    fs::rename(temp, target)?;
    Ok(())
}
fn result(
    rows: usize,
    candidates: usize,
    accepted: usize,
    excluded: usize,
    failed: usize,
    unresolved: usize,
    records: Vec<Value>,
    outcomes: Vec<Value>,
    diagnostics: Vec<Value>,
    classification: &str,
) -> Value {
    json!({"classification":classification,"records":records,"outcomes":outcomes,"diagnostics":diagnostics,"accounting":{"source_rows":rows,"candidate_units":candidates,"accepted_units":accepted,"excluded_units":excluded,"failed_units":failed,"unresolved_units":unresolved,"candidate_conservation":candidates==accepted+excluded+failed+unresolved,"cohort_denominator":rows,"missing_triage":0}})
}

#[cfg(test)]
mod tests {
    use super::map;
    use kairo_ecs_arrow::trace_time::relative_ticks;
    use serde_json::{json, Value};
    fn req() -> Value {
        json!({"profile_version":"c11.synthetic-map-v1","dataset_id":"d","mapping_version":"m","origin_utc":"2020-01-01T00:00:00Z","case_key_field":"case","source_family":"fixture","shape":"wide","event_bindings":[{"source_event_type":"ARRIVAL","kind":"arrival","rank":"2","occurrence_index":0,"occurrence_field":"at","key_field":"id","order_field":"seq"}],"rows":[{"case":"c","id":"e","seq":1,"at":{"raw":"2020-01-01T00:00:01Z","representation":"RFC3339","precision":"second","lineage":{"status":"observed"}}}]})
    }
    fn clock(s: &str, role: &str) -> Value {
        json!({"raw":s,"representation":"RFC3339","precision":"second","lineage":{"status":"observed","clock":role}})
    }
    #[test]
    fn signed_relative_helper_covers_full_i128_u128_domain() {
        assert_eq!(relative_ticks(i128::MIN, i128::MAX).unwrap(), u128::MAX);
        assert_eq!(relative_ticks(i128::MIN, i128::MIN).unwrap(), 0);
        assert!(relative_ticks(i128::MAX, i128::MIN).is_err());
    }
    #[test]
    fn outputs_full_c0_and_raw_mutation_changes_clock() {
        let a = map(&req());
        let mut r = req();
        r["rows"][0]["at"] = clock("2020-01-01T00:00:02Z", "occurrence");
        let b = map(&r);
        assert_ne!(
            a["records"][0]["relative_ticks"],
            b["records"][0]["relative_ticks"]
        );
        let e = &a["records"][0];
        for f in [
            "occurrence_time",
            "source_recorded_time",
            "message_created_time",
            "source_order",
            "resource_key",
            "actor_key",
            "location_key",
            "disposition",
            "knowledge_availability",
            "raw_event",
            "quality_flags",
        ] {
            assert!(e.get(f).is_some(), "missing {f}")
        }
        assert!(e.get("raw_row").is_none());
        assert_eq!(e["occurrence_time"]["lineage"]["mapping_version"], "m");
        assert_eq!(e["time_lineage"]["source_recorded"]["status"], "unknown");
    }
    #[test]
    fn optional_clocks_are_role_normalized_and_distinct() {
        let mut r = req();
        r["event_bindings"][0]["recorded_field"] = json!("rec");
        r["event_bindings"][0]["message_field"] = json!("msg");
        r["rows"][0]["rec"] = clock("2020-01-01T00:00:02Z", "source_recorded");
        r["rows"][0]["msg"] = clock("2020-01-01T00:00:03Z", "message");
        let e = &map(&r)["records"][0];
        assert_eq!(e["source_recorded_time"]["relative_ticks"], "2000000000");
        assert_eq!(e["message_created_time"]["relative_ticks"], "3000000000");
    }
    #[test]
    fn duplicate_and_missing_keys_fail_dataset() {
        for v in [Value::Null, json!(" ")] {
            let mut r = req();
            r["rows"][0]["id"] = v;
            let m = map(&r);
            assert_eq!(m["accounting"]["failed_units"], 1);
            assert!(m["records"].as_array().unwrap().is_empty())
        }
        let mut r = req();
        let mut row = r["rows"][0].clone();
        row["at"] = clock("2020-01-01T00:00:02Z", "occurrence");
        r["rows"].as_array_mut().unwrap().push(row);
        assert_eq!(map(&r)["accounting"]["failed_units"], 2);
    }
    #[test]
    fn classified_and_subnanosecond_reasons_are_c0_exclusions() {
        let mut r = req();
        r["rows"][0]["at"] = json!({"raw":"2020-11-01 01:30","representation":"classified","classification":"fold","classification_provenance":"synthetic_classifier_control","precision":"second","lineage":{"status":"unknown"}});
        assert_eq!(
            map(&r)["records"][0]["exclusion_reason"],
            "ambiguous_dst_fold"
        );
        r["rows"][0]["at"] = clock("2020-01-01T00:00:00.0000000001Z", "occurrence");
        assert_eq!(map(&r)["records"][0]["exclusion_reason"], "sub_nanosecond");
        r["rows"][0]["at"] = json!({"raw":"2020-03-08 02:30","representation":"classified","classification":"gap","classification_provenance":"synthetic_classifier_control","precision":"second","lineage":{"status":"unknown"}});
        assert_eq!(
            map(&r)["records"][0]["exclusion_reason"],
            "nonexistent_dst_gap"
        );
    }
    #[test]
    fn semantic_guards_emit_no_c0_records() {
        let mut r = req();
        r["source_family"] = json!("FHIR_AU");
        r["event_bindings"][0]["recorded_field"] = json!("meta.lastUpdated");
        let m = map(&r);
        assert_eq!(m["accounting"]["unresolved_units"], 1);
        assert!(m["records"].as_array().unwrap().is_empty());
        assert!(m["diagnostics"][0].get("raw_row").is_some());
    }
    #[test]
    fn four_semantic_guards_are_unresolved_without_fabricated_events() {
        let mut cases = Vec::new();
        let mut f = req();
        f["source_family"] = json!("FHIR_AU");
        f["event_bindings"][0]["recorded_field"] = json!("meta.lastUpdated");
        f["rows"][0]["meta.lastUpdated"] = clock("2020-01-01T00:00:01Z", "recorded");
        cases.push(f);
        let mut h = req();
        h["source_family"] = json!("HL7_V2");
        h["event_bindings"][0]["occurrence_field"] = json!("MSH-7");
        h["rows"][0]["MSH-7"] = clock("2020-01-01T00:00:01Z", "occurrence");
        cases.push(h);
        let mut a = req();
        a["source_family"] = json!("ADT");
        a["source_event_type_field"] = json!("msg_type");
        a["event_bindings"][0]["source_event_type"] = Value::Null;
        a["event_bindings"][0]["kind"] = json!("physical_departure");
        a["event_bindings"][0]["movement_evidence_field"] = json!("movement");
        a["rows"][0]["msg_type"] = json!("ADT^A08");
        cases.push(a);
        let mut o = req();
        o["source_family"] = json!("OMOP");
        o["event_bindings"][0]["occurrence_field"] = json!("visit_end_datetime");
        o["rows"][0]["visit_end_datetime"] = clock("2020-01-01T00:00:01Z", "occurrence");
        cases.push(o);
        for r in cases {
            let m = map(&r);
            assert_eq!(m["accounting"]["unresolved_units"], 1);
            assert!(m["records"].as_array().unwrap().is_empty());
            assert_eq!(m["accounting"]["candidate_conservation"], true);
        }
    }
    #[test]
    fn timestamp_mutations_reject_unknown_offset_naive_and_noncanonical_but_allow_zero_tail() {
        for raw in [
            "2020-01-01T00:00:00-00:00",
            "2020-01-01T00:00:00",
            "2020-1-01T00:00:00Z",
        ] {
            let mut r = req();
            r["rows"][0]["at"] = clock(raw, "occurrence");
            let m = map(&r);
            assert_ne!(m["accounting"]["accepted_units"], 1, "{raw}");
        }
        let mut r = req();
        r["rows"][0]["at"] = clock("2020-01-01T00:00:00.123456789000Z", "occurrence");
        assert_eq!(map(&r)["accounting"]["accepted_units"], 1);
    }
    #[test]
    fn integer_clock_preorigin_and_overflow_are_exclusions() {
        let mut r = req();
        r["rows"][0]["at"] = json!({"raw":"-1","representation":"relative_integer","precision":"second","unit":"s","lineage":{"status":"observed"}});
        assert_eq!(map(&r)["records"][0]["exclusion_reason"], "pre_origin");
        r["rows"][0]["at"] = json!({"raw":"170141183460469231731687303715884105727","representation":"relative_integer","precision":"nanosecond","unit":"s","lineage":{"status":"observed"}});
        assert_eq!(map(&r)["records"][0]["exclusion_reason"], "overflow");
        r["origin_utc"] = json!("9999-12-31T23:59:59Z");
        r["rows"][0]["at"] = json!({"raw":"1","representation":"relative_integer","precision":"nanosecond","unit":"s","lineage":{"status":"observed"}});
        assert_eq!(map(&r)["records"][0]["exclusion_reason"], "overflow");
    }
    #[test]
    fn ranks_above_u64_sort_numerically() {
        let mut r = req();
        r["event_bindings"] = json!([{ "source_event_type":"HIGH","kind":"high","rank":"18446744073709551616","occurrence_index":0,"occurrence_field":"at","key_field":"id1","order_field":"seq"},{"source_event_type":"LOW","kind":"low","rank":"9","occurrence_index":0,"occurrence_field":"at","key_field":"id2","order_field":"seq"}]);
        r["rows"][0]["id1"] = json!("high");
        r["rows"][0]["id2"] = json!("low");
        let m = map(&r);
        assert_eq!(m["records"][0]["event_kind"], "low");
        assert_eq!(m["records"][1]["event_kind"], "high");
    }
    #[test]
    fn knowledge_cutoff_equality_future_and_unknown_are_reported() {
        let mut r = req();
        r["event_bindings"][0]["knowledge_field"] = json!("feature");
        r["prediction_cutoff"] = json!("2020-01-01T00:00:05Z");
        for (status, available, expected) in [
            ("known", "2020-01-01T00:00:05Z", true),
            ("known", "2020-01-01T00:00:06Z", false),
            ("unknown", "2020-01-01T00:00:04Z", false),
        ] {
            r["rows"][0]["feature"] =
                json!({"status":status,"available_at":clock(available,"available")});
            let m = map(&r);
            let d = m["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["reason"] == "knowledge_cutoff_evaluated")
                .unwrap();
            assert_eq!(d["knowledge_eligible"], expected);
        }
    }
    #[test]
    fn observed_task_started_survives_and_future_outcome_cannot_change_knowledge() {
        let mut r = req();
        r["source_family"] = json!("generic");
        r["event_bindings"][0]["source_event_type"] = json!("TASK_STARTED");
        r["event_bindings"][0]["kind"] = json!("task_started");
        r["event_bindings"][0]["knowledge_field"] = json!("feature");
        r["prediction_cutoff"] = json!("2020-01-01T00:00:05Z");
        r["rows"][0]["unrelated_standard_column"] = json!("retained raw evidence");
        r["rows"][0]["feature"] =
            json!({"status":"known","available_at":clock("2020-01-01T00:00:05Z","available")});
        r["rows"][0]["outcome"] = json!({"endpoint":"end","risk_start":clock("2020-01-01T00:00:00Z","risk"),"last_observed":clock("2020-01-01T00:00:05Z","last"),"event_clock":clock("2020-01-01T00:00:10Z","future"),"censor_cause":"end","censor_status":"not_censored","lineage":{"status":"observed"}});
        let a = map(&r);
        let eligible = |v: &Value| {
            v["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["reason"] == "knowledge_cutoff_evaluated")
                .unwrap()["knowledge_eligible"]
                .clone()
        };
        assert_eq!(a["records"][0]["event_kind"], "task_started");
        assert_eq!(
            a["records"][0]["raw_event"]["source_fields"]["unrelated_standard_column"],
            "retained raw evidence"
        );
        assert_eq!(eligible(&a), true);
        r["rows"][0]["outcome"]["event_clock"] = clock("2020-01-01T00:00:20Z", "future");
        let b = map(&r);
        assert_eq!(eligible(&b), true);
    }
    #[test]
    fn missing_occurrence_keeps_available_at_raw_time_c0_safe() {
        let mut r = req();
        r["event_bindings"][0]["knowledge_field"] = json!("feature");
        r["rows"][0]["at"] = Value::Null;
        r["rows"][0]["feature"] =
            json!({"status":"known","available_at":clock("2020-01-01T00:00:02Z","available")});
        let m = map(&r);
        let ex = &m["records"][0];
        assert_eq!(ex["record_type"], "trace_exclusion.v1");
        assert_eq!(ex["raw_time_values"]["occurrence"], Value::Null);
        assert_eq!(
            ex["raw_time_values"]["available_at"],
            "2020-01-01T00:00:02Z"
        );
    }
    #[test]
    fn reversed_open_intervals_and_cohort_denominator_are_diagnostic_only() {
        let mut r = req();
        r["cohort_field"] = json!("cohort");
        r["triage_field"] = json!("triage");
        r["event_bindings"][0]["interval_field"] = json!("span");
        r["rows"][0]["cohort"] = json!(true);
        r["rows"][0]["span"] = json!({"start_ticks":"9","end_ticks":"2"});
        let m = map(&r);
        assert_eq!(m["accounting"]["cohort_denominator"], 1);
        assert_eq!(m["accounting"]["missing_triage"], 1);
        assert!(m["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["reason"] == "reversed_interval"));
        r["rows"][0]["span"] = json!({"start_ticks":"2","end_ticks":null});
        assert!(map(&r)["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["reason"] == "open_location_interval"));
    }
    #[test]
    fn right_censored_outcome_keeps_last_observed_separate() {
        let mut r = req();
        r["rows"][0]["outcome"] = json!({"endpoint":"departure","risk_start":clock("2020-01-01T00:00:00Z","risk"),"last_observed":clock("2020-01-01T00:00:04Z","last"),"event_clock":null,"censor_cause":"administrative_end","censor_status":"right","lineage":{"status":"observed"}});
        let o = map(&r);
        let x = &o["outcomes"][0];
        assert_eq!(x["censor_status"], "right");
        assert_eq!(x["event_observed"], false);
        assert_eq!(x["event_time"], Value::Null);
        assert_eq!(x["last_observed"]["relative_ticks"], "4000000000");
    }
    #[test]
    fn integer_units_are_exactly_equivalent_for_one_second() {
        let mut ticks = Vec::new();
        for (unit, raw) in [
            ("s", "1"),
            ("ms", "1000"),
            ("us", "1000000"),
            ("ns", "1000000000"),
        ] {
            let mut r = req();
            r["rows"][0]["at"] = json!({"raw":raw,"representation":"relative_integer","precision":"second","unit":unit,"lineage":{"status":"observed"}});
            ticks.push(map(&r)["records"][0]["relative_ticks"].clone());
        }
        assert!(ticks.iter().all(|v| v == "1000000000"));
    }
    #[test]
    fn derived_lineage_is_retained_and_bad_lineage_becomes_safe_exclusion() {
        let mut r = req();
        r["rows"][0]["at"]["lineage"] = json!({"status":"derived","evidence_ref":"evidence:source","derivation":"derived from source row"});
        let event = &map(&r)["records"][0];
        assert_eq!(event["occurrence_time"]["lineage"]["status"], "derived");
        assert_eq!(
            event["occurrence_time"]["lineage"]["evidence_ref"],
            "evidence:source"
        );
        assert_eq!(
            event["occurrence_time"]["lineage"]["derivation"],
            "derived from source row"
        );
        let bad = [
            Value::Null,
            json!({"status":"observed","mapping_version":"other"}),
            json!({"status":"derived","evidence_ref":7}),
        ];
        for lineage in bad {
            let mut x = req();
            x["rows"][0]["at"]["lineage"] = lineage;
            let m = map(&x);
            let ex = &m["records"][0];
            assert_eq!(ex["record_type"], "trace_exclusion.v1");
            assert_eq!(ex["exclusion_reason"], "invalid_mapping");
            assert!(ex["lineage"]["status"].is_string());
            assert_eq!(
                ex["raw_event"]["source_fields"]["at"]["lineage"],
                x["rows"][0]["at"]["lineage"]
            );
        }
    }
    #[test]
    fn addition_and_calendar_overflow_preserve_declared_clock_evidence() {
        let mut r = req();
        r["rows"][0]["at"] = json!({"raw":"170141183460469231731687303715884105727","representation":"relative_integer","precision":"nanosecond","unit":"ns","lineage":{"status":"observed"}});
        let add = map(&r);
        assert_eq!(add["records"][0]["exclusion_reason"], "overflow");
        assert_eq!(
            add["records"][0]["raw_event"]["source_fields"]["at"]["unit"],
            "ns"
        );
        assert_eq!(r["origin_utc"], "2020-01-01T00:00:00Z");
        r["origin_utc"] = json!("9999-12-31T23:59:59Z");
        r["rows"][0]["at"] = json!({"raw":"1","representation":"relative_integer","precision":"second","unit":"s","lineage":{"status":"observed"}});
        let cal = map(&r);
        assert_eq!(cal["records"][0]["exclusion_reason"], "overflow");
        assert_eq!(
            cal["records"][0]["raw_event"]["source_fields"]["at"]["raw"],
            "1"
        );
        assert_eq!(r["origin_utc"], "9999-12-31T23:59:59Z");
    }
    #[test]
    fn wide_row_emits_one_outcome_even_if_event_is_excluded() {
        let mut r = req();
        r["event_bindings"] = json!([{ "source_event_type":"A","kind":"a","rank":"1","occurrence_index":0,"occurrence_field":"at","key_field":"id1","order_field":"seq"},{"source_event_type":"B","kind":"b","rank":"2","occurrence_index":0,"occurrence_field":"at2","key_field":"id2","order_field":"seq"}]);
        r["rows"][0]["id1"] = json!("event-a");
        r["rows"][0]["id2"] = json!("event-b");
        r["rows"][0]["at2"] = Value::Null;
        r["rows"][0]["outcome"] = json!({"endpoint":"endpoint","risk_start":clock("2020-01-01T00:00:00Z","risk"),"last_observed":clock("2020-01-01T00:00:02Z","last"),"event_clock":Value::Null,"censor_cause":"admin","censor_status":"right","lineage":{"status":"observed"}});
        let m = map(&r);
        assert_eq!(m["accounting"]["candidate_units"], 2);
        assert_eq!(m["accounting"]["accepted_units"], 1);
        assert_eq!(m["accounting"]["excluded_units"], 1);
        assert_eq!(m["outcomes"].as_array().unwrap().len(), 1);
        assert_eq!(m["accounting"]["candidate_conservation"], true);
    }
    #[test]
    fn identities_keep_exact_unicode_case_and_ties_ignore_input_row_permutation() {
        let mut r = req();
        r["rows"][0]["case"] = json!("Straße");
        r["rows"][0]["id"] = json!("é");
        let mut second = r["rows"][0].clone();
        second["id"] = json!("e\u{301}");
        r["rows"].as_array_mut().unwrap().push(second);
        let mut third = r["rows"][0].clone();
        third["case"] = json!("STRASSE");
        third["id"] = json!("third");
        r["rows"].as_array_mut().unwrap().push(third);
        let a = map(&r);
        r["rows"].as_array_mut().unwrap().reverse();
        let b = map(&r);
        assert_eq!(a["records"], b["records"]);
        assert_eq!(a["accounting"]["accepted_units"], 3);
    }
    #[test]
    fn source_order_and_occurrence_limits_are_validated_before_clocks() {
        let mut r = req();
        r["rows"][0]["seq"] = json!(u64::MAX);
        r["event_bindings"][0]["occurrence_index"] = json!(u32::MAX);
        assert_eq!(map(&r)["accounting"]["accepted_units"], 1);
        r["rows"][0]["seq"] = json!(u64::MAX.to_string() + "0");
        let m = map(&r);
        assert_eq!(m["accounting"]["failed_units"], 1);
        assert_eq!(m["records"].as_array().unwrap().len(), 0);
        r = req();
        r["event_bindings"][0]["occurrence_index"] = json!(u64::from(u32::MAX) + 1);
        let m = map(&r);
        assert_eq!(m["accounting"]["failed_units"], 1);
    }
    #[test]
    fn wide_episode_end_does_not_infer_later_boarding_departure() {
        let mut r = req();
        r["event_bindings"] = json!([{ "source_event_type":"EPISODE_END","kind":"episode_end","rank":"10","occurrence_index":0,"occurrence_field":"end","key_field":"id1","order_field":"seq"},{"source_event_type":"BOARDING","kind":"boarding","rank":"2","occurrence_index":0,"occurrence_field":"board","key_field":"id2","order_field":"seq"}]);
        r["rows"][0]["id1"] = json!("e1");
        r["rows"][0]["id2"] = json!("e2");
        r["rows"][0]["end"] = clock("2020-01-01T00:00:01Z", "occurrence");
        r["rows"][0]["board"] = clock("2020-01-01T00:00:02Z", "occurrence");
        let m = map(&r);
        assert_eq!(m["accounting"]["accepted_units"], 2);
        assert_eq!(m["records"][0]["event_kind"], "episode_end");
        assert_eq!(m["records"][1]["event_kind"], "boarding");
    }
    #[test]
    fn fixture_file_executes_raw_requests_and_exact_goldens_only() {
        let f: Value = serde_json::from_str(include_str!(
            "../../../conformance/c11/mapper-cases-v1.json"
        ))
        .unwrap();
        for c in f["raw_requests"].as_array().unwrap() {
            let got = map(&c["request"]);
            let gold = f["expected_goldens"]
                .as_array()
                .unwrap()
                .iter()
                .find(|g| g["case_id"] == c["case_id"])
                .unwrap();
            assert_eq!(
                got["records"], gold["expected_output"]["records"],
                "{}",
                c["case_id"]
            );
            assert_eq!(
                got["accounting"], gold["expected_output"]["accounting"],
                "{}",
                c["case_id"]
            );
        }
    }
}
