# KairoECS Wave Policy

This policy prevents track work from being skipped or advanced out of order.

## Wave 0

Tracks:

- 00 Project Foundation, Governance & Naming
- 13 CI/CD, Code Quality & Supply Chain
- 14 Documentation Site & Education
- 16 Release Governance & Maintenance
- 27 Developer Experience & Reproducible Environments

Requirement:

- All Wave 0 tracks must have explicit owners, required artifacts, and a validation path before Wave 1 starts.

## Wave 1

Tracks:

- 01 The Heart: kairo-ecs-core & kairo-ecs-state
- 12 Conformance, Testing & Benchmarks

Requirement:

- Wave 1 may start only after Track 00 is accepted and Wave 0 control artifacts exist.

## Wave 2

Tracks:

- 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat
- 03 The Flow: DES Trajectory API & ABM Behavior API
- 04 The Analyst: kairo-ecs-arrow
- 05 The Window: kairo-ecs-viz

Requirement:

- Wave 2 may start only after the core contract inputs from Tracks 01 and 12 exist.

## Wave 3

Tracks:

- 06 Python Binding 3.10-3.14
- 07 R Binding
- 08 Julia Binding
- 09 TypeScript/Wasm Binding
- 10 C# Binding .NET 10-11
- 11 Go Binding

Requirement:

- Wave 3 may start only after Track 02 has a release-candidate facade and Track 12 has fixture stability.

## Wave 4

Tracks:

- 15 Packaging, Publishing & Delivery
- 17 Community Adoption, Education & Ecosystem
- 18 Comparative Benchmarks & Reproducibility
- 19 Research Software, Citation & Archival
- 20 OpenSSF, Supply Chain Trust & Institutional Readiness
- 21 Verification, Validation & Uncertainty
- 22 Experiment Runner & Scenario Management
- 23 Domain Starter Kits & Model Zoo
- 24 Playground, Demos & Visualization UX
- 25 API Design Review & Compatibility Governance
- 26 Interoperability Standards Review
- 28 Red Team & Devil's Advocate Review

Requirement:

- Wave 4 may only move when the release-gating tracks 20, 25, and 28 are current and no earlier-wave track is missing required artifacts.

## Wave 5

Tracks:

- 29 Wave Manager & Execution Gatekeeper
- 30 Toolchain & Version Support Matrix
- 31 Performance Regression Guard

Requirement:

- Wave 5 tracks are always-on control tracks. They may start immediately, but their required artifacts and validation paths must exist before the repo can claim release-readiness for governance, toolchain, or performance guarantees.

## No-skip controls

1. Every track must have `spec.md`, `plan.md`, `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.
2. Every track must appear in `conductor/tracks.yaml`.
3. Every track must have a declared owner and at least one required gate.
4. A track may not be marked `In Progress` unless its readiness level is documented.
5. A later wave may not start if an earlier wave has an unresolved required artifact.
6. Release tracks must treat missing evidence as a blocker rather than as a future enhancement.
