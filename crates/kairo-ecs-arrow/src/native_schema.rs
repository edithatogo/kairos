//! Track 62 Task 0.2 - official Apache Arrow schema parity.
//!
//! `EVENT_LOG_FIELDS` remains the single source of truth for the KairoECS event
//! log contract. This module *derives* an `arrow_schema::Schema` from those
//! declarations rather than restating them, so the dependency-free smoke format
//! and the native Arrow schema cannot silently drift apart.
//!
//! Parity is bidirectional and is asserted by the tests at the bottom of this
//! file: every declared type string must parse to a real Arrow `DataType`, and
//! every field of the derived schema must render back to the exact declared
//! name, type string and nullability.

use arrow_schema::{DataType, Field, Schema, SchemaRef};

use crate::{ArrowError, EventLogField, EVENT_LOG_FIELDS, EVENT_LOG_SCHEMA_MAJOR};

/// Parse a declared KairoECS type string into the official Arrow `DataType`.
///
/// Supports the fixed-width integer, floating point, boolean, UTF-8 and
/// fixed-size-binary spellings used by the event-log and snapshot contracts.
/// Returns [`ArrowError::UnsupportedDataType`] for anything else so an
/// unrecognised declaration fails closed instead of silently degrading.
pub fn parse_declared_type(declared: &str) -> Result<DataType, ArrowError> {
    let fixed = match declared {
        "Boolean" => Some(DataType::Boolean),
        "Int8" => Some(DataType::Int8),
        "Int16" => Some(DataType::Int16),
        "Int32" => Some(DataType::Int32),
        "Int64" => Some(DataType::Int64),
        "UInt8" => Some(DataType::UInt8),
        "UInt16" => Some(DataType::UInt16),
        "UInt32" => Some(DataType::UInt32),
        "UInt64" => Some(DataType::UInt64),
        "Float32" => Some(DataType::Float32),
        "Float64" => Some(DataType::Float64),
        "Utf8" => Some(DataType::Utf8),
        "LargeUtf8" => Some(DataType::LargeUtf8),
        _ => None,
    };
    if let Some(data_type) = fixed {
        return Ok(data_type);
    }

    if let Some(inner) = declared
        .strip_prefix("FixedSizeBinary(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let width: i32 = inner
            .parse()
            .map_err(|_| ArrowError::UnsupportedDataType(declared.to_string()))?;
        if width < 0 {
            return Err(ArrowError::UnsupportedDataType(declared.to_string()));
        }
        return Ok(DataType::FixedSizeBinary(width));
    }

    Err(ArrowError::UnsupportedDataType(declared.to_string()))
}

/// Render an Arrow `DataType` back into its declared KairoECS type string.
///
/// This is the inverse of [`parse_declared_type`] for every type that function
/// accepts, and is what makes the parity test meaningful in both directions.
pub fn declared_type_string(data_type: &DataType) -> String {
    format!("{data_type:?}")
}

/// Derive the official Arrow schema for the event log from `EVENT_LOG_FIELDS`.
///
/// Field order, names and nullability are preserved exactly as declared.
pub fn event_log_arrow_schema() -> Result<Schema, ArrowError> {
    event_log_arrow_schema_for(EVENT_LOG_FIELDS)
}

/// Derive an official Arrow schema from an arbitrary declared field list.
pub fn event_log_arrow_schema_for(fields: &[EventLogField]) -> Result<Schema, ArrowError> {
    let mut arrow_fields = Vec::with_capacity(fields.len());
    for field in fields {
        arrow_fields.push(Field::new(
            field.name,
            parse_declared_type(field.data_type)?,
            field.nullable,
        ));
    }
    Ok(Schema::new(arrow_fields))
}

/// Shared handle to the derived event-log schema.
pub fn event_log_arrow_schema_ref() -> Result<SchemaRef, ArrowError> {
    Ok(std::sync::Arc::new(event_log_arrow_schema()?))
}

/// The declared Arrow schema major version carried by the native schema.
pub const ARROW_SCHEMA_MAJOR: u16 = EVENT_LOG_SCHEMA_MAJOR;

