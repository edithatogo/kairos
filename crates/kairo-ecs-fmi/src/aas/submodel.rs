use std::collections::HashSet;
use std::fmt::Write as _;

use crate::{
    error::{validation_error, FmiResult},
    FmiError,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AasSubmodel {
    pub id: String,
    pub id_short: String,
    pub elements: Vec<AasProperty>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AasProperty {
    pub id_short: String,
    pub value_type: String,
    pub semantic_id: Option<String>,
}

impl AasSubmodel {
    pub fn new(id: impl Into<String>, id_short: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            id_short: id_short.into(),
            elements: Vec::new(),
        }
    }

    pub fn with_property(mut self, property: AasProperty) -> Self {
        self.elements.push(property);
        self
    }

    pub fn validate(&self) -> FmiResult<()> {
        require_non_empty("submodel id", &self.id)?;
        require_non_empty("submodel idShort", &self.id_short)?;

        let mut property_ids = HashSet::new();
        for property in &self.elements {
            property.validate()?;
            if !property_ids.insert(property.id_short.as_str()) {
                return Err(validation_error(
                    "AAS descriptor",
                    format!("duplicate property idShort '{}'", property.id_short),
                ));
            }
        }

        Ok(())
    }

    pub fn to_json(&self) -> String {
        let elements = self
            .elements
            .iter()
            .map(AasProperty::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"type\":\"ModelReference\",\"keys\":[{{\"type\":\"Submodel\",\"value\":\"{}\"}}],\"idShort\":\"{}\",\"submodelElements\":[{}]}}",
            escape_json(&self.id),
            escape_json(&self.id_short),
            elements
        )
    }
}

impl AasProperty {
    pub fn new(id_short: impl Into<String>, value_type: impl Into<String>) -> Self {
        Self {
            id_short: id_short.into(),
            value_type: value_type.into(),
            semantic_id: None,
        }
    }

    pub fn to_json(&self) -> String {
        let semantic_id = self.semantic_id.as_ref().map_or(String::new(), |value| {
            format!(
                ",\"semanticId\":{{\"type\":\"ExternalReference\",\"keys\":[{{\"type\":\"GlobalReference\",\"value\":\"{}\"}}]}}",
                escape_json(value)
            )
        });
        format!(
            "{{\"modelType\":\"Property\",\"idShort\":\"{}\",\"valueType\":\"{}\"{}}}",
            escape_json(&self.id_short),
            escape_json(&self.value_type),
            semantic_id
        )
    }

    pub fn validate(&self) -> FmiResult<()> {
        require_non_empty("property idShort", &self.id_short)?;
        require_non_empty("property valueType", &self.value_type)
    }
}

pub(super) fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{00}'..='\u{1F}' => {
                write!(escaped, "\\u{:04x}", character as u32)
                    .expect("writing to a String cannot fail");
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), FmiError> {
    if value.trim().is_empty() {
        Err(validation_error(
            "AAS descriptor",
            format!("{field} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_preserve_fields_and_with_property_appends_in_order() {
        let first = AasProperty::new("queueDepth", "xs:integer");
        let second = AasProperty {
            semantic_id: Some("urn:kairo:queue-depth".to_string()),
            ..AasProperty::new("queueName", "xs:string")
        };
        let submodel = AasSubmodel::new("urn:kairo:queue", "queue")
            .with_property(first.clone())
            .with_property(second.clone());

        assert_eq!(submodel.id, "urn:kairo:queue");
        assert_eq!(submodel.id_short, "queue");
        assert_eq!(submodel.elements, [first, second]);
    }

    #[test]
    fn property_validation_rejects_empty_and_whitespace_only_fields() {
        for (id_short, value_type, message) in [
            ("", "xs:string", "property idShort"),
            (" \t\n", "xs:string", "property idShort"),
            ("valid", "", "property valueType"),
            ("valid", " \t\n", "property valueType"),
        ] {
            let error = AasProperty::new(id_short, value_type)
                .validate()
                .expect_err("empty or whitespace-only property fields are invalid");
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn submodel_validation_rejects_empty_fields_and_invalid_or_duplicate_properties() {
        for (submodel, message) in [
            (AasSubmodel::new("", "valid"), "submodel id"),
            (AasSubmodel::new("valid", " \t\n"), "submodel idShort"),
            (
                AasSubmodel::new("valid", "valid").with_property(AasProperty::new("", "xs:string")),
                "property idShort",
            ),
            (
                AasSubmodel::new("valid", "valid")
                    .with_property(AasProperty::new("same", "xs:string"))
                    .with_property(AasProperty::new("same", "xs:integer")),
                "duplicate property idShort",
            ),
        ] {
            let error = submodel
                .validate()
                .expect_err("invalid submodel descriptor must be rejected");
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn to_json_escapes_quotes_and_backslashes_in_all_serialized_string_fields() {
        let mut property = AasProperty::new("prop\"quoted\\", "xs:string\\");
        property.semantic_id = Some("urn:semantic:\"quoted\"\\path".to_string());
        let submodel = AasSubmodel::new("urn:submodel:\"quoted\"\\path", "sub\"quoted\\")
            .with_property(property);

        assert_eq!(
            submodel.to_json(),
            r#"{"type":"ModelReference","keys":[{"type":"Submodel","value":"urn:submodel:\"quoted\"\\path"}],"idShort":"sub\"quoted\\","submodelElements":[{"modelType":"Property","idShort":"prop\"quoted\\","valueType":"xs:string\\","semanticId":{"type":"ExternalReference","keys":[{"type":"GlobalReference","value":"urn:semantic:\"quoted\"\\path"}]}}]}"#
        );
    }

    #[test]
    fn to_json_escapes_control_characters_and_preserves_unicode() {
        let control_value = "line\nwith\ttab\u{0001} and café 🩺";
        let property = AasProperty::new(control_value, "xs:string");
        let submodel = AasSubmodel::new(control_value, control_value).with_property(property);

        let json = submodel.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON output");
        assert_eq!(parsed["keys"][0]["value"], control_value);
        assert_eq!(parsed["idShort"], control_value);
        assert_eq!(parsed["submodelElements"][0]["idShort"], control_value);
        assert_eq!(parsed["submodelElements"][0]["valueType"], "xs:string");
        assert!(json.contains(r#"line\nwith\ttab\u0001 and café 🩺"#));
    }
}
