use std::collections::HashSet;

use crate::{
    error::{validation_error, FmiResult},
    FmiError,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModelDescription {
    pub model_name: String,
    pub guid: String,
    pub generation_tool: String,
    pub variables: Vec<ScalarVariable>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScalarVariable {
    pub name: String,
    pub value_reference: u32,
    pub causality: Causality,
    pub variability: Variability,
    pub variable_type: VariableType,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Causality {
    Input,
    Output,
    Local,
    Parameter,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Variability {
    Constant,
    Fixed,
    Tunable,
    Discrete,
    Continuous,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VariableType {
    Real,
    Integer,
    Boolean,
    String,
}

impl ModelDescription {
    pub fn new(model_name: impl Into<String>, guid: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            guid: guid.into(),
            generation_tool: "kairo-ecs-fmi".to_string(),
            variables: Vec::new(),
        }
    }

    pub fn with_variable(mut self, variable: ScalarVariable) -> Self {
        self.variables.push(variable);
        self
    }

    pub fn validate(&self) -> FmiResult<()> {
        require_non_empty("modelName", &self.model_name)?;
        require_non_empty("guid", &self.guid)?;
        require_non_empty("generationTool", &self.generation_tool)?;

        let mut names = HashSet::new();
        let mut value_references = HashSet::new();
        for variable in &self.variables {
            require_non_empty("ScalarVariable.name", &variable.name)?;
            if !names.insert(variable.name.as_str()) {
                return Err(validation_error(
                    "modelDescription.xml",
                    format!("duplicate ScalarVariable name '{}'", variable.name),
                ));
            }
            if !value_references.insert(variable.value_reference) {
                return Err(validation_error(
                    "modelDescription.xml",
                    format!(
                        "duplicate ScalarVariable valueReference {}",
                        variable.value_reference
                    ),
                ));
            }
        }

        Ok(())
    }

    pub fn to_fmi2_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(&format!(
            "<fmiModelDescription fmiVersion=\"2.0\" modelName=\"{}\" guid=\"{}\" generationTool=\"{}\">\n",
            escape_xml(&self.model_name),
            escape_xml(&self.guid),
            escape_xml(&self.generation_tool)
        ));
        xml.push_str("  <CoSimulation modelIdentifier=\"kairo_ecs_model\" />\n");
        xml.push_str("  <ModelVariables>\n");
        for variable in &self.variables {
            xml.push_str(&format!(
                "    <ScalarVariable name=\"{}\" valueReference=\"{}\" causality=\"{}\" variability=\"{}\"><{} /></ScalarVariable>\n",
                escape_xml(&variable.name),
                variable.value_reference,
                variable.causality.as_fmi2(),
                variable.variability.as_fmi2(),
                variable.variable_type.as_fmi2()
            ));
        }
        xml.push_str("  </ModelVariables>\n");
        xml.push_str("  <ModelStructure>\n");
        for (index, variable) in self.variables.iter().enumerate() {
            if variable.causality == Causality::Output {
                xml.push_str(&format!(
                    "    <Output valueReference=\"{}\" />\n",
                    index + 1
                ));
            }
        }
        xml.push_str("  </ModelStructure>\n");
        xml.push_str("</fmiModelDescription>\n");
        xml
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), FmiError> {
    if value.trim().is_empty() {
        Err(validation_error(
            "modelDescription.xml",
            format!("{field} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

impl ScalarVariable {
    pub fn real_input(name: impl Into<String>, value_reference: u32) -> Self {
        Self {
            name: name.into(),
            value_reference,
            causality: Causality::Input,
            variability: Variability::Continuous,
            variable_type: VariableType::Real,
        }
    }

    pub fn real_output(name: impl Into<String>, value_reference: u32) -> Self {
        Self {
            name: name.into(),
            value_reference,
            causality: Causality::Output,
            variability: Variability::Continuous,
            variable_type: VariableType::Real,
        }
    }
}

impl Causality {
    fn as_fmi2(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Local => "local",
            Self::Parameter => "parameter",
        }
    }
}

impl Variability {
    fn as_fmi2(self) -> &'static str {
        match self {
            Self::Constant => "constant",
            Self::Fixed => "fixed",
            Self::Tunable => "tunable",
            Self::Discrete => "discrete",
            Self::Continuous => "continuous",
        }
    }
}

impl VariableType {
    fn as_fmi2(self) -> &'static str {
        match self {
            Self::Real => "Real",
            Self::Integer => "Integer",
            Self::Boolean => "Boolean",
            Self::String => "String",
        }
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_fmi2_model_description_with_variables() {
        let xml = ModelDescription::new("oscillator", "{kairo-test}")
            .with_variable(ScalarVariable::real_input("force", 1))
            .with_variable(ScalarVariable::real_output("position", 2))
            .to_fmi2_xml();

        assert!(xml.contains("fmiVersion=\"2.0\""));
        assert!(xml.contains("name=\"position\""));
        assert!(xml.contains("<Real />"));
    }

    #[test]
    fn rejects_duplicate_value_references() {
        let error = ModelDescription::new("oscillator", "{kairo-test}")
            .with_variable(ScalarVariable::real_input("force", 1))
            .with_variable(ScalarVariable::real_output("position", 1))
            .validate()
            .expect_err("duplicate value reference");

        assert!(error
            .to_string()
            .contains("duplicate ScalarVariable valueReference 1"));
    }
}
