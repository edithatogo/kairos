# Conductor Setup Commands for KairoECS

This pack follows the current Conductor artifact model: `/conductor:setup` creates shared context files such as product, guidelines, tech stack, workflow, style guides, and a tracks index; `/conductor:newTrack` creates per-track `spec.md`, `plan.md`, and metadata artifacts.

## 1. Initialize Conductor

```text
/conductor:setup
```

Populate these setup artifacts from this pack:

```text
conductor/product.md
conductor/product-guidelines.md
conductor/tech-stack.md
conductor/workflow.md
conductor/code_styleguides/
conductor/tracks.md
conductor/track-map.md
conductor/subagents.md
conductor/parallel-execution.md
conductor/quality-gates.md
conductor/package-catalog.md
conductor/package-matrix.md
conductor/release-engineering.md
conductor/maintenance-governance.md
conductor/naming-due-diligence.md
conductor/red-team-review.md
conductor/devils-advocate-review.md
```

## 2. Create all tracks

```text
/conductor:newTrack "00 Project Foundation, Governance & Naming"
/conductor:newTrack "01 The Heart: kairo-ecs-core & kairo-ecs-state"
/conductor:newTrack "02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat"
/conductor:newTrack "03 The Flow: DES Trajectory API & ABM Behavior API"
/conductor:newTrack "04 The Analyst: kairo-ecs-arrow"
/conductor:newTrack "05 The Window: kairo-ecs-viz"
/conductor:newTrack "06 Python Binding 3.10-3.14"
/conductor:newTrack "07 R Binding"
/conductor:newTrack "08 Julia Binding"
/conductor:newTrack "09 TypeScript/Wasm Binding"
/conductor:newTrack "10 C# Binding .NET 10-11"
/conductor:newTrack "11 Go Binding"
/conductor:newTrack "12 Conformance, Testing & Benchmarks"
/conductor:newTrack "13 CI/CD, Code Quality & Supply Chain"
/conductor:newTrack "14 Documentation Site & Education"
/conductor:newTrack "15 Packaging, Publishing & Delivery"
/conductor:newTrack "16 Release Governance & Maintenance"
/conductor:newTrack "17 Community Adoption, Education & Ecosystem"
/conductor:newTrack "18 Comparative Benchmarks & Reproducibility"
/conductor:newTrack "19 Research Software, Citation & Archival"
/conductor:newTrack "20 OpenSSF, Supply Chain Trust & Institutional Readiness"
/conductor:newTrack "21 Verification, Validation & Uncertainty"
/conductor:newTrack "22 Experiment Runner & Scenario Management"
/conductor:newTrack "23 Domain Starter Kits & Model Zoo"
/conductor:newTrack "24 Playground, Demos & Visualization UX"
/conductor:newTrack "25 API Design Review & Compatibility Governance"
/conductor:newTrack "26 Interoperability Standards Review"
/conductor:newTrack "27 Developer Experience & Reproducible Environments"
/conductor:newTrack "28 Red Team & Devil's Advocate Review"
```

Use the matching files under `conductor/tracks/<nn-slug>/` as direct population material for each Conductor track.

## 3. Parallel execution rule

Do not wait for all implementation tracks before starting docs, CI, conformance, governance, or packaging. Use the contract files in `conductor/contracts/` as stable handoff artifacts.

The strict early dependency is:

```text
Track 00 -> Track 01 contracts -> parallel workstreams
```

After Track 01 contract phase, subagents can work in parallel on scheduler, ECS, FFI scaffolding, Arrow schemas, docs, CI, conformance fixtures, V&V/UQ, research metadata, and community examples.

## 4. Release discipline

Do not publish stable packages until:

```text
- naming/legal due diligence is complete
- release-specific conformance fixtures pass
- docs maturity labels match implementation maturity
- benchmark claims link to reproducible benchmark artifacts
- native artifacts include checksums/SBOM/provenance
- red-team review has no unresolved critical release blockers
```
