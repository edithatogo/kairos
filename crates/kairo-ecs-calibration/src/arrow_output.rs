//! Private C4.3 physical encoder for calibration sidecars.
#![expect(
    dead_code,
    reason = "private C4.3 codec is path-imported by its focused integration qualification"
)]
use arrow_array::{
    builder::FixedSizeBinaryBuilder, ArrayRef, BooleanArray, Float64Array, RecordBatch,
    StringArray, UInt32Array, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use serde_json::{Map, Number, Value};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum OutputError {
    #[error("unknown sidecar record type")]
    UnknownType,
    #[error("invalid row: {0}")]
    Invalid(String),
    #[error("Arrow batch does not match the frozen schema")]
    SchemaMismatch,
    #[error("Arrow operation failed: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Str,
    NullableStr,
    U32,
    U64,
    Tick,
    NullableTick,
    Bool,
    NullableF64,
    JsonObject,
}
struct Col {
    name: &'static str,
    kind: Kind,
    encoding: Option<&'static str>,
}
macro_rules! c {
    ($n:literal,$k:ident) => {
        Col {
            name: $n,
            kind: Kind::$k,
            encoding: None,
        }
    };
}
macro_rules! ct {
    ($n:literal,$k:ident,$e:literal) => {
        Col {
            name: $n,
            kind: Kind::$k,
            encoding: Some($e),
        }
    };
}
macro_rules! cj {
    ($n:literal,$k:ident,$e:literal) => {
        ct!($n, $k, $e)
    };
}
const R: &[Col] = &[
    c!("record_type", Str),
    c!("schema_version", Str),
    c!("dataset_id", Str),
    c!("scenario_id", Str),
    c!("run_id", Str),
    c!("candidate_id", Str),
    c!("case_key", Str),
    c!("task_key", Str),
    c!("occurrence", U32),
    c!("endpoint", Str),
    c!("fidelity", Str),
    c!("observed_ticks", NullableTick),
    c!("predicted_ticks", NullableTick),
    c!("residual_status", Str),
    c!("residual_sign", Str),
    c!("residual_magnitude", NullableTick),
    c!("prediction_unclamped", Bool),
    c!("anchor_role", Str),
    c!("feasibility", Str),
    c!("censor_status", Str),
    c!("study_id", Str),
    c!("replication_id", Str),
    c!("seed_schedule_id", Str),
    c!("seed_purpose", Str),
    c!("seed_map_ref", Str),
    c!("seed_contract_version", Str),
    c!("mapping_version", Str),
    c!("parameter_hash", Str),
    c!("graph_hash", NullableStr),
    c!("causal_ref", NullableStr),
];
const M: &[Col] = &[
    c!("record_type", Str),
    c!("schema_version", Str),
    c!("metric", Str),
    c!("algorithm_version", Str),
    c!("endpoint", Str),
    cj!("strata", JsonObject, "canonical_json_object"),
    ct!("window_start_ticks", Tick, "unsigned_u128_le"),
    ct!("window_end_ticks", Tick, "unsigned_u128_le"),
    c!("units", Str),
    c!("reference_count", U64),
    c!("simulation_count", U64),
    c!("excluded_count", U64),
    c!("censored_count", U64),
    c!("missing_count", U64),
    c!("unmatched_count", U64),
    c!("failed_count", U64),
    c!("infeasible_count", U64),
    c!("value", NullableF64),
    c!("status", Str),
    c!("uncertainty", NullableStr),
    c!("provenance_dataset_id", Str),
    c!("provenance_run_id", Str),
    c!("provenance_mapping_version", Str),
    c!("provenance_seed_schedule_id", NullableStr),
    c!("provenance_seed_map_ref", NullableStr),
    c!("provenance_seed_contract_version", NullableStr),
    c!("provenance_parameter_hash", NullableStr),
];
fn cols(kind: &str) -> Result<(&'static [Col], &'static str), OutputError> {
    match kind {
        "calibration_residual.v1" => Ok((R, "calibration_residual.v1")),
        "calibration_metric.v1" => Ok((M, "calibration_metric.v1")),
        _ => Err(OutputError::UnknownType),
    }
}
fn canonical(v: &Value) -> Result<String, OutputError> {
    fn sorted(v: &Value) -> Value {
        match v {
            Value::Object(o) => {
                let mut b = BTreeMap::new();
                for (k, v) in o {
                    b.insert(k.clone(), sorted(v));
                }
                let mut m = Map::new();
                for (k, v) in b {
                    m.insert(k, v);
                }
                Value::Object(m)
            }
            Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
            x => x.clone(),
        }
    }
    serde_json::to_string(&sorted(v)).map_err(|e| OutputError::Invalid(e.to_string()))
}
fn required<'a>(row: &'a Value, key: &str) -> Result<&'a Value, OutputError> {
    row.get(key)
        .ok_or_else(|| OutputError::Invalid(format!("missing {key}")))
}
fn exact_object(value: &Value, expected: &[&str], name: &str) -> Result<(), OutputError> {
    let object = value
        .as_object()
        .ok_or_else(|| OutputError::Invalid(format!("{name} must be object")))?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(OutputError::Invalid(format!(
            "{name} fields do not match contract"
        )));
    }
    Ok(())
}
fn flat_row(kind: &str, row: &Value) -> Result<Value, OutputError> {
    let mut x = row.clone();
    if kind == "calibration_metric.v1" {
        for (from, to) in [
            ("start_ticks", "window_start_ticks"),
            ("end_ticks", "window_end_ticks"),
        ] {
            x[to] = required(required(row, "window")?, from)?.clone();
        }
        for (from, to) in [
            ("dataset_id", "provenance_dataset_id"),
            ("run_id", "provenance_run_id"),
            ("mapping_version", "provenance_mapping_version"),
            ("seed_schedule_id", "provenance_seed_schedule_id"),
            ("seed_map_ref", "provenance_seed_map_ref"),
            ("seed_contract_version", "provenance_seed_contract_version"),
            ("parameter_hash", "provenance_parameter_hash"),
        ] {
            x[to] = required(required(row, "provenance")?, from)?.clone();
        }
    }
    Ok(x)
}
fn validate(row: &Value, kind: &str, columns: &[Col]) -> Result<Value, OutputError> {
    let obj = row
        .as_object()
        .ok_or_else(|| OutputError::Invalid("row must be object".into()))?;
    let allowed: HashSet<&str> = if kind == "calibration_residual.v1" {
        R.iter().map(|c| c.name).collect()
    } else {
        [
            "record_type",
            "schema_version",
            "metric",
            "algorithm_version",
            "endpoint",
            "strata",
            "window",
            "units",
            "reference_count",
            "simulation_count",
            "excluded_count",
            "censored_count",
            "missing_count",
            "unmatched_count",
            "failed_count",
            "infeasible_count",
            "value",
            "status",
            "uncertainty",
            "provenance",
        ]
        .into_iter()
        .collect()
    };
    if obj.keys().any(|k| !allowed.contains(k.as_str())) {
        return Err(OutputError::Invalid("unknown logical field".into()));
    }
    if kind == "calibration_metric.v1" {
        exact_object(
            required(row, "window")?,
            &["start_ticks", "end_ticks"],
            "window",
        )?;
        exact_object(
            required(row, "provenance")?,
            &[
                "dataset_id",
                "run_id",
                "mapping_version",
                "seed_schedule_id",
                "seed_map_ref",
                "seed_contract_version",
                "parameter_hash",
            ],
            "provenance",
        )?;
    }
    let flat = flat_row(kind, row)?;
    let fo = flat.as_object().unwrap();
    for c in columns {
        let v = fo
            .get(c.name)
            .ok_or_else(|| OutputError::Invalid(format!("missing {}", c.name)))?;
        match c.kind {
            Kind::Str => {
                if !v.as_str().is_some_and(|s| !s.trim().is_empty()) {
                    return Err(OutputError::Invalid(format!("{} must be string", c.name)));
                }
            }
            Kind::NullableStr => {
                if !v.is_null() && !v.as_str().is_some_and(|s| !s.trim().is_empty()) {
                    return Err(OutputError::Invalid(format!(
                        "{} must be string or null",
                        c.name
                    )));
                }
            }
            Kind::U32 => {
                if !v.as_u64().is_some_and(|n| n <= u32::MAX as u64) {
                    return Err(OutputError::Invalid(format!("{} must be UInt32", c.name)));
                }
            }
            Kind::U64 => {
                if !v.as_u64().is_some() {
                    return Err(OutputError::Invalid(format!("{} must be UInt64", c.name)));
                }
            }
            Kind::Bool => {
                if !v.is_boolean() {
                    return Err(OutputError::Invalid(format!("{} must be boolean", c.name)));
                }
            }
            Kind::Tick | Kind::NullableTick => {
                if !(c.kind == Kind::NullableTick && v.is_null()) {
                    parse_tick(v, c.name)?;
                }
            }
            Kind::NullableF64 => {
                if !v.is_null() && v.as_f64().is_none_or(|n| !n.is_finite()) {
                    return Err(OutputError::Invalid(
                        "value must be finite number or null".into(),
                    ));
                }
            }
            Kind::JsonObject => {
                if !v.is_object() {
                    return Err(OutputError::Invalid("strata must be object".into()));
                }
            }
        }
    }
    let expected = if kind == "calibration_residual.v1" {
        "calibration_residual.v1"
    } else {
        "calibration_metric.v1"
    };
    if fo["record_type"] != expected || fo["schema_version"] != "calibration-v1" {
        return Err(OutputError::Invalid("record identity mismatch".into()));
    }
    if kind == "calibration_residual.v1" {
        if !["ShadowAnchored", "FreeRunning"].contains(&fo["fidelity"].as_str().unwrap())
            || !["service", "transit", "behavior", "calibration"]
                .contains(&fo["seed_purpose"].as_str().unwrap())
            || ![
                "not_censored",
                "left",
                "right",
                "interval",
                "unknown",
                "missing",
            ]
            .contains(&fo["censor_status"].as_str().unwrap())
        {
            return Err(OutputError::Invalid(
                "invalid fidelity/censor status".into(),
            ));
        }
        for (key, allowed) in [
            (
                "residual_status",
                &[
                    "computed",
                    "missing_observed",
                    "censored",
                    "probe_failed",
                    "infeasible",
                ][..],
            ),
            (
                "residual_sign",
                &["negative", "zero", "positive", "undefined"][..],
            ),
            (
                "anchor_role",
                &["observed_source_transition", "none", "unknown"][..],
            ),
            ("feasibility", &["feasible", "infeasible", "unknown"][..]),
        ] {
            if !allowed.contains(&fo[key].as_str().unwrap()) {
                return Err(OutputError::Invalid(format!("invalid {key}")));
            }
        }
        for key in ["parameter_hash", "graph_hash"] {
            if (key == "parameter_hash" && fo[key].is_null())
                || (!fo[key].is_null() && !is_hash(fo[key].as_str().unwrap()))
            {
                return Err(OutputError::Invalid(format!("invalid {key}")));
            }
        }
        if fo["prediction_unclamped"] != true {
            return Err(OutputError::Invalid(
                "prediction_unclamped must be true".into(),
            ));
        }
        let st = fo["residual_status"].as_str().unwrap();
        let sign = fo["residual_sign"].as_str().unwrap();
        let p = fo["predicted_ticks"]
            .as_str()
            .map(parse_tick_str)
            .transpose()?;
        let o = fo["observed_ticks"]
            .as_str()
            .map(parse_tick_str)
            .transpose()?;
        let mag = fo["residual_magnitude"]
            .as_str()
            .map(parse_tick_str)
            .transpose()?;
        if st == "computed" {
            let (p, o, m) = (
                p.ok_or_else(|| {
                    OutputError::Invalid("computed prediction/observation required".into())
                })?,
                o.ok_or_else(|| {
                    OutputError::Invalid("computed prediction/observation required".into())
                })?,
                mag.ok_or_else(|| OutputError::Invalid("computed magnitude required".into()))?,
            );
            let (want, wm) = if p > o {
                ("positive", p - o)
            } else if p < o {
                ("negative", o - p)
            } else {
                ("zero", 0)
            };
            if sign != want || m != wm {
                return Err(OutputError::Invalid(
                    "computed residual does not equal predicted-observed".into(),
                ));
            }
        } else if sign != "undefined" || mag.is_some() {
            return Err(OutputError::Invalid(
                "noncomputed residual requires undefined sign and null magnitude".into(),
            ));
        }
        if matches!(st, "missing_observed" | "censored") && o.is_some() {
            return Err(OutputError::Invalid(
                "missing/censored observation must be null".into(),
            ));
        }
        if st == "probe_failed" && p.is_some() {
            return Err(OutputError::Invalid(
                "probe_failed prediction must be null".into(),
            ));
        }
    }
    if kind == "calibration_metric.v1" {
        if !["W1", "KS_D", "paired_residual_summary", "other"]
            .contains(&fo["metric"].as_str().unwrap())
            || ![
                "computed",
                "insufficient_data",
                "empty",
                "invalid",
                "unverified",
            ]
            .contains(&fo["status"].as_str().unwrap())
        {
            return Err(OutputError::Invalid(
                "invalid metric/status vocabulary".into(),
            ));
        }
        let start = parse_tick(&fo["window_start_ticks"], "window_start_ticks")?;
        let end = parse_tick(&fo["window_end_ticks"], "window_end_ticks")?;
        if start >= end {
            return Err(OutputError::Invalid(
                "window must be nonempty half-open interval".into(),
            ));
        }
        if !fo["provenance_parameter_hash"].is_null()
            && !is_hash(fo["provenance_parameter_hash"].as_str().unwrap())
        {
            return Err(OutputError::Invalid(
                "invalid provenance_parameter_hash".into(),
            ));
        }
        if !fo["uncertainty"].is_null() {
            return Err(OutputError::Invalid(
                "uncertainty method not supported".into(),
            ));
        }
        if fo["status"] != "computed" && !fo["value"].is_null() {
            return Err(OutputError::Invalid(
                "noncomputed value must be null".into(),
            ));
        }
        if fo["status"] == "computed" {
            let value = fo["value"]
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| {
                    OutputError::Invalid("computed metric requires finite value".into())
                })?;
            if matches!(fo["metric"].as_str(), Some("W1" | "KS_D"))
                && (fo["reference_count"].as_u64() == Some(0)
                    || fo["simulation_count"].as_u64() == Some(0))
            {
                return Err(OutputError::Invalid(
                    "computed W1/KS_D requires nonempty cohorts".into(),
                ));
            }
            if fo["metric"] == "KS_D" && !(0.0..=1.0).contains(&value) {
                return Err(OutputError::Invalid("KS_D must be within [0,1]".into()));
            }
            if fo["metric"] == "W1" && value < 0.0 {
                return Err(OutputError::Invalid("W1 must be nonnegative".into()));
            }
        }
        if fo["provenance_seed_schedule_id"].is_null()
            && (fo["simulation_count"].as_u64().unwrap_or(0) > 0
                || fo["failed_count"].as_u64().unwrap_or(0) > 0)
        {
            return Err(OutputError::Invalid(
                "seed provenance required for simulation attempts".into(),
            ));
        }
        if fo["metric"] == "KS_D" && fo["units"] != "dimensionless" {
            return Err(OutputError::Invalid(
                "KS_D units must be dimensionless".into(),
            ));
        }
        let s = fo["provenance_seed_schedule_id"].is_null();
        let m = fo["provenance_seed_map_ref"].is_null();
        if s != m {
            return Err(OutputError::Invalid(
                "seed schedule/map nullity mismatch".into(),
            ));
        }
    }
    Ok(flat)
}
fn is_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn parse_tick(v: &Value, name: &str) -> Result<u128, OutputError> {
    let s = v
        .as_str()
        .ok_or_else(|| OutputError::Invalid(format!("{name} must be decimal string")))?;
    parse_tick_str(s)
}
fn parse_tick_str(s: &str) -> Result<u128, OutputError> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(OutputError::Invalid("invalid u128 decimal".into()));
    }
    s.parse()
        .map_err(|_| OutputError::Invalid("u128 overflow".into()))
}
pub(crate) fn schema(kind: &str) -> Result<SchemaRef, OutputError> {
    let (cols, record) = cols(kind)?;
    let mut fs = Vec::new();
    for c in cols {
        let (dt, n) = match c.kind {
            Kind::Str => (DataType::Utf8, false),
            Kind::NullableStr => (DataType::Utf8, true),
            Kind::U32 => (DataType::UInt32, false),
            Kind::U64 => (DataType::UInt64, false),
            Kind::Tick => (DataType::FixedSizeBinary(16), false),
            Kind::NullableTick => (DataType::FixedSizeBinary(16), true),
            Kind::Bool => (DataType::Boolean, false),
            Kind::NullableF64 => (DataType::Float64, true),
            Kind::JsonObject => (DataType::Utf8, false),
        };
        let mut md = BTreeMap::new();
        if let Some(e) = c.encoding {
            md.insert("encoding".into(), e.into());
        }
        if c.kind == Kind::Tick || c.kind == Kind::NullableTick {
            md.insert("unit".into(), "1ns".into());
            md.insert("encoding".into(), "unsigned_u128_le".into());
        }
        fs.push(
            Field::new(c.name, dt, n)
                .with_metadata(md.into_iter().collect::<std::collections::HashMap<_, _>>()),
        );
    }
    let mut md = BTreeMap::new();
    for (k, v) in [
        ("format", "kairoecs.calibration.output"),
        ("physical_version", "1"),
        ("logical_schema", "calibration-v1"),
        ("record_type", record),
        ("byte_order", "little"),
    ] {
        md.insert(k.into(), v.into());
    }
    Ok(Arc::new(Schema::new_with_metadata(
        fs,
        md.into_iter().collect::<std::collections::HashMap<_, _>>(),
    )))
}
pub(crate) fn encode(kind: &str, rows: &[Value]) -> Result<RecordBatch, OutputError> {
    let schema = schema(kind)?;
    let (cols, _) = cols(kind)?;
    let mut norm = rows
        .iter()
        .map(|r| validate(r, kind, cols))
        .collect::<Result<Vec<_>, _>>()?;
    norm.sort_by(|a, b| sort_cmp(a, b, kind));
    for pair in norm.windows(2) {
        if sort_cmp(&pair[0], &pair[1], kind).is_eq() {
            return Err(OutputError::Invalid("duplicate logical output key".into()));
        }
    }
    let mut arrays: Vec<ArrayRef> = Vec::new();
    for c in cols {
        let vals = norm
            .iter()
            .map(|r| r.get(c.name).unwrap())
            .collect::<Vec<_>>();
        let a: ArrayRef = match c.kind {
            Kind::Str => Arc::new(StringArray::from(
                vals.iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>(),
            )),
            Kind::NullableStr => Arc::new(StringArray::from(
                vals.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
            )),
            Kind::U32 => Arc::new(UInt32Array::from(
                vals.iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect::<Vec<_>>(),
            )),
            Kind::U64 => Arc::new(UInt64Array::from(
                vals.iter().map(|v| v.as_u64().unwrap()).collect::<Vec<_>>(),
            )),
            Kind::Bool => Arc::new(BooleanArray::from(
                vals.iter()
                    .map(|v| v.as_bool().unwrap())
                    .collect::<Vec<_>>(),
            )),
            Kind::NullableF64 => Arc::new(Float64Array::from(
                vals.iter().map(|v| v.as_f64()).collect::<Vec<_>>(),
            )),
            Kind::JsonObject => Arc::new(StringArray::from(
                vals.iter()
                    .map(|v| canonical(v).unwrap())
                    .collect::<Vec<_>>(),
            )),
            Kind::Tick | Kind::NullableTick => {
                let mut b = FixedSizeBinaryBuilder::with_capacity(vals.len(), 16);
                for v in vals {
                    if v.is_null() {
                        b.append_null()
                    } else {
                        let n = parse_tick(v, c.name)?;
                        b.append_value(n.to_le_bytes())?
                    }
                }
                Arc::new(b.finish())
            }
        };
        arrays.push(a)
    }
    Ok(RecordBatch::try_new(schema, arrays)?)
}
fn sort_cmp(a: &Value, b: &Value, kind: &str) -> std::cmp::Ordering {
    let keys: &[&str] = if kind == "calibration_residual.v1" {
        &[
            "study_id",
            "dataset_id",
            "scenario_id",
            "seed_schedule_id",
            "replication_id",
            "case_key",
            "task_key",
            "occurrence",
            "endpoint",
            "seed_purpose",
            "seed_map_ref",
            "mapping_version",
            "candidate_id",
            "run_id",
        ]
    } else {
        &[
            "endpoint",
            "strata",
            "window_start_ticks",
            "window_end_ticks",
            "units",
            "provenance_dataset_id",
            "provenance_run_id",
            "provenance_mapping_version",
            "provenance_seed_schedule_id",
            "provenance_seed_map_ref",
            "provenance_seed_contract_version",
            "provenance_parameter_hash",
            "metric",
            "algorithm_version",
        ]
    };
    for key in keys {
        let ord = if *key == "occurrence" {
            a[*key].as_u64().cmp(&b[*key].as_u64())
        } else if *key == "window_start_ticks" || *key == "window_end_ticks" {
            parse_tick(&a[*key], key)
                .ok()
                .cmp(&parse_tick(&b[*key], key).ok())
        } else if *key == "strata" {
            canonical(&a[*key])
                .unwrap()
                .cmp(&canonical(&b[*key]).unwrap())
        } else {
            a[*key].as_str().cmp(&b[*key].as_str())
        };
        if !ord.is_eq() {
            return ord;
        }
    }
    std::cmp::Ordering::Equal
}

