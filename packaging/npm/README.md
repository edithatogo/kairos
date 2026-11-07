# npm packaging

The package loads an explicitly supplied generated Rust Wasm module. The old
JavaScript scheduler and synthetic inspector are retired. Build/typecheck,
metadata tests and browser failure controls do not qualify native execution.

A runtime release requires the generated module and Wasm artifact, pinned
provenance and the real `test:native` gate described in
[the binding guide](../../bindings/typescript/README.md). Required artifact
absence fails that gate. Native browser qualification remains required before
claiming browser runtime support.
