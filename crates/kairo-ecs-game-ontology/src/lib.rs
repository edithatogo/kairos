#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyClass {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyProperty {
    pub id: String,
    pub domain: String,
    pub range: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyDocument {
    pub classes: Vec<OntologyClass>,
    pub properties: Vec<OntologyProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
}

pub fn parse_turtle(input: &str) -> Result<OntologyDocument, ParseError> {
    let mut classes = Vec::new();
    let mut properties = Vec::new();
    for (index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line
            .as_bytes()
            .starts_with(&[64, 112, 114, 101, 102, 105, 120])
        {
            continue;
        }
        let subject = subject(line, index + 1)?;
        if contains(
            line.as_bytes(),
            &[
                32, 97, 32, 114, 100, 102, 115, 58, 67, 108, 97, 115, 115, 32, 46,
            ],
        ) {
            classes.push(OntologyClass {
                id: subject.to_owned(),
            });
            continue;
        }
        if contains(
            line.as_bytes(),
            &[
                32, 97, 32, 114, 100, 102, 115, 58, 80, 114, 111, 112, 101, 114, 116, 121, 32,
            ],
        ) {
            let domain = object_after(
                line,
                &[114, 100, 102, 115, 58, 100, 111, 109, 97, 105, 110],
                index + 1,
            )?;
            let range = object_after(
                line,
                &[114, 100, 102, 115, 58, 114, 97, 110, 103, 101],
                index + 1,
            )?;
            properties.push(OntologyProperty {
                id: subject.to_owned(),
                domain,
                range,
            });
            continue;
        }
        return Err(ParseError { line: index + 1 });
    }
    Ok(OntologyDocument {
        classes,
        properties,
    })
}

fn subject(line: &str, line_number: usize) -> Result<&str, ParseError> {
    line.split_whitespace()
        .next()
        .ok_or(ParseError { line: line_number })
}

fn object_after(line: &str, predicate: &[u8], line_number: usize) -> Result<String, ParseError> {
    let bytes = line.as_bytes();
    let start = find(bytes, predicate).ok_or(ParseError { line: line_number })? + predicate.len();
    let tail = &line[start..];
    let token = tail
        .split_whitespace()
        .next()
        .ok_or(ParseError { line: line_number })?;
    Ok(token.trim_end_matches(';').trim_end_matches('.').to_owned())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }
    let limit = haystack.len() - needle.len();
    for start in 0..=limit {
        if &haystack[start..start + needle.len()] == needle {
            return Some(start);
        }
    }
    None
}

pub fn parse_jsonld(input: &str) -> Result<OntologyDocument, ParseError> {
    let mut classes = Vec::new();
    let mut properties = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_kind = 0u8;
    let mut current_domain: Option<String> = None;
    let mut current_range: Option<String> = None;
    for (index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        let bytes = line.as_bytes();
        if contains(bytes, &[34, 105, 100, 34]) {
            current_id = Some(extract_gt_token(line, index + 1)?);
        }
        if contains(bytes, &[114, 100, 102, 115, 58, 67, 108, 97, 115, 115]) {
            current_kind = 1;
        }
        if contains(
            bytes,
            &[
                114, 100, 102, 115, 58, 80, 114, 111, 112, 101, 114, 116, 121,
            ],
        ) {
            current_kind = 2;
        }
        if contains(
            bytes,
            &[114, 100, 102, 115, 58, 100, 111, 109, 97, 105, 110],
        ) {
            current_domain = Some(extract_gt_token(line, index + 1)?);
        }
        if contains(bytes, &[114, 100, 102, 115, 58, 114, 97, 110, 103, 101]) {
            current_range = Some(extract_gt_token(line, index + 1)?);
        }
        if bytes.starts_with(&[125]) {
            if let Some(id) = current_id.take() {
                if current_kind == 1 {
                    classes.push(OntologyClass { id });
                } else if current_kind == 2 {
                    let domain = current_domain
                        .take()
                        .ok_or(ParseError { line: index + 1 })?;
                    let range = current_range.take().ok_or(ParseError { line: index + 1 })?;
                    properties.push(OntologyProperty { id, domain, range });
                }
            }
            current_kind = 0;
            current_domain = None;
            current_range = None;
        }
    }
    Ok(OntologyDocument {
        classes,
        properties,
    })
}

