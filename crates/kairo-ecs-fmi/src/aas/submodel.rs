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
    fn test_aas_property_to_json_no_semantic_id() {
        let property = AasProperty::new("prop1", "xs:string");
        let expected = r#"{"modelType":"Property","idShort":"prop1","valueType":"xs:string"}"#;
        assert_eq!(property.to_json(), expected);
    }

    #[test]
    fn test_aas_property_to_json_with_semantic_id() {
        let mut property = AasProperty::new("prop2", "xs:double");
        property.semantic_id = Some("http://acme.com/semantics/prop2".to_string());
        let expected = r#"{"modelType":"Property","idShort":"prop2","valueType":"xs:double","semanticId":{"type":"ExternalReference","keys":[{"type":"GlobalReference","value":"http://acme.com/semantics/prop2"}]}}"#;
        assert_eq!(property.to_json(), expected);
    }

    #[test]
    fn test_aas_submodel_to_json_empty() {
        let submodel = AasSubmodel::new("http://acme.com/submodels/sm1", "sm1");
        let expected = r#"{"type":"ModelReference","keys":[{"type":"Submodel","value":"http://acme.com/submodels/sm1"}],"idShort":"sm1","submodelElements":[]}"#;
        assert_eq!(submodel.to_json(), expected);
    }

    #[test]
    fn test_aas_submodel_to_json_with_elements() {
        let submodel = AasSubmodel::new("http://acme.com/sm2", "sm2")
            .with_property(AasProperty::new("prop1", "xs:string"))
            .with_property(AasProperty::new("prop2", "xs:int"));

        let expected = r#"{"type":"ModelReference","keys":[{"type":"Submodel","value":"http://acme.com/sm2"}],"idShort":"sm2","submodelElements":[{"modelType":"Property","idShort":"prop1","valueType":"xs:string"},{"modelType":"Property","idShort":"prop2","valueType":"xs:int"}]}"#;
        assert_eq!(submodel.to_json(), expected);
    }

    #[test]
    fn test_to_json_escapes_special_characters() {
        let mut property = AasProperty::new("prop\"1\"", "xs:string\\");
        property.semantic_id = Some("http://acme.com/\"semantic\"\\id".to_string());
        let submodel = AasSubmodel::new("sub\"model\"\\1", "sm\"1\"\\").with_property(property);

        let expected_property_json = r#"{"modelType":"Property","idShort":"prop\"1\"","valueType":"xs:string\\","semanticId":{"type":"ExternalReference","keys":[{"type":"GlobalReference","value":"http://acme.com/\"semantic\"\\id"}]}}"#;
        let expected_submodel_json = format!(
            r#"{{"type":"ModelReference","keys":[{{"type":"Submodel","value":"sub\"model\"\\1"}}],"idShort":"sm\"1\"\\","submodelElements":[{}]}}"#,
            expected_property_json
        );

        assert_eq!(submodel.to_json(), expected_submodel_json);
    }
}
