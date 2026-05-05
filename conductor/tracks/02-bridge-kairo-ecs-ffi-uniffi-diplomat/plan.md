# 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read `conductor/product.md`, `conductor/tech-stack.md`, `conductor/workflow.md`, and the bridge and binding contracts under `conductor/contracts/`.
- Confirm owned paths: `crates/kairo-ecs-ffi`, `crates/kairo-ecs-uniffi`, `crates/kairo-ecs-diplomat`, `include/`, and the generated binding outputs that hang off those roots.
- Refresh `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md` to freeze the bridge surface against the current Rust core and the future language wrappers.

## Phase 1 — Contract alignment

- Freeze the Rust facade surface that the generated bindings consume: handles, status codes, ownership rules, and error translation.
- Propose contract changes through ADR if required.
- Add bridge fixture references only after Track 12 parity is confirmed.

## Phase 2 — Scaffold

- Extend the bridge crate skeleton with the public Rust facade and the language-specific wrapper entry points.
- Add bridge smoke tests for the Rust facade, generated headers, and binding wrapper edges.
- Add concrete bridge docs for the Rust facade, generated binding surfaces, and fixture parity checks.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice for the frozen Rust facade and one generated binding path.
- Add unit tests and integration tests.
- Add fixture parity checks only for the exported bridge surface.
- Add startup or wrapper overhead checks only if the bridge layer exposes a measurable cost.

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

