use kairo_ecs_game_ontology::{
    normalize_ontology, parse_jsonld, parse_turtle, OntologyClass, OntologyDocument,
    OntologyProperty,
};

#[test]
fn normalizes_classes_and_properties_deterministically() {
    let doc = OntologyDocument {
        classes: vec![
            OntologyClass {
                id: s(&[103, 116, 58, 80, 108, 97, 121, 101, 114]),
            },
            OntologyClass {
                id: s(&[103, 116, 58, 71, 97, 109, 101]),
            },
            OntologyClass {
                id: s(&[103, 116, 58, 71, 97, 109, 101]),
            },
        ],
        properties: vec![
            OntologyProperty {
                id: s(&[103, 116, 58, 104, 97, 115, 80, 108, 97, 121, 101, 114]),
                domain: s(&[103, 116, 58, 71, 97, 109, 101]),
                range: s(&[103, 116, 58, 80, 108, 97, 121, 101, 114]),
            },
            OntologyProperty {
                id: s(&[103, 116, 58, 104, 97, 115, 80, 108, 97, 121, 101, 114]),
                domain: s(&[103, 116, 58, 71, 97, 109, 101]),
                range: s(&[103, 116, 58, 80, 108, 97, 121, 101, 114]),
            },
        ],
    };
    let normalized = normalize_ontology(doc);
    assert_eq!(normalized.classes.len(), 2);
    assert_eq!(normalized.properties.len(), 1);
    assert_eq!(
        normalized.classes[0].id.as_bytes(),
        &[103, 116, 58, 71, 97, 109, 101]
    );
}

#[test]
fn rejects_malformed_inputs() {
    assert!(parse_turtle(
        std::str::from_utf8(&[
            103, 116, 58, 71, 97, 109, 101, 32, 114, 100, 102, 115, 58, 67, 108, 97, 115, 115
        ])
        .unwrap()
    )
    .is_err());
    let bad_jsonld = [
        123, 10, 32, 32, 34, 105, 100, 34, 58, 32, 34, 103, 116, 58, 104, 97, 115, 80, 108, 97,
        121, 101, 114, 34, 44, 10, 32, 32, 34, 116, 121, 112, 101, 34, 58, 32, 34, 114, 100, 102,
        115, 58, 80, 114, 111, 112, 101, 114, 116, 121, 34, 10, 125, 10,
    ];
    assert!(parse_jsonld(std::str::from_utf8(&bad_jsonld).unwrap()).is_err());
}

fn s(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}
