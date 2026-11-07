# Track 114 implementation plan

Reduce custom maintenance through qualified reuse and make Kairos a standalone dependency consumed by CareOps through explicit public contracts.

## Phase 0: Existing-state decisions

- [ ] 114.0.1: Reconcile Tracks70–104 adoption/defer/no-go evidence; do not re-propose already adopted dependencies.
- [ ] 114.0.2: Correct inaccurate dependency-safety rationale and retain valid semantic/identity objections.
- [ ] 114.0.3: Record actual consumer/model need, eliminated code, build/runtime cost, licence/security and exit strategy for candidates.
- [ ] 114.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Independent core and adapters

- [ ] 114.1.1: Define the smallest public Rust crate/facade, ABI and schema contracts; keep heavyweight features optional.
- [ ] 114.1.2: Add a standalone installed downstream fixture that builds without CareOps or private paths/data.
- [ ] 114.1.3: Define the CareOps-owned domain adapter and immutable dependency selection; preserve current accepted pins until qualification.
- [ ] 114.1.4: Connect existing Track108 installed-consumer gates to candidate native dispatch/replay/schema/lifetime acceptance.
- [ ] 114.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Qualified reuse

- [ ] 114.2.1: Evaluate persistent pools, numerical/inference/FMI dependencies only for named accepted consumers.
- [ ] 114.2.2: Prefer generated bindings and existing Arrow/spatial/optimization libraries over duplicate custom wrappers.
- [ ] 114.2.3: Test default/minimal/optional feature closures and relevant native target build behaviour.
- [ ] 114.2.4: Automate reviewable dependency updates while gating behaviour-changing RNG/numerical/ABI/schema changes.
- [ ] 114.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 3: Acceptance

- [ ] 114.3.1: Measure net maintenance/runtime/build impact and independent consumer compatibility.
- [ ] 114.3.2: Record adoption or no-go decisions with evidence; update package/support contracts and exact consumer receipts.
- [ ] 114.3.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.

