use crate::aas::submodel::{escape_json, AasSubmodel};
use crate::{
    error::{validation_error, FmiResult},
    FmiError,
};
use std::collections::HashSet;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AasDescriptor {
    pub id: String,
    pub id_short: String,
    pub asset_kind: String,
    pub submodels: Vec<AasSubmodel>,
}

impl AasDescriptor {
    pub fn new(id: impl Into<String>, id_short: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            id_short: id_short.into(),
            asset_kind: "Instance".to_string(),
            submodels: Vec::new(),
        }
    }

    pub fn with_submodel(mut self, submodel: AasSubmodel) -> Self {
        self.submodels.push(submodel);
        self
    }

    pub fn validate(&self) -> FmiResult<()> {
        require_non_empty("AAS id", &self.id)?;
        require_non_empty("AAS idShort", &self.id_short)?;
        require_non_empty("asset kind", &self.asset_kind)?;
        let mut submodel_ids = HashSet::new();
        let mut submodel_id_shorts = HashSet::new();
        for submodel in &self.submodels {
            submodel.validate()?;
            if !submodel_ids.insert(submodel.id.as_str()) {
                return Err(validation_error(
                    "AAS descriptor",
                    format!("duplicate submodel id '{}'", submodel.id),
                ));
            }
            if !submodel_id_shorts.insert(submodel.id_short.as_str()) {
                return Err(validation_error(
                    "AAS descriptor",
                    format!("duplicate submodel idShort '{}'", submodel.id_short),
                ));
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        let submodels = self
            .submodels
            .iter()
            .map(AasSubmodel::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"assetAdministrationShells\":[{{\"id\":\"{}\",\"idShort\":\"{}\",\"assetInformation\":{{\"assetKind\":\"{}\"}},\"submodels\":[{}]}}]}}",
            escape_json(&self.id),
            escape_json(&self.id_short),
            escape_json(&self.asset_kind),
            submodels
        )
    }
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
    use crate::aas::submodel::AasProperty;

    #[test]
    fn validates_nested_submodels() {
        let descriptor = AasDescriptor::new("urn:kairo:test", "kairoTest").with_submodel(
            AasSubmodel::new("urn:kairo:test:state", "state")
                .with_property(AasProperty::new("queueDepth", "xs:integer")),
        );

        descriptor.validate().expect("valid descriptor");
        assert!(descriptor.to_json().contains("queueDepth"));
    }

    #[test]
    fn rejects_duplicate_submodel_ids() {
        let descriptor = AasDescriptor::new("urn:kairo:test", "kairoTest")
            .with_submodel(AasSubmodel::new("urn:kairo:test:state", "state"))
            .with_submodel(AasSubmodel::new("urn:kairo:test:state", "stateCopy"));

        let error = descriptor.validate().expect_err("duplicate submodel id");
        assert!(error.to_string().contains("duplicate submodel id"));
    }

    #[test]
    fn to_json_escapes_control_characters_and_preserves_unicode() {
        let control_value = "descriptor\nwith\ttab\u{0001} and café 🩺";
        let mut descriptor = AasDescriptor::new(control_value, control_value)
            .with_submodel(AasSubmodel::new(control_value, control_value));
        descriptor.asset_kind = control_value.to_string();

        let json = descriptor.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON output");
        let shell = &parsed["assetAdministrationShells"][0];
        assert_eq!(shell["id"], control_value);
        assert_eq!(shell["idShort"], control_value);
        assert_eq!(shell["assetInformation"]["assetKind"], control_value);
        assert_eq!(shell["submodels"][0]["keys"][0]["value"], control_value);
        assert_eq!(shell["submodels"][0]["idShort"], control_value);
        assert!(json.contains(r#"descriptor\nwith\ttab\u0001 and café 🩺"#));
    }
}