pub(crate) fn decode(kind: &str, batch: &RecordBatch) -> Result<Vec<Value>, OutputError> {
    let expected = schema(kind)?;
    if batch.schema().as_ref() != expected.as_ref() {
        return Err(OutputError::SchemaMismatch);
    }
    let (cols, _) = cols(kind)?;
    let mut out = Vec::new();
    for row in 0..batch.num_rows() {
        let mut obj = Map::new();
        for (i, c) in cols.iter().enumerate() {
            let a = batch.column(i);
            let v: Value = match c.kind {
                Kind::Str => Value::String(
                    a.as_any()
                        .downcast_ref::<StringArray>()
                        .unwrap()
                        .value(row)
                        .into(),
                ),
                Kind::NullableStr => {
                    let x = a.as_any().downcast_ref::<StringArray>().unwrap();
                    if x.is_null(row) {
                        Value::Null
                    } else {
                        Value::String(x.value(row).into())
                    }
                }
                Kind::U32 => json!(a.as_any().downcast_ref::<UInt32Array>().unwrap().value(row)),
                Kind::U64 => json!(a.as_any().downcast_ref::<UInt64Array>().unwrap().value(row)),
                Kind::Bool => json!(a
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                    .unwrap()
                    .value(row)),
                Kind::NullableF64 => {
                    let x = a.as_any().downcast_ref::<Float64Array>().unwrap();
                    if x.is_null(row) {
                        Value::Null
                    } else {
                        Value::Number(Number::from_f64(x.value(row)).ok_or_else(|| {
                            OutputError::Invalid("nonfinite decoded number".into())
                        })?)
                    }
                }
                Kind::JsonObject => {
                    let text = a.as_any().downcast_ref::<StringArray>().unwrap().value(row);
                    let value: Value = serde_json::from_str(text)
                        .map_err(|e| OutputError::Invalid(e.to_string()))?;
                    if !value.is_object() || canonical(&value)? != text {
                        return Err(OutputError::Invalid(
                            "strata must be canonical JSON object".into(),
                        ));
                    }
                    value
                }
                Kind::Tick | Kind::NullableTick => {
                    let x = a
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                        .unwrap();
                    if x.is_null(row) {
                        Value::Null
                    } else {
                        Value::String(
                            u128::from_le_bytes(x.value(row).try_into().unwrap()).to_string(),
                        )
                    }
                }
            };
            obj.insert(c.name.into(), v);
        }
        if kind == "calibration_metric.v1" {
            let mut window = Map::new();
            window.insert(
                "start_ticks".into(),
                obj.remove("window_start_ticks").unwrap(),
            );
            window.insert("end_ticks".into(), obj.remove("window_end_ticks").unwrap());
            obj.insert("window".into(), Value::Object(window));
            let mut p = Map::new();
            for (from, to) in [
                ("provenance_dataset_id", "dataset_id"),
                ("provenance_run_id", "run_id"),
                ("provenance_mapping_version", "mapping_version"),
                ("provenance_seed_schedule_id", "seed_schedule_id"),
                ("provenance_seed_map_ref", "seed_map_ref"),
                ("provenance_seed_contract_version", "seed_contract_version"),
                ("provenance_parameter_hash", "parameter_hash"),
            ] {
                p.insert(to.into(), obj.remove(from).unwrap());
            }
            obj.insert("provenance".into(), Value::Object(p));
        }
        out.push(Value::Object(obj));
    }
    let flat = out
        .iter()
        .map(|row| validate(row, kind, cols))
        .collect::<Result<Vec<_>, _>>()?;
    for pair in flat.windows(2) {
        match sort_cmp(&pair[0], &pair[1], kind) {
            std::cmp::Ordering::Equal => {
                return Err(OutputError::Invalid("duplicate logical output key".into()));
            }
            std::cmp::Ordering::Greater => {
                return Err(OutputError::Invalid("noncanonical output row order".into()));
            }
            std::cmp::Ordering::Less => {}
        }
    }
    Ok(out)
}
use arrow_array::Array;
use serde_json::json;
