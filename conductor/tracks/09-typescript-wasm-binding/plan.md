# 09 TypeScript/Wasm Binding — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `bindings/typescript, crates/kairo-ecs-wasm, packaging/npm`.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add TypeScript/Wasm fixture references tied to the bundle boundary.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Add bundle smoke tests for the TypeScript/Wasm binding surface.
- Document the bundle boundary, Node/Wasm test path, and fixture bridge in the package README once the bundle boundary is in place.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add fixture bridge checks for the bundle output.
- Add bundle size or load timing checks only if the WebAssembly surface needs them.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record risks and follow-up tasks.
- Confirm CI gates.
- Mark track ready for integration.

## Phase 7 — WASI support

- Add `wasm32-wasip2` target to build configuration.
- Integrate `wasmtime` runtime for testing; verify `wasmer` compatibility.
- Implement WASI Preview 2 bindings for server-side sandboxed execution.
- Add WASI smoke test: load `kairo-ecs-wasm` module in `wasmtime` and execute a 10K event simulation.
- Document WASI deployment scenarios (cloud-native runners, edge, plugin systems).

