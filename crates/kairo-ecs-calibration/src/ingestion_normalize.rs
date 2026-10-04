//! Bounded private dataset normalization over the established C11 logical mapper.
use super::{
    trace_mapping,
    trace_order::{EventKindRank, TraceOrderKeyV1},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub(crate) max_chunk_rows: usize,
    pub(crate) max_chunk_bytes: usize,
    pub(crate) max_identities: usize,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Counts {
    pub(crate) source_rows: u64,
    pub(crate) candidate_units: u64,
    pub(crate) accepted_units: u64,
    pub(crate) excluded_units: u64,
    pub(crate) failed_units: u64,
    pub(crate) unresolved_units: u64,
    pub(crate) cohort_denominator: u64,
    pub(crate) missing_triage: u64,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct MappedChunk {
    pub(crate) events: Vec<Value>,
    pub(crate) exclusions: Vec<Value>,
    pub(crate) outcomes: Vec<Value>,
    pub(crate) diagnostics: Vec<Value>,
}
pub(crate) struct Normalizer {
    template: Value,
    template_bytes: usize,
    limits: Limits,
    counts: Counts,
    source_keys: BTreeSet<String>,
    outcome_keys: BTreeSet<(String, String)>,
}
impl Normalizer {
    pub(crate) fn new(template: Value, limits: Limits) -> Result<Self, String> {
        if limits.max_chunk_rows == 0 || limits.max_chunk_bytes == 0 || limits.max_identities == 0 {
            return Err("all normalization limits must be nonzero".into());
        }
        let mut t = template;
        let obj = t.as_object_mut().ok_or("template must be an object")?;
        if obj.get("profile_version").and_then(Value::as_str) != Some("c11.synthetic-map-v1") {
            return Err("unsupported profile".into());
        }
        if obj
            .get("rows")
            .and_then(Value::as_array)
            .is_none_or(|r| !r.is_empty())
        {
            return Err("template rows must be an empty array".into());
        }
        validate_envelope(&t)?;
        if trace_mapping::map(&t)["classification"] == "failed" {
            return Err("logical mapper rejected template".into());
        }
        let bytes = serde_json::to_vec(&t).map_err(|e| e.to_string())?.len();
        if bytes > limits.max_chunk_bytes {
            return Err("serialized template exceeds max_chunk_bytes".into());
        }
        Ok(Self {
            template: t,
            template_bytes: bytes,
            limits,
            counts: Counts::default(),
            source_keys: BTreeSet::new(),
            outcome_keys: BTreeSet::new(),
        })
    }
    pub(crate) fn push(&mut self, rows: Vec<Value>) -> Result<MappedChunk, String> {
        if rows.is_empty() {
            return Err("chunk must contain rows".into());
        }
        if rows.len() > self.limits.max_chunk_rows {
            return Err("chunk row limit exceeded".into());
        }
        let mut bytes = self.template_bytes;
        for row in &rows {
            bytes = bytes
                .checked_add(serde_json::to_vec(row).map_err(|e| e.to_string())?.len())
                .ok_or("chunk byte count overflow")?;
        }
        // Include JSON row-array punctuation and envelope insertion overhead conservatively.
        bytes = bytes
            .checked_add(rows.len())
            .and_then(|n| n.checked_add(2))
            .ok_or("chunk byte count overflow")?;
        if bytes > self.limits.max_chunk_bytes {
            return Err("chunk byte limit exceeded".into());
        }
        let fanout = if self.template["shape"] == "wide" {
            self.template["event_bindings"]
                .as_array()
                .ok_or("invalid bindings")?
                .len()
        } else {
            1
        };
        let budget = rows
            .len()
            .checked_mul(fanout)
            .ok_or("candidate fanout overflow")?;
        if budget
            > self
                .limits
                .max_identities
                .saturating_sub(self.source_keys.len())
        {
            return Err("candidate fanout exceeds remaining identity limit".into());
        }
        let preflight = preflight(&self.template, &rows)?;
        if preflight.candidate_count
            > self
                .limits
                .max_identities
                .saturating_sub(self.source_keys.len())
        {
            return Err("candidate fanout exceeds remaining identity limit".into());
        }
        let identities = self
            .source_keys
            .len()
            .checked_add(preflight.source_keys.len())
            .ok_or("identity count overflow")?;
        if identities > self.limits.max_identities {
            return Err("dataset identity limit exceeded".into());
        }
        if preflight
            .source_keys
            .iter()
            .any(|k| self.source_keys.contains(k))
        {
            return Err("duplicate source-event key across chunks".into());
        }
        let outcome_count = self
            .outcome_keys
            .len()
            .checked_add(preflight.outcome_keys.len())
            .ok_or("outcome identity count overflow")?;
        if outcome_count > self.limits.max_identities {
            return Err("dataset outcome identity limit exceeded".into());
        }
        if preflight
            .outcome_keys
            .iter()
            .any(|k| self.outcome_keys.contains(k))
        {
            return Err("duplicate outcome identity across chunks".into());
        }
        let mut request = self.template.clone();
        request["rows"] = Value::Array(rows.clone());
        if serde_json::to_vec(&request)
            .map_err(|e| e.to_string())?
            .len()
            > self.limits.max_chunk_bytes
        {
            return Err("serialized chunk exceeds max_chunk_bytes".into());
        }
        let mapped = trace_mapping::map(&request);
        if mapped.get("classification").and_then(Value::as_str) == Some("failed") {
            return Err("logical mapper rejected chunk".into());
        }
        let ac = &mapped["accounting"];
        let mut next = self.counts;
        next.source_rows = add(next.source_rows, rows.len())?;
        next.candidate_units = add(next.candidate_units, preflight.candidate_count)?;
        next.accepted_units = next
            .accepted_units
            .checked_add(field_u64(ac, "accepted_units")?)
            .ok_or("counter overflow")?;
        next.excluded_units = next
            .excluded_units
            .checked_add(field_u64(ac, "excluded_units")?)
            .ok_or("counter overflow")?;
        next.failed_units = next
            .failed_units
            .checked_add(field_u64(ac, "failed_units")?)
            .ok_or("counter overflow")?;
        next.unresolved_units = next
            .unresolved_units
            .checked_add(field_u64(ac, "unresolved_units")?)
            .ok_or("counter overflow")?;
        next.cohort_denominator = next
            .cohort_denominator
            .checked_add(field_u64(ac, "cohort_denominator")?)
            .ok_or("counter overflow")?;
        next.missing_triage = next
            .missing_triage
            .checked_add(field_u64(ac, "missing_triage")?)
            .ok_or("counter overflow")?;
        let partition = next
            .accepted_units
            .checked_add(next.excluded_units)
            .and_then(|v| v.checked_add(next.failed_units))
            .and_then(|v| v.checked_add(next.unresolved_units))
            .ok_or("candidate partition overflow")?;
        if partition != next.candidate_units {
            return Err("candidate partition does not reconcile".into());
        }
        let mut result = MappedChunk::default();
        for value in mapped["records"]
            .as_array()
            .ok_or("mapper records missing")?
        {
            match value.get("record_type").and_then(Value::as_str) {
                Some("trace_event.v1") => result.events.push(value.clone()),
                Some("trace_exclusion.v1") => result.exclusions.push(value.clone()),
                _ => return Err("unexpected mapper record type".into()),
            }
        }
        result.outcomes = mapped["outcomes"]
            .as_array()
            .ok_or("mapper outcomes missing")?
            .clone();
        result.diagnostics = mapped["diagnostics"]
            .as_array()
            .ok_or("mapper diagnostics missing")?
            .clone();
        self.counts = next;
        self.source_keys.extend(preflight.source_keys);
        self.outcome_keys.extend(preflight.outcome_keys);
        Ok(result)
    }
    pub(crate) fn counts(&self) -> Counts {
        self.counts
    }
    pub(crate) fn mapping_hash(&self) -> String {
        let bytes = serde_json::to_vec(&self.template).expect("validated JSON template serializes");
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
fn validate_envelope(t: &Value) -> Result<(), String> {
    let o = t.as_object().ok_or("template must be object")?;
    for k in [
        "dataset_id",
        "mapping_version",
        "case_key_field",
        "source_family",
    ] {
        if o.get(k)
            .and_then(Value::as_str)
            .is_none_or(|s| s.trim().is_empty())
        {
            return Err(format!("invalid {k}"));
        }
    }
    if !["wide", "long"].contains(
        &o.get("shape")
            .and_then(Value::as_str)
            .ok_or("missing shape")?,
    ) {
        return Err("invalid shape".into());
    }
    if o.get("origin_utc").and_then(Value::as_str).is_none() {
        return Err("invalid origin_utc".into());
    }
    let bindings = o
        .get("event_bindings")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or("invalid event_bindings")?;
    let mut kinds = BTreeSet::new();
    let mut ranks = BTreeSet::new();
    for b in bindings {
        let b = b.as_object().ok_or("binding must be object")?;
        let rank = b
            .get("rank")
            .and_then(Value::as_str)
            .ok_or("invalid rank")?;
        EventKindRank::from_canonical_decimal(rank).map_err(|_| "invalid rank")?;
        let kind = b
            .get("kind")
            .and_then(Value::as_str)
            .ok_or("invalid kind")?;
        if !kinds.insert(kind.to_owned()) || !ranks.insert(rank.to_owned()) {
            return Err("duplicate kind/rank".into());
        }
        for k in ["kind", "occurrence_field", "key_field", "order_field"] {
            if b.get(k)
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                return Err(format!("invalid binding {k}"));
            }
        }
    }
    Ok(())
}
struct Preflight {
    candidate_count: usize,
    source_keys: BTreeSet<String>,
    outcome_keys: BTreeSet<(String, String)>,
}
fn preflight(t: &Value, rows: &[Value]) -> Result<Preflight, String> {
    let o = t.as_object().ok_or("invalid envelope")?;
    let bindings = o["event_bindings"].as_array().ok_or("invalid bindings")?;
    let shape = o["shape"].as_str().ok_or("invalid shape")?;
    let fanout = if shape == "wide" { bindings.len() } else { 1 };
    let count = rows
        .len()
        .checked_mul(fanout)
        .ok_or("candidate fanout overflow")?;
    let mut keys = BTreeSet::new();
    let mut outcomes = BTreeSet::new();
    let kind_field = o.get("event_kind_field").and_then(Value::as_str);
    for row in rows {
        let r = row.as_object().ok_or("row must be object")?;
        let selected: Vec<&Value> = if shape == "wide" {
            bindings.iter().collect()
        } else {
            let field = kind_field.ok_or("missing event_kind_field")?;
            let kind = r
                .get(field)
                .and_then(Value::as_str)
                .ok_or("missing long event_kind")?;
            vec![bindings
                .iter()
                .find(|b| b.get("kind").and_then(Value::as_str) == Some(kind))
                .ok_or("unknown long event_kind")?]
        };
        for b in selected {
            let key_field = b["key_field"].as_str().ok_or("invalid key_field")?;
            let key = r
                .get(key_field)
                .and_then(Value::as_str)
                .filter(|v| !v.trim().is_empty())
                .ok_or("missing source-event key")?
                .to_owned();
            if !keys.insert(key) {
                return Err("duplicate source-event key within chunk".into());
            }
        }
        if r.get("outcome").is_some_and(|v| v.is_object()) {
            let case = r
                .get(
                    o["case_key_field"]
                        .as_str()
                        .ok_or("invalid case key field")?,
                )
                .and_then(Value::as_str)
                .filter(|v| !v.trim().is_empty())
                .ok_or("missing outcome case key")?
                .to_owned();
            let endpoint = r["outcome"]
                .get("endpoint")
                .and_then(Value::as_str)
                .filter(|v| !v.trim().is_empty())
                .ok_or("missing outcome endpoint")?
                .to_owned();
            if !outcomes.insert((case, endpoint)) {
                return Err("duplicate outcome identity within chunk".into());
            }
        }
    }
    Ok(Preflight {
        candidate_count: count,
        source_keys: keys,
        outcome_keys: outcomes,
    })
}
fn add(a: u64, b: usize) -> Result<u64, String> {
    a.checked_add(u64::try_from(b).map_err(|_| "counter conversion overflow")?)
        .ok_or_else(|| "counter overflow".into())
}
fn field_u64(v: &Value, k: &str) -> Result<u64, String> {
    v.get(k)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("invalid mapper counter {k}"))
}

pub(crate) fn order_key(v: &Value) -> Result<TraceOrderKeyV1, String> {
    let o = v.as_object().ok_or("record must be object")?;
    let expected = [
        "relative_ticks",
        "case_key",
        "occurrence",
        "event_kind_rank",
        "source_event_key",
        "source_order",
    ];
    if expected.iter().any(|k| !o.contains_key(*k)) {
        return Err("order record must contain the six C0 fields".into());
    }
    let raw_ticks = o["relative_ticks"]
        .as_str()
        .ok_or("relative_ticks must be decimal string")?;
    EventKindRank::from_canonical_decimal(raw_ticks).map_err(|_| "noncanonical relative_ticks")?;
    let ticks = raw_ticks
        .parse::<u128>()
        .map_err(|_| "invalid relative_ticks")?;
    let case = o["case_key"].as_str().ok_or("invalid case_key")?.to_owned();
    let occurrence = o["occurrence"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("invalid occurrence")?;
    let rank = match &o["event_kind_rank"] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err("invalid event_kind_rank".into()),
    };
    let event_kind_rank =
        EventKindRank::from_canonical_decimal(&rank).map_err(|_| "invalid event_kind_rank")?;
    let source_event_key = o["source_event_key"]
        .as_str()
        .ok_or("invalid source_event_key")?
        .to_owned();
    let source_order = o["source_order"].as_u64().ok_or("invalid source_order")?;
    Ok(TraceOrderKeyV1 {
        relative_ticks: ticks,
        case_key: case,
        occurrence,
        event_kind_rank,
        source_event_key,
        source_order,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn clock(raw: &str) -> Value {
        json!({"raw":raw,"representation":"RFC3339","precision":"second","lineage":{"status":"observed"}})
    }
    fn template() -> Value {
        json!({"profile_version":"c11.synthetic-map-v1","dataset_id":"d","mapping_version":"m","origin_utc":"2020-01-01T00:00:00Z","case_key_field":"case","source_family":"fixture","shape":"wide","event_bindings":[{"source_event_type":"ARRIVAL","kind":"arrival","rank":"2","occurrence_index":0,"occurrence_field":"at","key_field":"id","order_field":"seq"}],"rows":[]})
    }
    fn row(id: &str, at: &str) -> Value {
        json!({"case":"c","id":id,"seq":1,"at":clock(at)})
    }
    fn norm(limits: Limits) -> Normalizer {
        Normalizer::new(template(), limits).unwrap()
    }
    fn limits() -> Limits {
        Limits {
            max_chunk_rows: 4,
            max_chunk_bytes: 8192,
            max_identities: 8,
        }
    }
    #[test]
    fn preserves_mapper_values_and_cumulative_partition_across_chunks() {
        let mut n = norm(limits());
        let a = n.push(vec![row("a", "2020-01-01T00:00:01Z")]).unwrap();
        let b = n.push(vec![row("b", "2020-01-01T00:00:02Z")]).unwrap();
        assert_eq!(a.events[0]["relative_ticks"], "1000000000");
        assert_eq!(b.events[0]["relative_ticks"], "2000000000");
        assert_eq!(
            a.events[0]["time_lineage"]["occurrence"]["status"],
            "observed"
        );
        assert_eq!(
            n.counts(),
            Counts {
                source_rows: 2,
                candidate_units: 2,
                accepted_units: 2,
                excluded_units: 0,
                failed_units: 0,
                unresolved_units: 0,
                cohort_denominator: 2,
                missing_triage: 0
            }
        );
        assert_eq!(n.mapping_hash().len(), 64);
    }
    #[test]
    fn rejects_cross_chunk_duplicates_even_when_first_candidate_was_excluded() {
        let mut n = norm(limits());
        let ex = n.push(vec![row("same", "2019-12-31T23:59:59Z")]).unwrap();
        assert_eq!(ex.exclusions.len(), 1);
        let before = n.counts();
        assert!(n.push(vec![row("same", "2020-01-01T00:00:01Z")]).is_err());
        assert_eq!(n.counts(), before);
    }
    #[test]
    fn unknown_long_kind_and_outcome_duplicate_reject_transactionally() {
        let mut t = template();
        t["shape"] = json!("long");
        t["event_kind_field"] = json!("kind");
        let mut n = Normalizer::new(t, limits()).unwrap();
        let mut r = row("x", "2020-01-01T00:00:01Z");
        r["kind"] = json!("alien");
        assert!(n.push(vec![r]).is_err());
        assert_eq!(n.counts(), Counts::default());
        let mut n = norm(limits());
        let mut a = row("a", "2020-01-01T00:00:01Z");
        a["outcome"] = json!({"endpoint":"e","risk_start":clock("2020-01-01T00:00:00Z"),"last_observed":clock("2020-01-01T00:00:01Z"),"event_clock":clock("2020-01-01T00:00:01Z"),"censor_cause":"event","censor_status":"not_censored","lineage":{"status":"observed"}});
        let mut b = a.clone();
        b["id"] = json!("b");
        n.push(vec![a]).unwrap();
        let before = n.counts();
        assert!(n.push(vec![b]).is_err());
        assert_eq!(n.counts(), before);
    }
    #[test]
    fn enforces_nonzero_limits_fanout_and_actual_serialized_bytes() {
        assert!(Normalizer::new(
            template(),
            Limits {
                max_chunk_rows: 0,
                ..limits()
            }
        )
        .is_err());
        assert!(Normalizer::new(
            template(),
            Limits {
                max_identities: 0,
                ..limits()
            }
        )
        .is_err());
        let mut t = template();
        t["event_bindings"] = json!([
            t["event_bindings"][0].clone(),
            t["event_bindings"][0].clone()
        ]);
        assert!(Normalizer::new(t, limits()).is_err());
        assert!(Normalizer::new(
            template(),
            Limits {
                max_chunk_bytes: 256,
                ..limits()
            }
        )
        .is_err());
        let template_size = serde_json::to_vec(&template()).unwrap().len();
        let mut n = norm(Limits {
            max_chunk_bytes: template_size + 16,
            ..limits()
        });
        assert!(n
            .push(vec![row("long-key", "2020-01-01T00:00:01Z")])
            .is_err());
        assert_eq!(n.counts(), Counts::default());
    }
    #[test]
    fn order_key_projects_full_records_and_unbounded_decimal_rank() {
        let v = json!({"relative_ticks":"0","case_key":"c","occurrence":0,"event_kind_rank":18446744073709551616u128,"source_event_key":"k","source_order":0});
        assert_eq!(
            order_key(&v).unwrap().event_kind_rank.as_str(),
            "18446744073709551616"
        );
        let mut extra = v.clone();
        extra["other"] = json!(0);
        assert_eq!(order_key(&extra).unwrap(), order_key(&v).unwrap());
        let mut bad = v.clone();
        bad["relative_ticks"] = json!("00");
        assert!(order_key(&bad).is_err());
        let mut n = norm(limits());
        let actual = n.push(vec![row("real", "2020-01-01T00:00:01Z")]).unwrap();
        assert!(order_key(&actual.events[0]).is_ok());
    }
}
