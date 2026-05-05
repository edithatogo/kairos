# 09 TypeScript/Wasm Binding — spec.md

## Mission

Provide npm/Wasm package for browser and Node with TypeScript types, Arrow JS support, and conformance tests.

## Primary subagent

```text
typescript-agent
```

## Dependencies

```text
Track 02 FFI RC; Track 04 Arrow schema.
```

## Owned paths

```text
bindings/typescript, crates/kairo-ecs-wasm, packaging/npm
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Stable C ABI from Track 02 FFI RC.
- Arrow schema from Track 04.
- Conformance fixtures from Track 12.
- `wasm-pack` toolchain.

## Outputs

- npm package in `bindings/typescript/`.
- Wasm module in `crates/kairo-ecs-wasm/`.
- Browser + Node.js smoke tests.

### WASI support (Phase 7)

Beyond browser/Node targets, the Wasm binding SHALL support WASI Preview 2 (`wasm32-wasip2`) for server-side sandboxed execution:

- Target: `wasm32-wasip2` in addition to `wasm32-unknown-unknown` (browser).
- Runtime: `wasmtime` for testing and deployment; `wasmer` as alternative.
- Use case: cloud-native simulation runners, edge deployment, sandboxed plugin systems.
- Acceptance: `kairo-ecs-wasm` module loads in `wasmtime` and executes a 10K event simulation.

## Blocked paths

- `crates/` (except `crates/kairo-ecs-wasm/`) — owned by Tracks 01–05.
- `bindings/` (except `bindings/typescript/`) — owned by other binding tracks.


## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.



