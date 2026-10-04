//! Independent Rust declaration of the frozen C0 physical v2 schemas.

use std::{collections::HashMap, sync::Arc};

use arrow_schema::{DataType, Field, Schema, SchemaRef};

pub const TABLES: [(&str, &str); 3] = [
    ("trace", "trace_event.v1"),
    ("exclusion", "trace_exclusion.v1"),
    ("outcome", "outcome_observation.v1"),
];

fn metadata(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn field(
    name: &str,
    data_type: DataType,
    nullable: bool,
    path: &str,
    encoding: Option<&str>,
    unit: Option<&str>,
) -> Field {
    let mut values = vec![("logical_path", path)];
    if let Some(encoding) = encoding {
        values.push(("encoding", encoding));
    }
    if let Some(unit) = unit {
        values.push(("unit", unit));
    }
    Field::new(name, data_type, nullable).with_metadata(metadata(&values))
}

fn string_field(name: &str, nullable: bool, path: &str) -> Field {
    field(name, DataType::Utf8, nullable, path, None, None)
}

fn lineage(path: &str) -> DataType {
    DataType::Struct(
        vec![
            string_field("status", false, &format!("{path}.status")),
            string_field("mapping_version", false, &format!("{path}.mapping_version")),
            string_field("evidence_ref", true, &format!("{path}.evidence_ref")),
            string_field("derivation", true, &format!("{path}.derivation")),
        ]
        .into(),
    )
}

fn time_value(path: &str) -> DataType {
    DataType::Struct(
        vec![
            string_field("raw", false, &format!("{path}.raw")),
            string_field("utc_text", false, &format!("{path}.utc")),
            field(
                "utc_i128_le",
                DataType::FixedSizeBinary(16),
                false,
                &format!("{path}.utc"),
                Some("signed_i128_le"),
                Some("ns_since_unix_epoch"),
            ),
            string_field(
                "source_offset_or_zone",
                true,
                &format!("{path}.source_offset_or_zone"),
            ),
            field(
                "relative_ticks",
                DataType::FixedSizeBinary(16),
                false,
                &format!("{path}.relative_ticks"),
                Some("unsigned_u128_le"),
                Some("1ns"),
            ),
            string_field("tick_resolution", true, &format!("{path}.tick_resolution")),
            string_field(
                "source_precision",
                false,
                &format!("{path}.source_precision"),
            ),
            field(
                "lineage",
                lineage(&format!("{path}.lineage")),
                false,
                &format!("{path}.lineage"),
                None,
                None,
            ),
        ]
        .into(),
    )
}

fn raw_event() -> DataType {
    DataType::Struct(
        vec![
            string_field("source_family", false, "raw_event.source_family"),
            string_field("source_event_type", false, "raw_event.source_event_type"),
            string_field("source_record_id", true, "raw_event.source_record_id"),
            string_field("source_fields_json", true, "raw_event.source_fields"),
        ]
        .into(),
    )
}

fn list_of_strings() -> DataType {
    DataType::List(Field::new("element", DataType::Utf8, false).into())
}

fn finish(record_type: &str, mut fields: Vec<Field>) -> SchemaRef {
    fields.push(field(
        "presence_fields",
        list_of_strings(),
        false,
        "@presence",
        None,
        None,
    ));
    Arc::new(Schema::new_with_metadata(
        fields,
        metadata(&[
            ("format", "careops.calibration.physical"),
            ("physical_version", "2"),
            ("logical_schema", "calibration-v1"),
            (
                "logical_schema_sha256",
                "8c46db62f691f243385a4ebdf8a7a3d3670e2655dd0c3f82cba4335ced2a3842",
            ),
            ("record_type", record_type),
            ("byte_order", "little"),
            ("optional_presence", "sorted_logical_paths"),
        ]),
    ))
}

fn trace_schema() -> SchemaRef {
    let time_lineage = DataType::Struct(
        ["occurrence", "source_recorded", "message_created"]
            .iter()
            .map(|key| {
                field(
                    key,
                    lineage(&format!("time_lineage.{key}")),
                    false,
                    key,
                    None,
                    None,
                )
            })
            .collect::<Vec<_>>()
            .into(),
    );
    let knowledge = DataType::Struct(
        vec![
            string_field("status", false, "knowledge_availability.status"),
            field(
                "available_at",
                time_value("knowledge_availability.available_at"),
                true,
                "knowledge_availability.available_at",
                None,
                None,
            ),
        ]
        .into(),
    );
    finish(
        "trace_event.v1",
        vec![
            string_field("record_type", false, "record_type"),
            string_field("schema_version", false, "schema_version"),
            string_field("dataset_id", false, "dataset_id"),
            string_field("mapping_version", false, "mapping_version"),
            string_field("case_key", false, "case_key"),
            string_field("source_event_key", false, "source_event_key"),
            field(
                "occurrence",
                DataType::UInt32,
                false,
                "occurrence",
                None,
                None,
            ),
            string_field("event_kind", false, "event_kind"),
            field(
                "relative_ticks",
                DataType::FixedSizeBinary(16),
                false,
                "relative_ticks",
                Some("unsigned_u128_le"),
                Some("1ns"),
            ),
            field(
                "source_order",
                DataType::UInt64,
                false,
                "source_order",
                None,
                None,
            ),
            string_field("event_kind_rank", true, "event_kind_rank"),
            field(
                "occurrence_time",
                time_value("occurrence_time"),
                false,
                "occurrence_time",
                None,
                None,
            ),
            field(
                "source_recorded_time",
                time_value("source_recorded_time"),
                true,
                "source_recorded_time",
                None,
                None,
            ),
            field(
                "message_created_time",
                time_value("message_created_time"),
                true,
                "message_created_time",
                None,
                None,
            ),
            field(
                "time_lineage",
                time_lineage,
                false,
                "time_lineage",
                None,
                None,
            ),
            string_field("resource_key", true, "resource_key"),
            string_field("actor_key", true, "actor_key"),
            string_field("location_key", true, "location_key"),
            field(
                "quality_flags",
                list_of_strings(),
                true,
                "quality_flags",
                None,
                None,
            ),
            field("raw_event", raw_event(), false, "raw_event", None, None),
            string_field("disposition", false, "disposition"),
            field(
                "knowledge_availability",
                knowledge,
                false,
                "knowledge_availability",
                None,
                None,
            ),
        ],
    )
}

fn exclusion_schema() -> SchemaRef {
    let raw_time_entries = DataType::List(
        Field::new(
            "element",
            DataType::Struct(
                vec![
                    string_field("key", false, "raw_time_values.@key"),
                    string_field("value", true, "raw_time_values.@value"),
                ]
                .into(),
            ),
            false,
        )
        .into(),
    );
    finish(
        "trace_exclusion.v1",
        vec![
            string_field("record_type", false, "record_type"),
            string_field("schema_version", false, "schema_version"),
            string_field("dataset_id", false, "dataset_id"),
            string_field("mapping_version", false, "mapping_version"),
            string_field("source_event_key", false, "source_event_key"),
            field("raw_event", raw_event(), false, "raw_event", None, None),
            field(
                "raw_time_values",
                raw_time_entries,
                false,
                "raw_time_values",
                Some("sorted_unique_entries_v2"),
                None,
            ),
            string_field("exclusion_reason", false, "exclusion_reason"),
            field("lineage", lineage("lineage"), false, "lineage", None, None),
            string_field("detail", true, "detail"),
        ],
    )
}

fn outcome_schema() -> SchemaRef {
    finish(
        "outcome_observation.v1",
        vec![
            string_field("record_type", false, "record_type"),
            string_field("schema_version", false, "schema_version"),
            string_field("dataset_id", false, "dataset_id"),
            string_field("case_key", false, "case_key"),
            string_field("endpoint", false, "endpoint"),
            field(
                "risk_start",
                time_value("risk_start"),
                true,
                "risk_start",
                None,
                None,
            ),
            field(
                "last_observed",
                time_value("last_observed"),
                true,
                "last_observed",
                None,
                None,
            ),
            field(
                "event_observed",
                DataType::Boolean,
                false,
                "event_observed",
                None,
                None,
            ),
            field(
                "event_time",
                time_value("event_time"),
                true,
                "event_time",
                None,
                None,
            ),
            string_field("event_cause", true, "event_cause"),
            string_field("censor_status", false, "censor_status"),
            string_field("censor_reason", true, "censor_reason"),
            field(
                "cluster_ids",
                list_of_strings(),
                false,
                "cluster_ids",
                None,
                None,
            ),
            field("lineage", lineage("lineage"), false, "lineage", None, None),
        ],
    )
}

pub fn schema(record_type: &str) -> SchemaRef {
    match record_type {
        "trace_event.v1" => trace_schema(),
        "trace_exclusion.v1" => exclusion_schema(),
        "outcome_observation.v1" => outcome_schema(),
        _ => panic!("unknown physical record type: {record_type}"),
    }
}

#[derive(Clone)]
pub enum FieldChange {
    DataType(DataType),
    Nullable(bool),
    Metadata(String, Option<String>),
}

pub fn change_field(schema: &SchemaRef, path: &[&str], change: FieldChange) -> SchemaRef {
    let fields = schema
        .fields()
        .iter()
        .map(|field| {
            if field.name() == path[0] {
                change_path(field.as_ref(), &path[1..], &change)
            } else {
                field.as_ref().clone()
            }
        })
        .collect::<Vec<_>>();
    Arc::new(Schema::new_with_metadata(fields, schema.metadata().clone()))
}

fn change_path(field: &Field, rest: &[&str], change: &FieldChange) -> Field {
    if rest.is_empty() {
        return match change {
            FieldChange::DataType(data_type) => field.clone().with_data_type(data_type.clone()),
            FieldChange::Nullable(nullable) => field.clone().with_nullable(*nullable),
            FieldChange::Metadata(key, value) => {
                let mut metadata = field.metadata().clone();
                if let Some(value) = value {
                    metadata.insert(key.clone(), value.clone());
                } else {
                    metadata.remove(key);
                }
                field.clone().with_metadata(metadata)
            }
        };
    }
    field
        .clone()
        .with_data_type(change_type_path(field.data_type(), rest, change))
}

fn change_type_path(data_type: &DataType, path: &[&str], change: &FieldChange) -> DataType {
    match data_type {
        DataType::Struct(fields) => DataType::Struct(
            fields
                .iter()
                .map(|field| {
                    if field.name() == path[0] {
                        change_path(field.as_ref(), &path[1..], change)
                    } else {
                        field.as_ref().clone()
                    }
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        DataType::List(item) if item.name() == path[0] => {
            DataType::List(change_path(item.as_ref(), &path[1..], change).into())
        }
        other => panic!("cannot walk schema path {path:?} through {other:?}"),
    }
}

pub fn change_global_metadata(schema: &SchemaRef, key: &str, value: Option<&str>) -> SchemaRef {
    let mut metadata = schema.metadata().clone();
    if let Some(value) = value {
        metadata.insert(key.to_string(), value.to_string());
    } else {
        metadata.remove(key);
    }
    Arc::new(Schema::new_with_metadata(schema.fields().clone(), metadata))
}
