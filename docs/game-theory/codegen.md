# Ontology-to-Rust Component Code Generation

## Overview

Track 58 bridges the semantic ontology layer to the high-performance ECS runtime by deterministically compiling ontology classes and properties into strongly-typed Rust component structs.

## Code Generation Invariants

The generator (`kairo_ecs_game_ontology::codegen`) compiles normalized IR schemas according to strict rules:

1. **Deterministic Output**: Output Rust code formatting and struct ordering are bit-for-bit identical across runs.
2. **Safe ECS Integration**: Generated components derive standard traits (`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`) and store edge relations purely as `EntityId` descriptors rather than raw pointers or boxed references.
3. **Identifier Normalization**:
   - Class URIs map to `PascalCase` component names.
   - Property URIs map to `snake_case` field names.
   - Reserved Rust keywords (e.g. `type`, `move`, `match`) are escaped safely.

## Golden Artifact Pipeline

Generated components are written to:
`open-game-theory-ontology/fixtures/generated/rust/game_components.rs`

This file is included directly in `kairo-ecs-game-theory` when the `generated-components` feature is active:

```rust
#[cfg(feature = "generated-components")]
pub mod generated_components {
    include!("../../../open-game-theory-ontology/fixtures/generated/rust/game_components.rs");
}
```

## Validation & Verification

- `cargo check -p kairo-ecs-game-theory --features generated-components --tests`: Ensures generated components compile cleanly in the workspace.
- `node scripts/validation/validate-game-theory-codegen.mjs`: Validates that generated code matches the checked-in golden fixture with zero divergence.
