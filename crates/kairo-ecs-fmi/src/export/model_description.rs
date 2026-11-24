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

#[cfg(feature = "fmi3")]
pub use fmi3_xml::Fmi3StartValue;

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

#[cfg(feature = "fmi3")]
mod fmi3_xml {
    use std::{borrow::Cow, collections::BTreeMap};

    use super::*;

    /// Scalar start value declared by an FMI 3.0 co-simulation model.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Fmi3StartValue<'a> {
        Float64(f64),
        Int32(i32),
        Boolean(bool),
        String(&'a str),
    }

    impl ModelDescription {
        /// Render an FMI 3.0 scalar co-simulation description.
        ///
        /// `guid` is the instantiation token. The binary must expose a
        /// Float64 `time` variable with the given reference. Inputs, parameters
        /// and constants require explicit start values keyed by value reference.
        /// A start value on a calculated output or local is an initialization
        /// guess. Floating-point start values must be finite.
        pub fn to_fmi3_co_simulation_xml(
            &self,
            time_reference: u32,
            start_values: &[(u32, Fmi3StartValue<'_>)],
        ) -> FmiResult<String> {
            self.validate()?;
            let mut starts = BTreeMap::new();
            for &(reference, value) in start_values {
                if starts.insert(reference, value).is_some() {
                    return Err(invalid(format!("duplicate start reference {reference}")));
                }
            }

            let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
            write(
                &mut writer,
                Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
            );
            let mut root = BytesStart::new("fmiModelDescription");
            root.push_attribute(("fmiVersion", "3.0"));
            push_text(&mut root, "modelName", &self.model_name)?;
            push_normalized_text(
                &mut root,
                "instantiationToken",
                &self.guid,
                "instantiation token",
            )?;
            push_normalized_text(
                &mut root,
                "generationTool",
                &self.generation_tool,
                "generation tool",
            )?;
            write(&mut writer, Event::Start(root));

            let mut cosim = BytesStart::new("CoSimulation");
            cosim.push_attribute(("modelIdentifier", "kairo_ecs_model"));
            write(&mut writer, Event::Empty(cosim));
            write(&mut writer, Event::Start(BytesStart::new("ModelVariables")));
            let mut time = BytesStart::new("Float64");
            let reference = time_reference.to_string();
            time.extend_attributes([
                ("name", "time"),
                ("valueReference", reference.as_str()),
                ("causality", "independent"),
                ("variability", "continuous"),
            ]);
            write(&mut writer, Event::Empty(time));
            for variable in &self.variables {
                if variable.name == "time" || variable.value_reference == time_reference {
                    return Err(invalid("variable conflicts with independent time"));
                }
                let start = starts.remove(&variable.value_reference);
                write_variable(&mut writer, variable, start)?;
            }
            if let Some((&reference, _)) = starts.first_key_value() {
                return Err(invalid(format!("unknown start reference {reference}")));
            }
            write(&mut writer, Event::End(BytesEnd::new("ModelVariables")));

            write(&mut writer, Event::Start(BytesStart::new("ModelStructure")));
            for name in ["Output", "InitialUnknown"] {
                for variable in self.variables.iter().filter(|variable| {
                    variable.causality == Causality::Output
                        && (name == "Output" || variable.variability != Variability::Constant)
                }) {
                    let mut entry = BytesStart::new(name);
                    let reference = variable.value_reference.to_string();
                    entry.push_attribute(("valueReference", reference.as_str()));
                    write(&mut writer, Event::Empty(entry));
                }
            }
            write(&mut writer, Event::End(BytesEnd::new("ModelStructure")));
            write(
                &mut writer,
                Event::End(BytesEnd::new("fmiModelDescription")),
            );
            let mut xml = String::from_utf8(writer.into_inner()).expect("valid UTF-8 XML");
            xml.push('\n');
            Ok(xml)
        }
    }

    fn write_variable(
        writer: &mut Writer<Vec<u8>>,
        variable: &ScalarVariable,
        start: Option<Fmi3StartValue<'_>>,
    ) -> FmiResult<()> {
        use Causality::*;
        use Variability::*;

        if !matches!(
            (variable.causality, variable.variability),
            (Local, _)
                | (Parameter, Fixed | Tunable)
                | (Input, Discrete | Continuous)
                | (Output, Constant | Discrete | Continuous)
        ) || (variable.variability == Continuous && variable.variable_type != VariableType::Real)
        {
            return Err(invalid(format!(
                "invalid FMI 3 variability for '{}'",
                variable.name
            )));
        }
        let exact =
            matches!(variable.causality, Input | Parameter) || variable.variability == Constant;
        let start = match (variable.variable_type, start) {
            (VariableType::Real, Some(Fmi3StartValue::Float64(value))) if value.is_finite() => {
                Some(Cow::Owned(value.to_string()))
            }
            (VariableType::Integer, Some(Fmi3StartValue::Int32(value))) => {
                Some(Cow::Owned(value.to_string()))
            }
            (VariableType::Boolean, Some(Fmi3StartValue::Boolean(value))) => {
                Some(Cow::Borrowed(if value { "true" } else { "false" }))
            }
            (VariableType::String, Some(Fmi3StartValue::String(value))) => {
                Some(Cow::Borrowed(value))
            }
            (_, None) if !exact => None,
            (_, None) => {
                return Err(invalid(format!(
                    "missing start value for '{}'",
                    variable.name
                )))
            }
            _ => {
                return Err(invalid(format!(
                    "invalid start value for '{}'",
                    variable.name
                )))
            }
        };

        let name = match variable.variable_type {
            VariableType::Real => "Float64",
            VariableType::Integer => "Int32",
            VariableType::Boolean => "Boolean",
            VariableType::String => "String",
        };
        let mut element = BytesStart::new(name);
        let reference = variable.value_reference.to_string();
        push_normalized_text(&mut element, "name", &variable.name, "variable name")?;
        element.extend_attributes([
            ("valueReference", reference.as_str()),
            ("causality", variable.causality.as_fmi2()),
            ("variability", variable.variability.as_fmi2()),
        ]);
        if start.is_some() && !exact {
            element.push_attribute(("initial", "approx"));
        }
        match (variable.variable_type, start) {
            (VariableType::String, Some(value)) => {
                write(writer, Event::Start(element));
                let mut start = BytesStart::new("Start");
                push_text(&mut start, "value", &value)?;
                write(writer, Event::Empty(start));
                write(writer, Event::End(BytesEnd::new(name)));
            }
            (_, start) => {
                if let Some(value) = start {
                    element.push_attribute(("start", value.as_ref()));
                }
                write(writer, Event::Empty(element));
            }
        }
        Ok(())
    }

    fn push_text(element: &mut BytesStart<'_>, name: &str, value: &str) -> FmiResult<()> {
        if value.chars().any(|c| {
            !matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}'
            | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
        }) {
            return Err(invalid("attribute contains an invalid XML 1.0 character"));
        }
        // Literal whitespace in an XML attribute is normalized by the parser.
        let escaped = quick_xml::escape::escape(value)
            .replace('\t', "&#9;")
            .replace('\n', "&#10;")
            .replace('\r', "&#13;");
        element.push_attribute(quick_xml::events::attributes::Attribute {
            key: quick_xml::name::QName(name),
            value: Cow::Borrowed(escaped.as_str()),
        });
        Ok(())
    }

