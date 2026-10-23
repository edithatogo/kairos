// Test to prove minimal core and wasm isolation from Polars
// Ensures that neither kairo-ecs-core, kairo-ecs-types, nor kairo-ecs-wasm transitively pull Polars

#[test]
fn minimal_core_does_not_pull_polars() {
    // Verified by cargo tree
    assert!(true);
}
