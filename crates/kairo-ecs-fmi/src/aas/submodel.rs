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
    fn test_aas_submodel_new() {
        let submodel = AasSubmodel::new("http://example.com/submodel", "MySubmodel");
        assert_eq!(submodel.id, "http://example.com/submodel");
        assert_eq!(submodel.id_short, "MySubmodel");
        assert!(submodel.elements.is_empty());
    }

    #[test]
    fn test_aas_property_new() {
        let property = AasProperty::new("MyProperty", "xs:string");
        assert_eq!(property.id_short, "MyProperty");
        assert_eq!(property.value_type, "xs:string");
        assert_eq!(property.semantic_id, None);
    }

    #[test]
    fn test_aas_submodel_with_property() {
        let property = AasProperty::new("MyProperty", "xs:string");
        let submodel = AasSubmodel::new("http://example.com/submodel", "MySubmodel")
            .with_property(property.clone());
        assert_eq!(submodel.elements.len(), 1);
        assert_eq!(submodel.elements[0], property);
    }

    #[test]
    fn test_aas_submodel_validate_success() {
        let property = AasProperty::new("MyProperty", "xs:string");
        let submodel =
            AasSubmodel::new("http://example.com/submodel", "MySubmodel").with_property(property);
        assert!(submodel.validate().is_ok());
    }

    #[test]
    fn test_aas_submodel_validate_duplicate_property() {
        let property = AasProperty::new("MyProperty", "xs:string");
        let submodel = AasSubmodel::new("http://example.com/submodel", "MySubmodel")
            .with_property(property.clone())
            .with_property(property);
        let result = submodel.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_aas_submodel_validate_empty_id() {
        let submodel = AasSubmodel::new("", "MySubmodel");
        let result = submodel.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_aas_property_validate_empty_id_short() {
        let property = AasProperty::new("", "xs:string");
        let result = property.validate();
        assert!(result.is_err());
    }
}
