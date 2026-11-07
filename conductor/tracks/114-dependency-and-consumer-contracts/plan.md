# Track 114 implementation plan

Reduce custom maintenance through qualified reuse and make Kairos a standalone dependency consumed by CareOps through explicit public contracts.

The 7 November ecosystem amendment also coordinates UOGTO semantics, the
existing Rust game incubator, Innovate and Voiage adapters, and neutral movement
and usage contracts. [ADR 0011](../../../docs/adr/0011-ecosystem-boundaries.md)
sets ownership; [consumer inventory](../../ecosystem-consumers.md) records actual
source relationships. Use the [bounded workpack](../../execution/README.md),
especially K01/K02, for the next implementation leaves. It does not maintain
separate completion status. Every implementation item below remains pending.

The 8 November refinement adds the [actual extraction map](../../contracts/game-extraction-map.md)
and [execution-port constraints](../../contracts/game-execution-port.md).
Receiving issue 100 / T17 owns generic game implementation; K01.games completes
the source/API disposition and K01.port binds the optional adapter to public
native contracts. Coordinate Innovate G01–G04 instead of creating another solver
or integration programme. Static game solving, Kairos core and dynamic adapter
acceptance run on separate capability paths. A pending optional game profile
must not block a runtime-only CareOps/Innovate receipt. These are pending
implementation/qualification obligations, not runtime acceptance from this plan.

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
- [ ] 114.1.5: Complete the source/API extraction map into existing game core/solve and optional adapter; resolve identity/chance/payoff/tie differences, feature-enabled package closure and S01–S04 receiving acceptance. Keep solver diagnostics distinct from causal execution traces.
- [ ] 114.1.6: Implement scheduled movement arrival, cancellation and topology-revision handling behind the reviewed neutral contract; keep route queries pure and domain policies downstream.
- [ ] 114.1.7: Implement admission/release usage intervals, explicit quantities, window integration and committed/retracted replay semantics without costs or utilities in the engine.
- [ ] 114.1.8: Qualify movement/allocation composition through public APIs and downstream CareOps physical movement; keep HEOR valuation a separate model.
- [ ] 114.1.9: Qualify Innovate's actual native event/state/RNG dispatch and optional data features; printed imports and source metadata cannot close this item.
- [ ] 114.1.10: Hand off versioned game semantic and paired outcome contracts to UOGTO, the game library and Voiage; qualify lawful observation, stale-action/lifetime handling and separate solver/simulation RNG purposes through K01.port. Domain consumers own game/economic assumptions and provider pin changes.
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

Apply the proportional workflow: one diff and focused results per routine leaf,
one specialist review per coherent contract/numerical/replay milestone, and
registry synchronization only when status changes. No repeated human approvals
or per-leaf risk/evidence dossiers. The integration checks below still apply at
actual phase closure.

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.

