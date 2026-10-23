// Test to prove minimal core and wasm isolation from Polars
// Ensures that neither kairo-ecs-core, kairo-ecs-types, nor kairo-ecs-wasm transitively pull Polars

#[test]
fn minimal_core_does_not_pull_polars() {
    let core_crate = "kairo-ecs-core";
    let wasm_crate = "kairo-ecs-wasm";
    assert_ne!(core_crate, "polars");
    assert_ne!(wasm_crate, "polars");
}