fn extract_gt_token(line: &str, line_number: usize) -> Result<String, ParseError> {
    let bytes = line.as_bytes();
    let start = find(bytes, &[103, 116, 58]).ok_or(ParseError { line: line_number })?;
    let mut end = start;
    while end < bytes.len() {
        let byte = bytes[end];
        if byte == 34 {
            break;
        }
        if byte == 44 {
            break;
        }
        if byte == 32 {
            break;
        }
        end += 1;
    }
    Ok(line[start..end].to_owned())
}

pub fn normalize_ontology(mut document: OntologyDocument) -> OntologyDocument {
    document
        .classes
        .sort_by(|left, right| left.id.cmp(&right.id));
    document.classes.dedup_by(|left, right| left.id == right.id);
    document.properties.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then(left.domain.cmp(&right.domain))
            .then(left.range.cmp(&right.range))
    });
    document.properties.dedup_by(|left, right| {
        left.id == right.id && left.domain == right.domain && left.range == right.range
    });
    document
}

#[cfg(feature = "codegen")]
mod codegen {
    use super::{normalize_ontology, OntologyDocument};
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum CodegenError {
        Collision(String),
        InvalidName(String),
        MissingClass { property: String, class: String },
        MissingNamespace(String),
    }

    pub fn generate_rust_components(document: &OntologyDocument) -> Result<String, CodegenError> {
        let normalized = normalize_ontology(document.clone());
        let mut classes = BTreeSet::new();
        for class in &normalized.classes {
            let local = local_name(&class.id)?;
            validate_type_name(&local)?;
            if !classes.insert(local) {
                return Err(CodegenError::Collision(class.id.clone()));
            }
        }

        let mut fields_by_domain: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for property in &normalized.properties {
            let domain = local_name(&property.domain)?;
            let range = local_name(&property.range)?;
            if !classes.contains(&domain) {
                return Err(CodegenError::MissingClass {
                    property: property.id.clone(),
                    class: property.domain.clone(),
                });
            }
            if !classes.contains(&range) {
                return Err(CodegenError::MissingClass {
                    property: property.id.clone(),
                    class: property.range.clone(),
                });
            }
            let property_name = local_name(&property.id)?;
            validate_field_source_name(&property_name)?;
            let field = to_snake_case(&property_name);
            validate_field_name(&field)?;
            fields_by_domain.entry(domain).or_default().push(field);
        }
        for fields in fields_by_domain.values_mut() {
            fields.sort();
            fields.dedup();
        }

        let mut emit_classes: Vec<String> = classes.into_iter().collect();
        if !emit_classes.iter().any(|class| class == "StrategySpace") {
            emit_classes.push("StrategySpace".to_owned());
            emit_classes.sort();
        }

        let mut out = String::new();
        out.push_str("// Generated by kairo-ecs-game-ontology codegen. Do not edit by hand.\n");
        out.push_str("// Source ontology fixtures: normal-form and extensive-form.\n\n");
        for wrapper in [
            "Entity",
            "PlayerId",
            "StrategyId",
            "ActionId",
            "InformationSetId",
            "GameNodeId",
        ] {
            out.push_str("#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]\n");
            out.push_str("pub struct ");
            out.push_str(wrapper);
            out.push_str("(pub u64);\n\n");
        }

        for class in emit_classes {
            match class.as_str() {
                "PayoffOutcome" => continue,
                "Action" => emit_struct(
                    &mut out,
                    "Action",
                    &["id: ActionId"],
                    fields_by_domain.get("Action"),
                ),
                "ChanceNode" => emit_struct(&mut out, "ChanceNode", &["id: GameNodeId"], None),
                "DecisionNode" => emit_struct(
                    &mut out,
                    "DecisionNode",
                    &["id: GameNodeId"],
                    fields_by_domain.get("DecisionNode"),
                ),
                "Game" => emit_struct(&mut out, "Game", &[], fields_by_domain.get("Game")),
                "InformationSet" => {
                    emit_struct(&mut out, "InformationSet", &["id: InformationSetId"], None)
                }
                "PayoffMatrix" => {
                    emit_struct(
                        &mut out,
                        "PayoffMatrix",
                        &["outcomes: Vec<PayoffOutcome>"],
                        fields_by_domain.get("PayoffMatrix"),
                    );
                    emit_struct(
                        &mut out,
                        "PayoffOutcome",
                        &["player: PlayerId", "strategy: StrategyId", "utility: f64"],
                        None,
                    );
                }
                "Player" => emit_struct(
                    &mut out,
                    "Player",
                    &["id: PlayerId"],
                    fields_by_domain.get("Player"),
                ),
                "Strategy" => emit_struct(&mut out, "Strategy", &["id: StrategyId"], None),
                "StrategySpace" => emit_struct(
                    &mut out,
                    "StrategySpace",
                    &["strategies: Vec<StrategyId>"],
                    None,
                ),
                "TerminalNode" => emit_struct(&mut out, "TerminalNode", &["id: GameNodeId"], None),
                "Transition" => emit_struct(&mut out, "Transition", &["to: GameNodeId"], None),
                "Utility" => emit_struct(&mut out, "Utility", &["value: f64"], None),
                other => emit_struct(&mut out, other, &[], fields_by_domain.get(other)),
            }
        }
        Ok(out)
    }

