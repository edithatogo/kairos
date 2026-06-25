use kairo_ecs_game_ontology::parse_turtle;

#[test]
fn parses_turtle_classes_and_properties() {
    let bytes = [
        103, 116, 58, 71, 97, 109, 101, 32, 97, 32, 114, 100, 102, 115, 58, 67, 108, 97, 115, 115,
        32, 46, 10, 103, 116, 58, 80, 108, 97, 121, 101, 114, 32, 97, 32, 114, 100, 102, 115, 58,
        67, 108, 97, 115, 115, 32, 46, 10, 103, 116, 58, 104, 97, 115, 80, 108, 97, 121, 101, 114,
        32, 97, 32, 114, 100, 102, 115, 58, 80, 114, 111, 112, 101, 114, 116, 121, 32, 59, 32, 114,
        100, 102, 115, 58, 100, 111, 109, 97, 105, 110, 32, 103, 116, 58, 71, 97, 109, 101, 32, 59,
        32, 114, 100, 102, 115, 58, 114, 97, 110, 103, 101, 32, 103, 116, 58, 80, 108, 97, 121,
        101, 114, 32, 46, 10,
    ];
    let text = std::str::from_utf8(&bytes).unwrap();
    let doc = parse_turtle(text).unwrap();
    assert_eq!(doc.classes.len(), 2);
    assert_eq!(doc.properties.len(), 1);
    assert_eq!(
        doc.classes[0].id.as_bytes(),
        &[103, 116, 58, 71, 97, 109, 101]
    );
    assert_eq!(
        doc.properties[0].domain.as_bytes(),
        &[103, 116, 58, 71, 97, 109, 101]
    );
    assert_eq!(
        doc.properties[0].range.as_bytes(),
        &[103, 116, 58, 80, 108, 97, 121, 101, 114]
    );
}
