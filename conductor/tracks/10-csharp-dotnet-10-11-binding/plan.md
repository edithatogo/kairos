# 10 C# Binding .NET 10-11 — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `bindings/csharp`.
- Keep package publishing, registry, and release dry-run work out of this slice.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add .NET fixture references tied to the package boundary and SDK range.

## Phase 2 — Scaffold

- Create `bindings/csharp/Kairo.ECS.sln`, `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj`, and `bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj`.
- Add project smoke tests for the C# binding surface.
- Document the package boundary, supported SDK range, and fixture bridge in the project README once the package boundary is in place.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add fixture bridge checks for the exported C# APIs.
- Add build or load timing checks only if the .NET surface needs them.

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

