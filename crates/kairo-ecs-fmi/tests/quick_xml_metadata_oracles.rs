#![cfg(feature = "fmi2")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kairo_ecs_fmi::export::model_description::{ModelDescription, ScalarVariable, VariableType};
use kairo_ecs_fmi::import::fmu_loader::{
    current_fmi_platform, validate_unpacked_fmu_layout, MAX_XML_BYTES, MAX_XML_DEPTH,
};

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kairo-fmi-xml-oracle-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ))
}

fn shared_library_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

fn setup_unpacked_fmu(root: &Path, model_description_xml: &str) {
    let platform = current_fmi_platform();
    let binary_dir = root.join("binaries").join(&platform);
    fs::create_dir_all(&binary_dir).expect("create binary dir");
    fs::write(root.join("modelDescription.xml"), model_description_xml)
        .expect("write modelDescription.xml");
    fs::write(
        binary_dir.join(format!("model.{}", shared_library_extension())),
        b"mock_binary_content",
    )
    .expect("write binary");
}

#[test]
fn oracle_false_root_in_comment_rejected() {
    let root = temp_path("false-root-comment");
    // Substring checks would find <fmiModelDescription and fmiVersion="2.0",
    // but the true XML root element is <maliciousRoot>.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- <fmiModelDescription fmiVersion="2.0"> -->
<maliciousRoot>
  <payload />
</maliciousRoot>
"#;
    setup_unpacked_fmu(&root, xml);

    let result = validate_unpacked_fmu_layout(&root);
    assert!(result.is_err(), "Must reject false root in comment");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("fmiModelDescription root"),
        "Error message must indicate invalid root: {err_msg}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn oracle_false_version_in_comment_rejected() {
    let root = temp_path("false-version-comment");
    // Substring checks would find fmiVersion="2.0" in the comment,
    // but the real element declares unsupported fmiVersion="1.0".
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- fmiVersion="2.0" -->
<fmiModelDescription fmiVersion="1.0" modelName="legacy">
</fmiModelDescription>
"#;
    setup_unpacked_fmu(&root, xml);

    let result = validate_unpacked_fmu_layout(&root);
    assert!(result.is_err(), "Must reject unsupported FMI version");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not declare FMI version 2.0 or 3.0")
            || err_msg.contains("unsupported FMI version"),
        "Expected version error, got: {err_msg}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn oracle_fmi3_version_accepted() {
    let root = temp_path("fmi3-version-accepted");
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription fmiVersion="3.0" modelName="fmi3_model">
</fmiModelDescription>
"#;
    setup_unpacked_fmu(&root, xml);

    let result = validate_unpacked_fmu_layout(&root);
    assert!(result.is_ok(), "FMI 3.0 model description must be accepted");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn oracle_malformed_unclosed_xml_rejected() {
    let root = temp_path("malformed-unclosed");
    // Substring checks pass because markers exist, but XML syntax is broken
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription fmiVersion="2.0" modelName="broken">
  <ModelVariables>
    <ScalarVariable name="var1" valueReference="1">
  <!-- unclosed tags -->
"#;
    setup_unpacked_fmu(&root, xml);

    let result = validate_unpacked_fmu_layout(&root);
    assert!(result.is_err(), "Must reject malformed/unclosed XML");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("malformed") || err_msg.contains("unclosed"),
        "Error message should mention malformed or unclosed XML: {err_msg}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn oracle_xml_nesting_depth_budget_enforced() {
    let root = temp_path("depth-limit");
    let mut deeply_nested = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<fmiModelDescription fmiVersion=\"2.0\" modelName=\"deep\">\n");
    for i in 0..40 {
        deeply_nested.push_str(&format!("<nested level=\"{i}\">\n"));
    }
    for _ in 0..40 {
        deeply_nested.push_str("</nested>\n");
    }
    deeply_nested.push_str("</fmiModelDescription>\n");

    setup_unpacked_fmu(&root, &deeply_nested);

    let result = validate_unpacked_fmu_layout(&root);
    assert!(result.is_err(), "Must reject depth > {MAX_XML_DEPTH}");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("nesting depth"),
        "Error message should mention nesting depth: {err_msg}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn oracle_xml_file_size_budget_enforced() {
    let root = temp_path("size-limit");
    let platform = current_fmi_platform();
    let binary_dir = root.join("binaries").join(&platform);
    fs::create_dir_all(&binary_dir).expect("create binary dir");
    fs::write(
        binary_dir.join(format!("model.{}", shared_library_extension())),
        b"mock_binary_content",
    )
    .expect("write binary");

    let xml_path = root.join("modelDescription.xml");
    let file = fs::File::create(&xml_path).expect("create xml file");
    // Truncate/set length to MAX_XML_BYTES + 1024 without writing 50 MB
    file.set_len(MAX_XML_BYTES + 1024).expect("set_len");
    drop(file);

    let result = validate_unpacked_fmu_layout(&root);
    assert!(result.is_err(), "Must reject size > {MAX_XML_BYTES}");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeds maximum allowed size"),
        "Error message should mention size limit: {err_msg}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn oracle_roundtrip_serialization_and_attribute_order() {
    let original = ModelDescription::new("oscillator & <pendulum>", "{kairo-guid-\"123\"}")
        .with_variable(ScalarVariable::real_input("force & strain", 1))
        .with_variable(ScalarVariable::real_output("position \"x\"", 2));

    let xml = original.to_fmi2_xml();

    // Verify XML entities are properly escaped
    assert!(xml.contains("&amp;"));
    assert!(xml.contains("&quot;"));
    assert!(xml.contains("&lt;"));
    assert!(xml.contains("&gt;"));

    // Parse back using ModelDescription::from_xml_str
    let parsed = ModelDescription::from_xml_str(&xml).expect("parse back XML");
    assert_eq!(parsed.model_name, original.model_name);
    assert_eq!(parsed.guid, original.guid);
    assert_eq!(parsed.generation_tool, original.generation_tool);
    assert_eq!(parsed.variables.len(), original.variables.len());
    for (orig_var, parsed_var) in original.variables.iter().zip(parsed.variables.iter()) {
        assert_eq!(orig_var.name, parsed_var.name);
        assert_eq!(orig_var.value_reference, parsed_var.value_reference);
        assert_eq!(orig_var.causality, parsed_var.causality);
        assert_eq!(orig_var.variability, parsed_var.variability);
        assert_eq!(orig_var.variable_type, parsed_var.variable_type);
    }

    // Now test attribute order insensitivity: reorder attributes in root and variables
    let reordered_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription generationTool="kairo-ecs-fmi" guid="{kairo-guid-&quot;123&quot;}" modelName="oscillator &amp; &lt;pendulum&gt;" fmiVersion="2.0">
  <CoSimulation modelIdentifier="kairo_ecs_model" />
  <ModelVariables>
    <ScalarVariable variability="continuous" causality="input" valueReference="1" name="force &amp; strain">
      <Real />
    </ScalarVariable>
    <ScalarVariable causality="output" name="position &quot;x&quot;" variability="continuous" valueReference="2">
      <Real />
    </ScalarVariable>
  </ModelVariables>
  <ModelStructure>
    <Outputs>
      <Unknown index="2" />
    </Outputs>
  </ModelStructure>
</fmiModelDescription>
"#;
    let parsed_reordered =
        ModelDescription::from_xml_str(reordered_xml).expect("parse reordered XML");
    assert_eq!(parsed_reordered.model_name, original.model_name);
    assert_eq!(parsed_reordered.guid, original.guid);
    assert_eq!(parsed_reordered.variables, original.variables);
}

