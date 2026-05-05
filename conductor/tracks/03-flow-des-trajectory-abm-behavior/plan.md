# 03 The Flow: DES Trajectory API & ABM Behavior API — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`.
- Read the current shared scaffolds in `crates/kairo-ecs-core`, `crates/kairo-ecs-types`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, and the example README stubs under `examples/des/` and `examples/abm/`.
- Confirm the intended owned paths remain `crates/kairo-ecs-des`, `crates/kairo-ecs-abm`, and `examples/flow/`; those package directories are still pending, so this track starts as contract and API design work.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Define the split between the DES trajectory API and the ABM behavior API.
- Map the shared scheduler, time, and state semantics that both surfaces must preserve.
- Propose contract changes through ADR if required.
- Add fixture stubs for the event ordering and behavior-update paths that will later consume shared conformance fixtures.

## Phase 2 — Scaffold

- Create package/crate/module skeletons for the DES and ABM surfaces once the owned directories are introduced.
- Add DES/ABM smoke tests that prove the shared workspace checks are wired into CI.
- Add docs stubs that name the concrete DES trajectory and ABM behavior follow-ups without claiming the packages are complete.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice: a deterministic trajectory request/response path for DES and a single behavior-update loop for ABM.
- Add unit tests and integration tests.
- Add fixture parity checks where the flow API consumes Track 12 outputs.
- Add benchmarks where queue churn or event scheduling becomes performance-sensitive.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff, especially tracks 01, 04, 06-11, and 12.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record the remaining API decisions and follow-up tasks.
- Confirm CI gates.
- Mark the track ready for the next implementation wave, not as finished.