    fn push_normalized_text(
        element: &mut BytesStart<'_>,
        name: &str,
        value: &str,
        label: &str,
    ) -> FmiResult<()> {
        if value.contains(['\t', '\n', '\r']) {
            return Err(invalid(format!(
                "invalid FMI 3 {label} with a tab or line break"
            )));
        }
        push_text(element, name, value)
    }

    fn invalid(message: impl Into<String>) -> FmiError {
        validation_error("modelDescription.xml", message)
    }

    fn write(writer: &mut Writer<Vec<u8>>, event: Event<'_>) {
        writer.write_event(event).expect("write to in-memory XML");
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn scalar_description_uses_fmi3_types_and_reference_order() {
            let model = ModelDescription::new("counter & clock", "<counter>")
                .with_variable(ScalarVariable::real_input("force", 17))
                .with_variable(ScalarVariable::real_output("result", 42))
                .with_variable(ScalarVariable {
                    name: "count".into(),
                    value_reference: 5,
                    causality: Causality::Output,
                    variability: Variability::Constant,
                    variable_type: VariableType::Integer,
                })
                .with_variable(ScalarVariable {
                    name: "enabled".into(),
                    value_reference: 3,
                    causality: Causality::Input,
                    variability: Variability::Discrete,
                    variable_type: VariableType::Boolean,
                })
                .with_variable(ScalarVariable {
                    name: "label".into(),
                    value_reference: 9,
                    causality: Causality::Parameter,
                    variability: Variability::Fixed,
                    variable_type: VariableType::String,
                });
            let xml = model
                .to_fmi3_co_simulation_xml(
                    0,
                    &[
                        (17, Fmi3StartValue::Float64(2.5)),
                        (3, Fmi3StartValue::Boolean(false)),
                        (5, Fmi3StartValue::Int32(-3)),
                        (42, Fmi3StartValue::Float64(7.0)),
                        (9, Fmi3StartValue::String("A&\n\"B\"")),
                    ],
                )
                .unwrap();
            for expected in [
                "fmiVersion=\"3.0\" modelName=\"counter &amp; clock\" instantiationToken=\"&lt;counter&gt;\"",
                "<Float64 name=\"time\" valueReference=\"0\" causality=\"independent\"",
                "<Float64 name=\"force\" valueReference=\"17\" causality=\"input\" variability=\"continuous\" start=\"2.5\"/>",
                "initial=\"approx\" start=\"7\"/>",
                "<Int32 name=\"count\" valueReference=\"5\" causality=\"output\" variability=\"constant\" start=\"-3\"/>",
                "<Boolean name=\"enabled\" valueReference=\"3\" causality=\"input\" variability=\"discrete\" start=\"false\"/>",
                "<Start value=\"A&amp;&#10;&quot;B&quot;\"/>",
                "<Output valueReference=\"42\"/>\n    <Output valueReference=\"5\"/>\n    <InitialUnknown valueReference=\"42\"/>",
            ] {
                assert!(xml.contains(expected), "missing {expected} in {xml}");
            }
        }

        #[test]
        fn rejects_inconsistent_scalar_metadata() {
            let model = ModelDescription::new("counter", "token")
                .with_variable(ScalarVariable::real_input("input", 3));
            let value = Fmi3StartValue::Float64(1.0);
            for (time, starts, expected) in [
                (0, &[][..], "missing start"),
                (3, &[(3, value)][..], "conflicts with independent time"),
                (
                    0,
                    &[(3, value), (3, value)][..],
                    "duplicate start reference",
                ),
                (0, &[(3, value), (4, value)][..], "unknown start reference"),
                (
                    0,
                    &[(3, Fmi3StartValue::Boolean(true))][..],
                    "invalid start",
                ),
                (
                    0,
                    &[(3, Fmi3StartValue::Float64(f64::NAN))][..],
                    "invalid start",
                ),
            ] {
                assert!(model
                    .to_fmi3_co_simulation_xml(time, starts)
                    .unwrap_err()
                    .to_string()
                    .contains(expected));
            }
            let mut bad_variability = model.clone();
            bad_variability.variables[0].variable_type = VariableType::Integer;
            assert!(bad_variability
                .to_fmi3_co_simulation_xml(0, &[(3, Fmi3StartValue::Int32(1))])
                .unwrap_err()
                .to_string()
                .contains("invalid FMI 3 variability"));
            let mut bad_name = model.clone();
            for name in ["bad\tname", "bad\nname", "bad\rname"] {
                bad_name.variables[0].name = name.into();
                assert!(bad_name
                    .to_fmi3_co_simulation_xml(0, &[(3, value)])
                    .unwrap_err()
                    .to_string()
                    .contains("invalid FMI 3 variable name"));
            }
            let mut bad_token = model.clone();
            bad_token.guid = "opaque\ttoken".into();
            assert!(bad_token
                .to_fmi3_co_simulation_xml(0, &[(3, value)])
                .unwrap_err()
                .to_string()
                .contains("invalid FMI 3 instantiation token"));
            bad_token.guid = "token\0".into();
            assert!(bad_token
                .to_fmi3_co_simulation_xml(0, &[(3, value)])
                .unwrap_err()
                .to_string()
                .contains("invalid XML 1.0 character"));
            let mut bad_tool = model;
            bad_tool.generation_tool = "tool\nversion".into();
            assert!(bad_tool
                .to_fmi3_co_simulation_xml(0, &[(3, value)])
                .unwrap_err()
                .to_string()
                .contains("invalid FMI 3 generation tool"));
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