#[test]
fn oracle_validate_generated_fmi2_xml_oracle() {
    let model = ModelDescription::new("valid_model", "{guid-1}")
        .with_variable(ScalarVariable::real_input("u", 1))
        .with_variable(ScalarVariable::real_output("y", 2));

    model
        .validate_generated_fmi2_xml()
        .expect("must validate successfully");
}

#[test]
fn oracle_from_xml_str_rejects_nested_model_description() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<wrapper>
  <fmiModelDescription fmiVersion="2.0" modelName="nested" guid="{g1}">
  </fmiModelDescription>
</wrapper>
"#;
    let result = ModelDescription::from_xml_str(xml);
    assert!(result.is_err(), "Must reject nested fmiModelDescription");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("expected <fmiModelDescription> at document root"),
        "Error message must specify expected root, got: {err_msg}"
    );
}

#[test]
fn oracle_from_xml_str_parses_variable_types_in_start_and_end_tags() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription fmiVersion="2.0" modelName="types_test" guid="{types-guid}">
  <ModelVariables>
    <ScalarVariable name="int_var" valueReference="1" causality="input" variability="continuous">
      <Integer start="10"></Integer>
    </ScalarVariable>
    <ScalarVariable name="bool_var" valueReference="2" causality="output" variability="continuous">
      <Boolean></Boolean>
    </ScalarVariable>
    <ScalarVariable name="str_var" valueReference="3" causality="local" variability="continuous">
      <String start="abc" />
    </ScalarVariable>
  </ModelVariables>
  <ModelStructure>
    <Outputs>
      <Unknown index="2" />
    </Outputs>
  </ModelStructure>
</fmiModelDescription>
"#;
    let model =
        ModelDescription::from_xml_str(xml).expect("parse XML with explicit start/end type tags");
    assert_eq!(model.variables.len(), 3);
    assert_eq!(model.variables[0].variable_type, VariableType::Integer);
    assert_eq!(model.variables[1].variable_type, VariableType::Boolean);
    assert_eq!(model.variables[2].variable_type, VariableType::String);
}

#[test]
fn oracle_from_xml_str_rejects_unsupported_variable_type() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription fmiVersion="2.0" modelName="enum_test" guid="{enum-guid}">
  <ModelVariables>
    <ScalarVariable name="mode" valueReference="1" causality="input" variability="continuous">
      <Enumeration declaredType="Mode" />
    </ScalarVariable>
  </ModelVariables>
  <ModelStructure />
</fmiModelDescription>
"#;
    let result = ModelDescription::from_xml_str(xml);
    assert!(
        result.is_err(),
        "Must reject unsupported variable type Enumeration"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("missing a supported type"),
        "Expected unsupported type error, got: {err_msg}"
    );
}