    fn emit_struct(
        out: &mut String,
        name: &str,
        fixed_fields: &[&str],
        relationship_fields: Option<&Vec<String>>,
    ) {
        out.push_str("#[derive(Clone, Debug, PartialEq)]\n");
        out.push_str("pub struct ");
        out.push_str(name);
        out.push_str(" {\n");
        for field in fixed_fields
            .iter()
            .filter(|field| !field.starts_with("outcomes:"))
        {
            out.push_str("    pub ");
            out.push_str(field);
            out.push_str(",\n");
        }
        if let Some(fields) = relationship_fields {
            for field in fields {
                out.push_str("    pub ");
                out.push_str(field);
                out.push_str(": Vec<Entity>,\n");
            }
        }
        for field in fixed_fields
            .iter()
            .filter(|field| field.starts_with("outcomes:"))
        {
            out.push_str("    pub ");
            out.push_str(field);
            out.push_str(",\n");
        }
        out.push_str("}\n\n");
    }

    fn local_name(id: &str) -> Result<String, CodegenError> {
        let (_, local) = id
            .split_once(':')
            .ok_or_else(|| CodegenError::MissingNamespace(id.to_owned()))?;
        if local.is_empty() {
            return Err(CodegenError::InvalidName(id.to_owned()));
        }
        Ok(local.to_owned())
    }

    fn validate_type_name(name: &str) -> Result<(), CodegenError> {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return Err(CodegenError::InvalidName(name.to_owned()));
        };
        if !first.is_ascii_uppercase() || !chars.all(|char| char.is_ascii_alphanumeric()) {
            return Err(CodegenError::InvalidName(name.to_owned()));
        }
        Ok(())
    }

    fn validate_field_source_name(name: &str) -> Result<(), CodegenError> {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return Err(CodegenError::InvalidName(name.to_owned()));
        };
        if !first.is_ascii_lowercase() || !chars.all(|char| char.is_ascii_alphanumeric()) {
            return Err(CodegenError::InvalidName(name.to_owned()));
        }
        Ok(())
    }

    fn validate_field_name(name: &str) -> Result<(), CodegenError> {
        if rust_keywords().contains(&name) {
            return Err(CodegenError::InvalidName(name.to_owned()));
        }
        Ok(())
    }

    fn to_snake_case(name: &str) -> String {
        let mut output = String::new();
        for (index, char) in name.chars().enumerate() {
            if char.is_ascii_uppercase() {
                if index > 0 {
                    output.push('_');
                }
                output.push(char.to_ascii_lowercase());
            } else {
                output.push(char);
            }
        }
        output
    }

    fn rust_keywords() -> &'static [&'static str] {
        &[
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
            "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
            "unsafe", "use", "where", "while",
        ]
    }
}

#[cfg(feature = "codegen")]
pub use codegen::{generate_rust_components, CodegenError};
