use std::collections::HashSet;

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

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
    fn property_to_json_basic() {
        let prop = AasProperty::new("myProp", "xs:string");
        assert_eq!(
            prop.to_json(),
            r#"{"modelType":"Property","idShort":"myProp","valueType":"xs:string"}"#
        );
    }

    #[test]
    fn property_to_json_with_semantic_id() {
        let mut prop = AasProperty::new("myProp", "xs:string");
        prop.semantic_id = Some("urn:kairo:my-semantic-id".to_string());
        assert_eq!(
            prop.to_json(),
            r#"{"modelType":"Property","idShort":"myProp","valueType":"xs:string","semanticId":{"type":"ExternalReference","keys":[{"type":"GlobalReference","value":"urn:kairo:my-semantic-id"}]}}"#
        );
    }

    #[test]
    fn property_to_json_escapes_special_chars() {
        let prop = AasProperty::new("my\"Prop\\", "xs:\"string\\");
        assert_eq!(
            prop.to_json(),
            r#"{"modelType":"Property","idShort":"my\"Prop\\","valueType":"xs:\"string\\"}"#
        );
    }

    #[test]
    fn submodel_to_json_empty() {
        let submodel = AasSubmodel::new("urn:test:submodel", "mySubmodel");
        assert_eq!(
            submodel.to_json(),
            r#"{"type":"ModelReference","keys":[{"type":"Submodel","value":"urn:test:submodel"}],"idShort":"mySubmodel","submodelElements":[]}"#
        );
    }

    #[test]
    fn submodel_to_json_with_properties() {
        let submodel = AasSubmodel::new("urn:test:submodel", "mySubmodel")
            .with_property(AasProperty::new("prop1", "xs:string"))
            .with_property(AasProperty::new("prop2", "xs:integer"));
        assert_eq!(
            submodel.to_json(),
            r#"{"type":"ModelReference","keys":[{"type":"Submodel","value":"urn:test:submodel"}],"idShort":"mySubmodel","submodelElements":[{"modelType":"Property","idShort":"prop1","valueType":"xs:string"},{"modelType":"Property","idShort":"prop2","valueType":"xs:integer"}]}"#
        );
    }
}