/// Assert that a derived Arrow schema round-trips exactly to `fields`.
///
/// Returns the first divergence as a human-readable reason. Shared by the unit
/// tests and reusable by the cross-reader conformance fixtures added in Task 1.
pub fn assert_schema_matches_declaration(
    schema: &Schema,
    fields: &[EventLogField],
) -> Result<(), String> {
    if schema.fields().len() != fields.len() {
        return Err(format!(
            "field count mismatch: arrow schema has {}, declaration has {}",
            schema.fields().len(),
            fields.len()
        ));
    }

    for (index, declared) in fields.iter().enumerate() {
        let actual = schema.field(index);
        if actual.name() != declared.name {
            return Err(format!(
                "field {index} name mismatch: arrow schema has {:?}, declaration has {:?}",
                actual.name(),
                declared.name
            ));
        }

        let actual_type = declared_type_string(actual.data_type());
        if actual_type != declared.data_type {
            return Err(format!(
                "field {index} ({}) type mismatch: arrow schema has {actual_type}, declaration has {}",
                declared.name, declared.data_type
            ));
        }

        if actual.is_nullable() != declared.nullable {
            return Err(format!(
                "field {index} ({}) nullability mismatch: arrow schema has {}, declaration has {}",
                declared.name,
                if actual.is_nullable() {
                    "nullable"
                } else {
                    "required"
                },
                if declared.nullable {
                    "nullable"
                } else {
                    "required"
                }
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log_schema_fingerprint;

    /// Every declared field must parse into a real Arrow type. This is the red
    /// case: an unrecognised declaration string fails the build-time contract
    /// rather than being skipped.
    #[test]
    fn every_declared_field_parses_to_an_arrow_type() {
        for field in EVENT_LOG_FIELDS {
            let parsed = parse_declared_type(field.data_type)
                .unwrap_or_else(|error| panic!("field {} failed to parse: {error}", field.name));
            assert!(
                !matches!(parsed, DataType::Utf8 | DataType::LargeUtf8)
                    || field.name.ends_with("kind")
                    || field.name.ends_with("scale")
                    || field.name.ends_with("status")
                    || field.name == "run_id"
                    || field.name == "payload_ref",
                "unexpected UTF-8 column {}",
                field.name
            );
        }
    }

    /// Unknown type spellings must fail closed.
    #[test]
    fn unknown_type_spellings_fail_closed() {
        for declared in [
            "Utf16",
            "FixedSizeBinary(",
            "FixedSizeBinary(-1)",
            "FixedSizeBinary(abc)",
            "FixedSizeBinary(12",
            "Decimal128(10,2)",
            "",
        ] {
            assert!(
                parse_declared_type(declared).is_err(),
                "expected {declared:?} to be rejected"
            );
        }
    }

    /// The derived Arrow schema must round-trip to the declaration exactly:
    /// same order, same names, same type spellings, same nullability.
    #[test]
    fn derived_schema_round_trips_to_declaration() {
        let schema = event_log_arrow_schema().expect("event log schema must be derivable");
        assert_schema_matches_declaration(&schema, EVENT_LOG_FIELDS)
            .unwrap_or_else(|reason| panic!("schema parity failed: {reason}"));
    }

    /// Field-level nullability must survive the derivation, because nullable
    /// columns are exactly what a downstream Arrow/Parquet reader will diverge
    /// on when it is wrong.
    #[test]
    fn nullability_is_preserved() {
        let schema = event_log_arrow_schema().expect("event log schema must be derivable");

        assert!(!schema.field(0).is_nullable(), "schema_version is required");
        assert!(!schema.field(1).is_nullable(), "run_id is required");
        assert!(
            schema.field(3).is_nullable(),
            "entity_id must stay nullable for pending events"
        );
        assert!(
            schema.field(10).is_nullable(),
            "payload_ref must stay nullable for events without a payload"
        );
    }

    /// The dependency-free fingerprint and the native schema must describe the
    /// same contract, otherwise the two encodings disagree about what an event
    /// log is.
    #[test]
    fn native_schema_agrees_with_dependency_free_fingerprint() {
        let schema = event_log_arrow_schema().expect("event log schema must be derivable");
        let fingerprint = event_log_schema_fingerprint();

        assert!(
            fingerprint.contains(&format!("major={ARROW_SCHEMA_MAJOR}")),
            "fingerprint must carry the same schema major as the Arrow schema"
        );

        for field in EVENT_LOG_FIELDS {
            let expected = format!(
                "{}:{}:{}",
                field.name,
                field.data_type,
                if field.nullable {
                    "nullable"
                } else {
                    "required"
                }
            );
            assert!(
                fingerprint.contains(&expected),
                "fingerprint is missing the declared entry {expected:?}"
            );
        }

        assert_eq!(
            schema.fields().len(),
            EVENT_LOG_FIELDS.len(),
            "native schema and declaration must describe the same number of fields"
        );
    }

    /// The default-off build must remain free of Arrow entirely. That property
    /// is enforced structurally by the `cfg` gate on this module, and is
    /// verified out of band by asserting the default-feature dependency graph
    /// contains no `arrow-*` or `parquet` package.
    #[test]
    fn arbitrary_declarations_round_trip() {
        let declaration = [
            EventLogField::new("id", "UInt64", false),
            EventLogField::new("label", "Utf8", true),
            EventLogField::new("digest", "FixedSizeBinary(32)", false),
            EventLogField::new("score", "Float64", true),
            EventLogField::new("active", "Boolean", false),
        ];

        let schema =
            event_log_arrow_schema_for(&declaration).expect("declaration must map to Arrow types");
        assert_schema_matches_declaration(&schema, &declaration)
            .unwrap_or_else(|reason| panic!("round trip failed: {reason}"));

        assert_eq!(
            declared_type_string(schema.field(2).data_type()),
            "FixedSizeBinary(32)"
        );
        assert!(schema.field(1).is_nullable());
        assert!(!schema.field(0).is_nullable());
    }
}
