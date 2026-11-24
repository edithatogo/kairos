use std::collections::HashSet;
use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use quick_xml::XmlVersion;

use crate::import::fmu_loader::{MAX_XML_BYTES, MAX_XML_DEPTH};
use crate::{
    error::{validation_error, FmiResult},
    FmiError,
};

const FMI2_UNKNOWN_SECTIONS: [&str; 2] = ["Outputs", "InitialUnknowns"];

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

    fn fmi2_unknown_indexes(&self) -> [Vec<usize>; 2] {
        let mut outputs = Vec::new();
        let mut initial_unknowns = Vec::new();
        for (position, variable) in self.variables.iter().enumerate() {
            if variable.causality == Causality::Output {
                let index = position + 1;
                outputs.push(index);
                // With no explicit initial attribute, constant outputs are exact.
                if variable.variability != Variability::Constant {
                    initial_unknowns.push(index);
                }
            }
        }
        [outputs, initial_unknowns]
    }

    pub fn to_fmi2_xml(&self) -> String {
        debug_assert!(self.validate().is_ok());
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(Cursor::new(&mut buffer), b' ', 2);

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .expect("write xml decl");

        let mut root = BytesStart::new("fmiModelDescription");
        root.push_attribute(("fmiVersion", "2.0"));
        root.push_attribute(("modelName", self.model_name.as_str()));
        root.push_attribute(("guid", self.guid.as_str()));
        root.push_attribute(("generationTool", self.generation_tool.as_str()));
        writer.write_event(Event::Start(root)).expect("write root");

        let mut cosim = BytesStart::new("CoSimulation");
        cosim.push_attribute(("modelIdentifier", "kairo_ecs_model"));
        writer
            .write_event(Event::Empty(cosim))
            .expect("write CoSimulation");

        writer
            .write_event(Event::Start(BytesStart::new("ModelVariables")))
            .expect("write ModelVariables");

        for variable in &self.variables {
            let mut var_elem = BytesStart::new("ScalarVariable");
            var_elem.push_attribute(("name", variable.name.as_str()));
            let vr_str = variable.value_reference.to_string();
            var_elem.push_attribute(("valueReference", vr_str.as_str()));
            var_elem.push_attribute(("causality", variable.causality.as_fmi2()));
            var_elem.push_attribute(("variability", variable.variability.as_fmi2()));
            writer
                .write_event(Event::Start(var_elem))
                .expect("write ScalarVariable");

            let type_elem = BytesStart::new(variable.variable_type.as_fmi2());
            writer
                .write_event(Event::Empty(type_elem))
                .expect("write variable type");

            writer
                .write_event(Event::End(BytesEnd::new("ScalarVariable")))
                .expect("write ScalarVariable end");
        }

        writer
            .write_event(Event::End(BytesEnd::new("ModelVariables")))
            .expect("write ModelVariables end");

        writer
            .write_event(Event::Start(BytesStart::new("ModelStructure")))
            .expect("write ModelStructure");

        for (name, indexes) in FMI2_UNKNOWN_SECTIONS
            .into_iter()
            .zip(self.fmi2_unknown_indexes())
        {
            if indexes.is_empty() {
                continue;
            }
            writer
                .write_event(Event::Start(BytesStart::new(name)))
                .expect("write variable indexes");
            for index in indexes {
                let mut unknown = BytesStart::new("Unknown");
                let idx_str = index.to_string();
                unknown.push_attribute(("index", idx_str.as_str()));
                writer
                    .write_event(Event::Empty(unknown))
                    .expect("write Unknown");
            }
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .expect("write variable indexes end");
        }

        writer
            .write_event(Event::End(BytesEnd::new("ModelStructure")))
            .expect("write ModelStructure end");

        writer
            .write_event(Event::End(BytesEnd::new("fmiModelDescription")))
            .expect("write fmiModelDescription end");

        let mut xml = String::from_utf8(buffer).expect("valid utf-8 XML");
        if !xml.ends_with('\n') {
            xml.push('\n');
        }
        xml
    }

    pub fn from_xml_str(xml: &str) -> FmiResult<Self> {
        if (xml.len() as u64) > MAX_XML_BYTES {
            return Err(validation_error(
                "modelDescription.xml",
                format!(
                    "XML exceeds maximum allowed size ({} bytes > {} bytes)",
                    xml.len(),
                    MAX_XML_BYTES
                ),
            ));
        }

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut depth: usize = 0;
        let mut model_name: Option<String> = None;
        let mut guid: Option<String> = None;
        let mut generation_tool: Option<String> = None;
        let mut fmi_version: Option<String> = None;
        let mut variables = Vec::new();
        let mut current_variable: Option<(
            String,
            u32,
            Causality,
            Variability,
            Option<VariableType>,
        )> = None;
        let mut in_model_variables = false;
        let mut seen_model_variables = false;
        let mut in_model_structure = false;
        let mut seen_model_structure = false;
        let mut in_outputs = false;
        let mut seen_outputs = false;
        let mut output_unknowns = Vec::new();
        let mut found_root = false;
        let mut root_closed = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    if depth > MAX_XML_DEPTH {
                        return Err(validation_error(
                            "modelDescription.xml",
                            format!("XML exceeds maximum nesting depth of {MAX_XML_DEPTH}"),
                        ));
                    }
                    if root_closed {
                        return Err(validation_error(
                            "modelDescription.xml",
                            format!(
                                "unexpected element <{}> after root element closed",
                                e.local_name().as_ref()
                            ),
                        ));
                    }
                    if !found_root {
                        if depth != 1 || e.local_name().as_ref() != "fmiModelDescription" {
                            return Err(validation_error(
                                "modelDescription.xml",
                                format!(
                                    "root element is <{}>, expected <fmiModelDescription> at document root",
                                    e.local_name().as_ref()
                                ),
                            ));
                        }
                        found_root = true;
                        for attr in e.attributes() {
                            let attr = attr.map_err(|err| {
                                validation_error(
                                    "modelDescription.xml",
                                    format!("malformed XML attribute: {err}"),
                                )
                            })?;
                            let key = attr.key.as_ref();
                            let val =
                                attr.normalized_value(XmlVersion::Explicit1_0)
                                    .map_err(|err| {
                                        validation_error(
                                            "modelDescription.xml",
                                            format!("malformed XML attribute value: {err}"),
                                        )
                                    })?;
                            match key {
                                "modelName" => model_name = Some(val.into_owned()),
                                "guid" => guid = Some(val.into_owned()),
                                "generationTool" => generation_tool = Some(val.into_owned()),
                                "fmiVersion" => fmi_version = Some(val.into_owned()),
                                _ => {}
                            }
                        }
                        continue;
                    }

                    match e.local_name().as_ref() {
                        "ModelVariables" if depth == 2 => {
                            if seen_model_variables {
                                return Err(validation_error(
                                    "modelDescription.xml",
                                    "duplicate ModelVariables element",
                                ));
                            }
                            seen_model_variables = true;
                            in_model_variables = true;
                        }
                        "ScalarVariable" if depth == 3 && in_model_variables => {
                            let mut name = None;
                            let mut vr = None;
                            let mut causality = Causality::Local;
                            let mut variability = Variability::Continuous;

                            for attr in e.attributes() {
                                let attr = attr.map_err(|err| {
                                    validation_error(
                                        "modelDescription.xml",
                                        format!("malformed ScalarVariable attribute: {err}"),
                                    )
                                })?;
                                let key = attr.key.as_ref();
                                let val = attr.normalized_value(XmlVersion::Explicit1_0).map_err(
                                    |err| {
                                        validation_error(
                                            "modelDescription.xml",
                                            format!(
                                                "malformed ScalarVariable attribute value: {err}"
                                            ),
                                        )
                                    },
                                )?;
                                match key {
                                    "name" => name = Some(val.into_owned()),
                                    "valueReference" => {
                                        vr = Some(val.parse::<u32>().map_err(|err| {
                                            validation_error(
                                                "modelDescription.xml",
                                                format!("invalid valueReference: {err}"),
                                            )
                                        })?);
                                    }
                                    "causality" => causality = Causality::from_fmi2(&val)?,
                                    "variability" => variability = Variability::from_fmi2(&val)?,
                                    _ => {}
                                }
                            }

                            let name = name.ok_or_else(|| {
                                validation_error(
                                    "modelDescription.xml",
                                    "ScalarVariable missing name attribute",
                                )
                            })?;
                            let vr = vr.ok_or_else(|| {
                                validation_error(
                                    "modelDescription.xml",
                                    "ScalarVariable missing valueReference attribute",
                                )
                            })?;

                            current_variable = Some((name, vr, causality, variability, None));
                        }
                        "Real" | "Integer" | "Boolean" | "String"
                            if depth == 4 && current_variable.is_some() =>
                        {
                            if let Some(ref mut var) = current_variable {
                                if var.4.is_some() {
                                    return Err(validation_error(
                                        "modelDescription.xml",
                                        format!(
                                            "ScalarVariable '{}' declares multiple type elements",
                                            var.0
                                        ),
                                    ));
                                }
                                var.4 = Some(VariableType::from_fmi2(e.local_name().as_ref())?);
                            }
                        }
                        "ModelStructure" if depth == 2 => {
                            if seen_model_structure {
                                return Err(validation_error(
                                    "modelDescription.xml",
                                    "duplicate ModelStructure element",
                                ));
                            }
                            seen_model_structure = true;
                            in_model_structure = true;
                        }
                        "Outputs" if depth == 3 && in_model_structure => {
                            if seen_outputs {
                                return Err(validation_error(
                                    "modelDescription.xml",
                                    "duplicate Outputs element in ModelStructure",
                                ));
                            }
                            seen_outputs = true;
                            in_outputs = true;
                        }
                        "Unknown" if depth == 4 && in_outputs => {
                            for attr in e.attributes() {
                                let attr = attr.map_err(|err| {
                                    validation_error(
                                        "modelDescription.xml",
                                        format!("malformed Unknown attribute: {err}"),
                                    )
                                })?;
                                if attr.key.as_ref() == "index" {
                                    let val = attr
                                        .normalized_value(XmlVersion::Explicit1_0)
                                        .map_err(|err| {
                                            validation_error(
                                                "modelDescription.xml",
                                                format!("malformed Unknown index attribute: {err}"),
                                            )
                                        })?;
                                    let idx = val.parse::<usize>().map_err(|err| {
                                        validation_error(
                                            "modelDescription.xml",
                                            format!("invalid Unknown index value: {err}"),
                                        )
                                    })?;
                                    output_unknowns.push(idx);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let local_name = e.local_name();
                    if root_closed {
                        return Err(validation_error(
                            "modelDescription.xml",
                            format!(
                                "unexpected element <{}> after root element closed",
                                local_name.as_ref()
                            ),
                        ));
                    }
                    if !found_root {
                        if depth != 0 || local_name.as_ref() != "fmiModelDescription" {
                            return Err(validation_error(
                                "modelDescription.xml",
                                format!(
                                    "root element is <{}>, expected <fmiModelDescription> at document root",
                                    local_name.as_ref()
                                ),
                            ));
                        }
                        found_root = true;
                        root_closed = true;
                        for attr in e.attributes() {
                            let attr = attr.map_err(|err| {
                                validation_error(
                                    "modelDescription.xml",
                                    format!("malformed XML attribute: {err}"),
                                )
                            })?;
                            let key = attr.key.as_ref();
                            let val =
                                attr.normalized_value(XmlVersion::Explicit1_0)
                                    .map_err(|err| {
                                        validation_error(
                                            "modelDescription.xml",
                                            format!("malformed XML attribute value: {err}"),
                                        )
                                    })?;
                            match key {
                                "modelName" => model_name = Some(val.into_owned()),
                                "guid" => guid = Some(val.into_owned()),
                                "generationTool" => generation_tool = Some(val.into_owned()),
                                "fmiVersion" => fmi_version = Some(val.into_owned()),
                                _ => {}
                            }
                        }
                        continue;
                    }

                    match local_name.as_ref() {
                        "ScalarVariable" if depth == 2 && in_model_variables => {
                            return Err(validation_error(
                                "modelDescription.xml",
                                "ScalarVariable cannot be self-closing without a type element",
                            ));
                        }
                        "Real" | "Integer" | "Boolean" | "String"
                            if depth == 3 && current_variable.is_some() =>
                        {
                            if let Some(ref mut var) = current_variable {
                                if var.4.is_some() {
                                    return Err(validation_error(
                                        "modelDescription.xml",
                                        format!(
                                            "ScalarVariable '{}' declares multiple type elements",
                                            var.0
                                        ),
                                    ));
                                }
                                var.4 = Some(VariableType::from_fmi2(local_name.as_ref())?);
                            }
                        }
                        "Unknown" if depth == 3 && in_outputs => {
                            for attr in e.attributes() {
                                let attr = attr.map_err(|err| {
                                    validation_error(
                                        "modelDescription.xml",
                                        format!("malformed Unknown attribute: {err}"),
                                    )
                                })?;
                                if attr.key.as_ref() == "index" {
                                    let val = attr
                                        .normalized_value(XmlVersion::Explicit1_0)
                                        .map_err(|err| {
                                            validation_error(
                                                "modelDescription.xml",
                                                format!("malformed Unknown index attribute: {err}"),
                                            )
                                        })?;
                                    let idx = val.parse::<usize>().map_err(|err| {
                                        validation_error(
                                            "modelDescription.xml",
                                            format!("invalid Unknown index value: {err}"),
                                        )
                                    })?;
                                    output_unknowns.push(idx);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && e.local_name().as_ref() == "fmiModelDescription" {
                        root_closed = true;
                    }
                    match e.local_name().as_ref() {
                        "ScalarVariable" if depth == 2 && in_model_variables => {
                            if let Some((name, vr, causality, variability, var_type)) =
                                current_variable.take()
                            {
                                let var_type = var_type.ok_or_else(|| {
                                    validation_error(
                                        "modelDescription.xml",
                                        format!(
                                            "ScalarVariable '{}' is missing a supported type (Real, Integer, Boolean, String)",
                                            name
                                        ),
                                    )
                                })?;
                                variables.push(ScalarVariable {
                                    name,
                                    value_reference: vr,
                                    causality,
                                    variability,
                                    variable_type: var_type,
                                });
                            }
                        }
                        "ModelVariables" if depth == 1 => {
                            in_model_variables = false;
                        }
                        "Outputs" if depth == 2 => {
                            in_outputs = false;
                        }
                        "ModelStructure" if depth == 1 => {
                            in_model_structure = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Ok(Event::Text(ref t)) => {
                    if (!found_root || root_closed) && !t.as_ref().trim().is_empty() {
                        return Err(validation_error(
                            "modelDescription.xml",
                            "unexpected text content outside root element",
                        ));
                    }
                }
                Ok(Event::Comment(_))
                | Ok(Event::Decl(_))
                | Ok(Event::DocType(_))
                | Ok(Event::PI(_))
                | Ok(Event::CData(_))
                | Ok(Event::GeneralRef(_)) => {}
                Err(err) => {
                    return Err(validation_error(
                        "modelDescription.xml",
                        format!("malformed XML: {err}"),
                    ));
                }
            }
        }

        if !found_root {
            return Err(validation_error(
                "modelDescription.xml",
                "missing fmiModelDescription root element",
            ));
        }

        if depth != 0 {
            return Err(validation_error(
                "modelDescription.xml",
                "unclosed XML tags in modelDescription",
            ));
        }

        let fmi_version = fmi_version.ok_or_else(|| {
            validation_error(
                "modelDescription.xml",
                "missing required fmiVersion attribute",
            )
        })?;
        if fmi_version != "2.0" {
            return Err(validation_error(
                "modelDescription.xml",
                format!("unsupported FMI version '{fmi_version}', expected 2.0"),
            ));
        }

        let model_name = model_name.ok_or_else(|| {
            validation_error(
                "modelDescription.xml",
                "missing required modelName attribute",
            )
        })?;
        let guid = guid.ok_or_else(|| {
            validation_error("modelDescription.xml", "missing required guid attribute")
        })?;
        let generation_tool = generation_tool.unwrap_or_else(|| "kairo-ecs-fmi".to_string());

        let model = ModelDescription {
            model_name,
            guid,
            generation_tool,
            variables,
        };

        model.validate()?;

        let output_count = model
            .variables
            .iter()
            .filter(|v| v.causality == Causality::Output)
            .count();
        if !output_unknowns.is_empty() && output_unknowns.len() != output_count {
            return Err(validation_error(
                "modelDescription.xml",
                format!(
                    "ModelStructure outputs count mismatch: {} Unknowns for {} output variables",
                    output_unknowns.len(),
                    output_count
                ),
            ));
        }

        Ok(model)
    }

    pub fn validate_generated_fmi2_xml(&self) -> FmiResult<()> {
        self.validate()?;
        self.validate_fmi2_xml(&self.to_fmi2_xml())
    }

    fn validate_fmi2_xml(&self, xml: &str) -> FmiResult<()> {
        let parsed = Self::from_xml_str(xml)?;
        if parsed != *self {
            return Err(validation_error(
                "modelDescription.xml",
                "parsed ModelDescription does not match generated ModelDescription",
            ));
        }

        let mut reader = Reader::from_str(xml);
        let mut actual = [Vec::new(), Vec::new()];
        let mut section = None;
        let mut found_cosim = false;
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    section = FMI2_UNKNOWN_SECTIONS
                        .iter()
                        .position(|&name| name == e.local_name().as_ref());
                }
                Ok(Event::End(_)) => section = None,
                Ok(Event::Empty(ref e)) => match e.local_name().as_ref() {
                    "CoSimulation" => found_cosim = true,
                    "Unknown" => {
                        if let Some(group) = section {
                            let index = e
                                .try_get_attribute("index")
                                .map_err(|err| {
                                    validation_error("modelDescription.xml", err.to_string())
                                })?
                                .ok_or_else(|| {
                                    validation_error(
                                        "modelDescription.xml",
                                        "missing Unknown index",
                                    )
                                })?
                                .normalized_value(XmlVersion::Explicit1_0)
                                .map_err(|err| {
                                    validation_error("modelDescription.xml", err.to_string())
                                })?
                                .parse::<usize>()
                                .map_err(|err| {
                                    validation_error("modelDescription.xml", err.to_string())
                                })?;
                            actual[group].push(index);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(err) => {
                    return Err(validation_error(
                        "modelDescription.xml",
                        format!("malformed XML in generated output: {err}"),
                    ));
                }
            }
        }

        if !found_cosim {
            return Err(validation_error(
                "modelDescription.xml",
                "generated XML missing CoSimulation element",
            ));
        }

        if actual != self.fmi2_unknown_indexes() {
            return Err(validation_error(
                "modelDescription.xml",
                "generated ModelStructure indexes differ from output and initialization variables",
            ));
        }

        Ok(())
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
    pub fn as_fmi2(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Local => "local",
            Self::Parameter => "parameter",
        }
    }

    pub fn from_fmi2(s: &str) -> FmiResult<Self> {
        match s {
            "input" => Ok(Self::Input),
            "output" => Ok(Self::Output),
            "local" => Ok(Self::Local),
            "parameter" => Ok(Self::Parameter),
            other => Err(validation_error(
                "modelDescription.xml",
                format!("unsupported Causality: '{other}'"),
            )),
        }
    }
}

impl Variability {
    pub fn as_fmi2(self) -> &'static str {
        match self {
            Self::Constant => "constant",
            Self::Fixed => "fixed",
            Self::Tunable => "tunable",
            Self::Discrete => "discrete",
            Self::Continuous => "continuous",
        }
    }

    pub fn from_fmi2(s: &str) -> FmiResult<Self> {
        match s {
            "constant" => Ok(Self::Constant),
            "fixed" => Ok(Self::Fixed),
            "tunable" => Ok(Self::Tunable),
            "discrete" => Ok(Self::Discrete),
            "continuous" => Ok(Self::Continuous),
            other => Err(validation_error(
                "modelDescription.xml",
                format!("unsupported Variability: '{other}'"),
            )),
        }
    }
}

impl VariableType {
    pub fn as_fmi2(self) -> &'static str {
        match self {
            Self::Real => "Real",
            Self::Integer => "Integer",
            Self::Boolean => "Boolean",
            Self::String => "String",
        }
    }

    pub fn from_fmi2(s: &str) -> FmiResult<Self> {
        match s {
            "Real" => Ok(Self::Real),
            "Integer" => Ok(Self::Integer),
            "Boolean" => Ok(Self::Boolean),
            "String" => Ok(Self::String),
            other => Err(validation_error(
                "modelDescription.xml",
                format!("unsupported VariableType: '{other}'"),
            )),
        }
    }
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
        assert!(xml.contains("<Real/>") || xml.contains("<Real />"));
        let (_, structure) = xml.split_once("<ModelStructure>").unwrap();
        let (outputs, initial) = structure
            .split_once("<InitialUnknowns>")
            .expect("the output uses the default calculated initialization");
        assert!(outputs.contains("<Outputs>"));
        assert!(outputs.contains("<Unknown index=\"2\"/>"));
        let (initial, _) = initial.split_once("</InitialUnknowns>").unwrap();
        assert_eq!(initial.trim(), "<Unknown index=\"2\"/>");
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

    #[test]
    fn generated_xml_requires_each_initial_unknown_index() {
        let model = ModelDescription::new("oscillator", "{kairo-test}")
            .with_variable(ScalarVariable::real_input("force", 1))
            .with_variable(ScalarVariable::real_output("position", 2));
        let xml = model.to_fmi2_xml();
        let (prefix, initial) = xml.split_once("<InitialUnknowns>").unwrap();

        for suffix in [
            initial.replace("<Unknown index=\"2\"/>", ""),
            initial.replace("index=\"2\"", "index=\"1\""),
            initial.replace("index=\"2\"", "index=\"bad\""),
            initial.replace(" index=\"2\"", ""),
        ] {
            let incomplete = format!("{prefix}<InitialUnknowns>{suffix}");
            assert!(model.validate_fmi2_xml(&incomplete).is_err());
        }
    }

    #[test]
    fn validates_generated_fmi2_xml_structure() {
        ModelDescription::new("oscillator", "{kairo-test}")
            .with_variable(ScalarVariable::real_input("force", 1))
            .with_variable(ScalarVariable::real_output("position", 2))
            .validate_generated_fmi2_xml()
            .expect("generated XML structure");
    }
}
