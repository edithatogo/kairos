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
    fn property_new() {
        let prop = AasProperty::new("myProp", "xs:string");
        assert_eq!(prop.id_short, "myProp");
        assert_eq!(prop.value_type, "xs:string");
        assert_eq!(prop.semantic_id, None);
    }

    #[test]
    fn property_validate_success() {
        let prop = AasProperty::new("myProp", "xs:string");
        assert!(prop.validate().is_ok());
    }

    #[test]
    fn property_validate_failure_empty_id_short() {
        let prop = AasProperty::new("", "xs:string");
        assert!(prop.validate().is_err());
    }

    #[test]
    fn property_validate_failure_empty_value_type() {
        let prop = AasProperty::new("myProp", "");
        assert!(prop.validate().is_err());
    }

    #[test]
    fn property_to_json_without_semantic_id() {
        let prop = AasProperty::new("myProp", "xs:string");
        let json = prop.to_json();
        assert_eq!(
            json,
            r#"{"modelType":"Property","idShort":"myProp","valueType":"xs:string"}"#
        );
    }

    #[test]
    fn property_to_json_with_semantic_id() {
        let mut prop = AasProperty::new("myProp", "xs:string");
        prop.semantic_id = Some("urn:kairo:test:semanticId".to_string());
        let json = prop.to_json();
        assert_eq!(
            json,
            r#"{"modelType":"Property","idShort":"myProp","valueType":"xs:string","semanticId":{"type":"ExternalReference","keys":[{"type":"GlobalReference","value":"urn:kairo:test:semanticId"}]}}"#
        );
    }

    #[test]
    fn submodel_new() {
        let submodel = AasSubmodel::new("urn:kairo:test:sm", "mySubmodel");
        assert_eq!(submodel.id, "urn:kairo:test:sm");
        assert_eq!(submodel.id_short, "mySubmodel");
        assert!(submodel.elements.is_empty());
    }

    #[test]
    fn submodel_with_property() {
        let prop = AasProperty::new("myProp", "xs:string");
        let submodel =
            AasSubmodel::new("urn:kairo:test:sm", "mySubmodel").with_property(prop.clone());
        assert_eq!(submodel.elements.len(), 1);
        assert_eq!(submodel.elements[0], prop);
    }

    #[test]
    fn submodel_validate_success() {
        let submodel = AasSubmodel::new("urn:kairo:test:sm", "mySubmodel")
            .with_property(AasProperty::new("prop1", "xs:string"))
            .with_property(AasProperty::new("prop2", "xs:integer"));
        assert!(submodel.validate().is_ok());
    }

    #[test]
    fn submodel_validate_failure_empty_id() {
        let submodel = AasSubmodel::new("", "mySubmodel");
        assert!(submodel.validate().is_err());
    }

    #[test]
    fn submodel_validate_failure_empty_id_short() {
        let submodel = AasSubmodel::new("urn:kairo:test:sm", "");
        assert!(submodel.validate().is_err());
    }

    #[test]
    fn submodel_validate_failure_duplicate_property() {
        let submodel = AasSubmodel::new("urn:kairo:test:sm", "mySubmodel")
            .with_property(AasProperty::new("prop1", "xs:string"))
            .with_property(AasProperty::new("prop1", "xs:integer"));
        let error = submodel.validate().unwrap_err();
        assert!(error
            .to_string()
            .contains("duplicate property idShort 'prop1'"));
    }

    #[test]
    fn submodel_to_json() {
        let submodel = AasSubmodel::new("urn:kairo:test:sm", "mySubmodel")
            .with_property(AasProperty::new("prop1", "xs:string"));
        let json = submodel.to_json();
        assert_eq!(
            json,
            r#"{"type":"ModelReference","keys":[{"type":"Submodel","value":"urn:kairo:test:sm"}],"idShort":"mySubmodel","submodelElements":[{"modelType":"Property","idShort":"prop1","valueType":"xs:string"}]}"#
        );
    }
}
