use crate::aas::submodel::AasSubmodel;
use crate::{
    error::{validation_error, FmiResult},
    FmiError,
};

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
        for submodel in &self.submodels {
            submodel.validate()?;
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
}
