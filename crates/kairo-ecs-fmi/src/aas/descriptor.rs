use crate::aas::submodel::AasSubmodel;

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
