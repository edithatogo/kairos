#![cfg(feature = "codegen")]

use kairo_ecs_game_ontology::{
    generate_rust_components, parse_jsonld, parse_turtle, CodegenError, OntologyClass,
    OntologyDocument, OntologyProperty,
};

fn turtle_document() -> OntologyDocument {
    let mut document = parse_turtle(include_str!(
        "../../../open-game-theory-ontology/schemas/turtle/normal-form.ttl"
    ))
    .expect("normal-form Turtle parses");
    let extensive = parse_turtle(include_str!(
        "../../../open-game-theory-ontology/schemas/turtle/extensive-form.ttl"
    ))
    .expect("extensive-form Turtle parses");
    document.classes.extend(extensive.classes);
    document.properties.extend(extensive.properties);
    document
}

fn jsonld_document() -> OntologyDocument {
    let mut document = parse_jsonld(include_str!(
        "../../../open-game-theory-ontology/schemas/jsonld/normal-form.jsonld"
    ))
    .expect("normal-form JSON-LD parses");
    let extensive = parse_jsonld(include_str!(
        "../../../open-game-theory-ontology/schemas/jsonld/extensive-form.jsonld"
    ))
    .expect("extensive-form JSON-LD parses");
    document.classes.extend(extensive.classes);
    document.properties.extend(extensive.properties);
    document
}

#[test]
fn regenerates_checked_in_golden_components() {
    let generated = generate_rust_components(&turtle_document()).expect("codegen succeeds");
    let golden = include_str!(
        "../../../open-game-theory-ontology/fixtures/generated/rust/game_components.rs"
    );
    assert_eq!(generated, golden);
}

#[test]
fn generation_is_deterministic_across_equivalent_sources() {
    let first = generate_rust_components(&turtle_document()).expect("Turtle codegen succeeds");
    let second =
        generate_rust_components(&turtle_document()).expect("Turtle codegen is repeatable");
    let jsonld = generate_rust_components(&jsonld_document()).expect("JSON-LD codegen succeeds");
    assert_eq!(first, second);
    assert_eq!(first, jsonld);
}

#[test]
fn rejects_properties_without_known_domain_classes() {
    let document = OntologyDocument {
        classes: vec![OntologyClass {
            id: "gt:Game".to_owned(),
        }],
        properties: vec![OntologyProperty {
            id: "gt:hasPlayer".to_owned(),
            domain: "gt:Missing".to_owned(),
            range: "gt:Game".to_owned(),
        }],
    };
    assert_eq!(
        generate_rust_components(&document),
        Err(CodegenError::MissingClass {
            property: "gt:hasPlayer".to_owned(),
            class: "gt:Missing".to_owned(),
        })
    );
}

#[test]
fn rejects_names_without_a_namespace() {
    let document = OntologyDocument {
        classes: vec![OntologyClass {
            id: "Game".to_owned(),
        }],
        properties: Vec::new(),
    };
    assert_eq!(
        generate_rust_components(&document),
        Err(CodegenError::MissingNamespace("Game".to_owned()))
    );
}
