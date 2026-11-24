# Track 114 implementation plan

Reduce custom maintenance through qualified reuse and make Kairos a standalone dependency consumed by CareOps through explicit public contracts.

The 7 November ecosystem amendment also coordinates UOGTO semantics, the
existing Rust game incubator, Innovate and Voiage adapters, and neutral movement
and usage contracts. [ADR 0011](../../../docs/adr/0011-ecosystem-boundaries.md)
sets ownership; [consumer inventory](../../ecosystem-consumers.md) records actual
source relationships. Use the [bounded workpack](../../execution/README.md),
especially K01/K02, for the implementation leaves. This plan records their
completion status.

The 8 November refinement adds the [actual extraction map](../../contracts/game-extraction-map.md)
and [execution-port constraints](../../contracts/game-execution-port.md).
Receiving issue 100 / T17 owns generic game implementation; K01.games completes
the source/API disposition and K01.port binds the optional adapter to public
native contracts. Coordinate Innovate G01–G04 instead of creating another solver
or integration programme. Static game solving, Kairos core and dynamic adapter
acceptance run on separate capability paths. A pending optional game profile
must not block a runtime-only CareOps/Innovate receipt. These are pending
implementation/qualification obligations, not runtime acceptance from this plan.

Paignia is now the chosen game-library identity. The
[provider supplement](paignia-innovate-handoff.md) maps KP01–KP05 into existing
K01/114 tasks; those labels are acceptance refinements, not a second backlog.
The [CareOps source review](../../design/careops-composition-plan-20261020.md#paignia-and-careops-boundary-review)
routes shared healthcare cleanup downstream and generic experiment/RNG/resource
reuse to existing K-C02/K-C04/K-C05 owners. It creates no duplicate track and
does not change the 60-leaf workpack's pending statuses.

## Phase 0: Existing-state decisions

- [x] 114.0.1: Reconcile adoption, deferral and rejection across Tracks 70–104. [Disposition](../../dependency-and-consumer-contracts.md#tracks-70104-disposition--25-november-2026)
- [x] 114.0.2: Correct the crate-local unsafe-lint rationale. Retain the ECS identity, iteration, snapshot and borrowing obligations. [Track 84 decision](../84-hecs-robustness/handoff.md)
- [x] 114.0.3: Record the consumer, maintenance, build and runtime evidence and the conditions to reopen deferred dependencies. [Current assessment](handoff.md#consumer-contract-closeout--25-november-2026)
- [x] 114.0.review: Independently accept the reconciled adoption, rejection and deferral decisions. [Review evidence](handoff.md#phase-closeout-evidence)

## Phase 1: Independent core and adapters

- [x] 114.1.1: Bind the minimal Rust fixture, native-core-v1 ABI and versioned data boundary to the existing public contracts. [Minimal Rust contract](../../contracts/minimal-consumer.md) · [Native profile](../../release/beta-scope.md)
- [x] 114.1.2: Add a standalone installed downstream fixture that builds without CareOps or private paths/data. The [native install receipt](../113-beta-and-stable-release/handoff.md#native-core-v1-candidate--23-november-2026) covers the minimal Rust fixture.
- [x] 114.1.3: Accept the CareOps-owned adapter's immutable installed test pair and document shipping pin selection as a CareOps decision. [Receiving probe](handoff.md#installed-careops-movement--23-november-2026)
- [x] 114.1.4: Connect installed-consumer gates to candidate native dispatch, replay, schema and lifetime acceptance. Reuse the [Track 108 receipts](../108-scientific-ecosystem-alignment/handoff.md#installed-consumer-qualification--23-november-2026).
- [x] 114.1.5: Record the S01–S04 retirements and Paignia solver receipts, feature closure, and identity, chance, payoff and tie rules. [Source disposition](../../contracts/game-extraction-map.md) · [Receiving methods](../../scientific-completion.md#s01s04-paignia-transfer)
- [x] 114.1.6: Implement scheduled movement arrival, cancellation and topology-revision handling behind the neutral Rust contract. The movement and routing tests cover exact ticks, stale arrivals and native queue order.
- [x] 114.1.7: Implement admission/release intervals, atomic quantities, observation windows and committed-only export under optimistic replay. The replay fixture verifies a late release against canonical event order.
- [x] 114.1.8: Qualify the installed CareOps physical-movement case. The [receiving probe](handoff.md#installed-careops-movement--23-november-2026) combines the actual CareOps allocation coordinator with native movement, cancellation and measured porter occupancy.
- [x] 114.1.9: Qualify Innovate's native event dispatch, state, seeded replay and optional data features in the [installed Track 108 receiving wheel](handoff.md#reused-installed-consumer-evidence).
- [x] 114.1.10: Hand off the bounded native Paignia profile and the UOGTO and Voiage contracts. The receiving fixtures qualify owned observation, stale-action and lifetime rejection, separate randomness and checkpoint resume. [Exact handoff](../../contracts/game-port-acceptance.md#receiving-handoff--25-november-2026)
- [x] 114.1.review: Independently accept the fixed-scope Rust, CareOps, Innovate and optional Paignia contracts. [Review evidence](handoff.md#phase-closeout-evidence)

## Phase 2: Qualified reuse

- [x] 114.2.1: Record the measured worker-pool decision, named inference and FMI profiles, and the integrator deferral. [Dependency dispositions](../../dependency-and-consumer-contracts.md#tracks-70104-disposition--25-november-2026)
- [x] 114.2.2: Retain qualified Arrow, routing, optimization, serialization and native binding paths. Transfer the qualified reusable game methods to Paignia. [Reuse decisions](../../dependency-and-consumer-contracts.md#reuse-decisions)
- [x] 114.2.3: Verify the standalone normal/build graph, the pinned installed SDK targets and the separate optional game adapter. [Feature checks](test-matrix.md)
- [x] 114.2.4: Use the existing dependency PR workflow and version-specific compatibility gates. [Renovate](../../../renovate.json) · [Upgrade policy](../../dependency-value-policy.md#delivery-and-maintenance-refinements--24-october-2026)
- [x] 114.2.review: Verify the minimal graph and reject seeded domain and build-only dependencies. [Review evidence](handoff.md#phase-closeout-evidence)

## Phase 3: Acceptance

- [x] 114.3.1: Measure scoped source retirement, resolved build footprint and matched runtime cost. Verify independent consumer compatibility. [Measured evidence](handoff.md#consumer-contract-closeout--25-november-2026)
- [x] 114.3.2: Record supported profiles, reuse decisions and exact native, CareOps, Innovate and Paignia receipts. [Current assessment](handoff.md#consumer-contract-closeout--25-november-2026)
- [x] 114.3.review: Close the bounded maintenance and independent-consumer decision with the measured costs and explicit limits. [Review evidence](handoff.md#phase-closeout-evidence)

## Phase closeout gate

Apply the proportional workflow: one diff and focused results per routine leaf,
one specialist review per coherent contract/numerical/replay milestone, and
registry synchronization only when status changes. No repeated human approvals
or per-leaf risk/evidence dossiers. The integration checks below still apply at
actual phase closure.

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.

