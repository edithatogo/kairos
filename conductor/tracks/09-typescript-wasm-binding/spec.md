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
bindings/typescript, crates/kairo-ecs-wasm
```

Package publishing, registry, and release dry-run work is explicitly out of
scope for this binding slice.

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

### Deferred runtime expansion

The current checked-in surface is a browser/Node contract crate and TypeScript
facade. Server-side sandbox runtimes are not claimed by this slice because no
checked-in runner or native artifact path exists yet.

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

## Release implications

This track contributes to release readiness only through the acceptance criteria and quality gates listed here and in conductor/quality-gates.md. It does not independently authorize public release, registry publication, or production-readiness claims without the dependent packaging, supply-chain, compatibility, red-team, and wave-management gates.
