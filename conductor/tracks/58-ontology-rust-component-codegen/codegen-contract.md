# Track 58 Codegen Contract

## IR Input

- Input is the Track 57 OntologyDocument after normalize_ontology has sorted and deduplicated classes and properties.
- Class identifiers use compact ontology form such as gt:PayoffMatrix. The generator must reject identifiers without a namespace prefix.
- Property identifiers, domains, and ranges use the same compact form. Missing domain or range classes are contract errors.

## Rust Name Mapping

- Strip the namespace prefix before Rust naming. gt:PayoffMatrix maps from local name PayoffMatrix.
- Class local names must be valid UpperCamelCase Rust type names after normalization.
- Property local names must be valid lowerCamelCase ontology names and map to snake_case Rust field names.
- Reserved Rust keywords are suffixed with _field for fields and Type for type names, with the original ontology identifier retained in generated comments.
- Name collisions after normalization are hard errors. The generator must not silently rename colliding ontology terms.

## Component Shape

- Each ontology class maps to a Clone Debug PartialEq Rust component struct.
- Relationship properties map to Vec of Entity fields when the range class is an entity-backed ontology class.
- PayoffMatrix contains outcomes as a vector of PayoffOutcome values.
- StrategySpace contains strategies as a vector of StrategyId values.
- Utility contains a value field using f64.
- Identifier wrappers are transparent tuple structs around u64 for player, action, strategy, information set, and node identity.

## Determinism

- Generated modules are emitted in normalized class order, then helper identifiers in stable lexical order.
- Generated files are ASCII, rustfmt compatible, and free of timestamps, absolute paths, hostnames, tool versions, and nondeterministic hash iteration output.
- The generator must produce byte-identical output when run twice against equivalent Turtle and JSON-LD inputs.

## Feature Boundaries

- Generated game-theory components compile only through the game-theory crate or feature boundary selected by Track 58 Phase 1.
- Graph topology remains Entity-ID based. Generated relationship fields must not use raw pointers, self-referential structs, Box topology nodes, or reference cycles.
