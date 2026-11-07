# careops-sim-driven composition: bounded upstream follow-up plan

Repository scope: **careops-sim** and its upstream **Kairos**; the separate
**careops** portfolio repository is not the target. Unqualified CareOps references
below mean careops-sim.

Authorised planning incorporation: 20 October 2026. All work below is pending.
This supplement routes requirements to existing owners; it neither changes their
historical status nor certifies completed APIs. Before implementation, the owner
must bind a reviewed packet and reconcile any required registry/phase transition.
Parallel update: Tracks 67–69 were registered in local commit
`0157d0cb60932b3cda86761954e1d0d9efebd55c`. Reuse those pending owners for
K-C01 (67), K-C03 (68) and K-C04 (69), with existing owners as reviewers.
Registration is not runtime acceptance; do not create duplicate tracks.

## Evidence and concurrent ownership

Inspected candidate: `f0d30e28c3299dcdb6ca138d95319ecaaf29a54a`.
CareOps currently pins `44261eb5cf8275f132f63a3396903d116d701dbe`.
The candidate declares `ConditionWait::AllOf/AnyOf`; this is not proof of event
execution or atomic grants. `Store<T>`'s synchronous container surface is not proof
of a queued process/resource contract. Require consumer tests at the actual pin.

A separate capability audit owns status/readiness/free-compute reconciliation.
This supplement does not alter its registry or hardware findings. Existing CLI
and game-theory writer leases also remain reserved, even when expired.
Use one writer per reviewed path and do not change runtime code in this pass.

## Contract boundaries

Engine contracts use neutral entity, job, resource, channel and observation terms.
CareOps retains person/episode meaning, maternal/infant relationships, clinical
progression/eligibility, staffing/triage policies, local targets, costs/funding and
outcome definitions. A second consumer and neutral example must justify extraction.
Avoid a dependency on game-theory solvers simply to use entity relationships;
Track 59/01 must review placement and compatibility before any extraction.

Use the existing scheduler, virtual time, RNG, Arrow, experiment and checkpoint
contracts. Do not introduce a second state authority or universal workflow engine.
User-facing methods below are semantic requirements, not invented existing APIs.

## Owner worklist and acceptance oracles

### K-C01 — Relationship and identity mechanics (67; 01/59 review, compatibility 25)

- [ ] Review neutral typed relationships, one-to-many cardinality and lifecycle ownership with the existing flat-ID graph representation; decide placement without forcing solver dependencies.
- [ ] Specify stable external identity mappings distinct from recycled generational ECS handles, deterministic creation and relationship cleanup.
- [ ] Test parent with multiple children, independent child completion, stale handle rejection, reparent/delete policy and checkpoint round trips; retain existing feature boundaries.

### K-C02 — Bundle grants and interruption (03/64, contracts 01/25)

- [ ] Separate event-condition completion, atomic command admission and atomic resource grant in the public contract.
- [ ] Define compatible multi-resource bundle reservation/commit/rollback or explicitly bounded staged alternatives; document fairness, timeout, cancellation, expiry and starvation policy.
- [ ] Write failing machine/operator/tool contention fixtures including partial failure, duplicate cancellation, timeout-at-grant ties and no leaked holds; implement only the accepted mechanism.
- [ ] Test non-interruptible work, safe interruption points, remaining-work/RNG ownership and retained dependencies. Clinical permission remains outside Kairos.

### K-C03 — Transfer and channel ownership (68; 03/01 review, 34/35 only for later distribution)

- [ ] Specify stable operation IDs, versioned envelopes, reserve/depart/arrive/acknowledge transitions, expiry, compensation and authority transfer.
- [ ] Qualify a two-depot single-process fixture with delayed/duplicate messages, reject-after-departure and no lost/duplicated jobs or prematurely released resources.
- [ ] Reuse checkpoint/recovery contracts; later demonstrate real process/transport failures separately. LP-per-service is a measured mapping, not the domain API.

### K-C04 — Progression, competing events and calendars (69; 03/64/21 review)

- [ ] Define state-dependent event/hazard rescheduling with explicit tie order, censoring, cancellation revisions and draw ownership while a job waits or travels.
- [ ] Test competing failure/completion/intervention events and observation-only agent decisions without looking ahead at sampled future outcomes.
- [ ] Evaluate neutral availability intervals and effective resource states; leave local calendar/time-zone and clinical policy interpretation in adapters unless neutral reuse is demonstrated.

### K-C05 — Reproducible composition and runner reuse (21/22/66, RNG owner 01)

- [ ] Inventory CareOps batch runner, manifests, cancellation and export transaction code; extract small libraries only where neutral consumers prove reuse, preserving CLI and filesystem independence.
- [ ] Extend existing stream contracts with explicit module/entity/process namespaces and draw ownership; qualify common-random-number alignment under policy changes and unrelated module additions.
- [ ] Verify 1/2/4 worker equivalence, bounded memory/backpressure, resume after interruption, deterministic result merge and incompatible checkpoint rejection; never infer PRNG equivalence from matching base seeds alone.

### K-C06 — Observation, validation and exchange (21/66, 04/62/22 review)

- [ ] Preserve latent state separately from recorded observations; define missing/delayed timestamps, exclusions, censoring and unobserved demand in neutral adapter contracts.
- [ ] Qualify synthetic recovery and held-out cases, non-identifiability controls, uncertainty decomposition and paired comparisons without claiming causal or clinical validity.
- [ ] Require actual Arrow IPC/Parquet round trips using independent official readers; names, magic bytes or custom payloads are insufficient interoperability evidence.
- [ ] Keep economic/fiscal semantics in downstream adapters. Version units, observation windows, aggregation and feedback lag before any multi-model coupling.

## Dispatch order and integration

1. Audit exact contracts/source and owner status; freeze K-C01/K-C02/K-C03
   semantics and the small experiment-lifecycle interface. Reject stale input hashes.
2. Qualify the smallest K-C05/CAP-22 runner extraction first: preserve the careops-sim
   facade, ED/logistics consumers, named streams, 1/2/4-worker parity, cancellation,
   committed output manifests and crash/restart recovery. Do not create a second runner.
3. Implement the neutral bundle/transfer fixtures under existing owners. K-C04
   follows only after consumed event/identity contracts are qualified. Keep domain
   resource migration behind the initial runner gate, as in careops-sim A0/A3.
4. Adapt ED → ward → community and birthing → bounded theatre → linked destinations.
   Two downstream consumers plus neutral fixtures qualify broader mechanism reuse.
5. Extend K-C05 to the broader experiment matrix and qualify K-C06 observation/format
   integration with independent readers. This later extension does not defer the
   initial runner proof from step2. Unrelated GPU/FMI/cluster work stays outside
   the headless dependency gate.
6. Run owner conformance/compatibility and applicable Conductor/CI gates; record
   exact commits, inputs, outputs, tools and limits. Promote a downstream pin only
   after its required gates pass. Public API/release acceptance remains separate.

## Cross-repository handoff

CareOps requirements: `conductor/design/composition-contracts-20261020.md`,
CC-01–CC-08, and R/A/B plans. R3 may retain the existing pin if required candidate
capabilities fail. A6 proves ED composition; B7 proves birthing composition; A7 joins
observation/evaluation evidence. A named primitive or registry Done label does not
satisfy any of those consumer gates.

The portfolio keeps source composition optional and independently runnable.
NHRA exchange starts offline with explicit periods, units and feedback lag;
NwauCore is a command wrapper, not a calculator to transplant into the kernel.

## Verification of this planning change

Run Conductor phase/DAG validation, local Markdown link checks and diff whitespace
checks. No Rust runtime change is made, so passing planning checks is not evidence
for any unchecked implementation item above. See the adjacent evidence record.

## Parallel capability audit alignment

Read back the separate audit commit `0cdc9ac` (20 October 2026). Preserve pinned
CareOps Q/C, physical IO, ED and checkpoint/recovery capabilities in a candidate
integration; mainline is not automatically a superset. Start K-C05's inventory
and smallest experiment-orchestration reuse before broader resource extraction,
while preserving its public facade and requiring two-consumer parity. This lane
is independently useful and does not wait for all new domain tracks. The audit
owns Mac/NAS/CUDA qualification and registry corrections; do not duplicate its
hardware plan or equate CPU fallback with device execution. When combining with
67–69 registration, recompute registry counts; its 67-track audit snapshot
predates those three registrations.

## Integrated review state — 21 October 2026

The capability audit at fc971d5 and this composition branch are combined in PR266's
integration candidate. Registry totals are now70 (60 recorded Done,7 In Progress,
3 Planned); earlier67-track totals remain dated audit evidence. The CAP tasks and
K-C worklist are complementary references, not duplicate engines or completion
claims. Track65/66 plan files are preserved from the audit baseline; their pending
CAP follow-up resides in handoff.md to avoid changing reserved worker inputs.
Track69's whole-track66 prerequisite is removed; only actually consumed experiment
capabilities require a handoff. All runtime reservations remain protected.

[Registration and dependency ADR](../tracks/29-wave-manager-execution-gatekeeper/adr-composition-registration-20261021.md) records the pending owner/dependency decision and unchanged historical statuses.

## Paignia and CareOps boundary review

Follow-up date: 8 November 2026 (Australia/Brisbane).

This amendment supersedes the current-pin interpretation of the older snapshots
above, while preserving their historical acceptance. Inspected CareOps main:
`a44a8c9f1461839c5306dbf44a1fa9a3b1981c50`; actual Kairos gitlink:
`07410266bf54b6f73c59db2f8cbfbc5697c53734`. Inspected upstream main:
`e76b97be2ea3712645933d903822ff6381c8883e`. Main is not automatically a compatible
replacement. This is source inspection; no new runtime or numerical result.

**Decision:** strengthen the existing `careops-core` package within careops-sim.
Its shared healthcare meaning is distinct from Kairos's neutral mechanics and
from specialty/site models. Independent packaging plus a second actual healthcare
consumer should precede a repository split. Paignia owns generic strategic-game
representation/algorithms; clinical game construction remains downstream. HEOR
values usage/outcomes and does not own scheduling, movement or allocation.

| Observed source / limitation | Existing owner and bounded next behavior |
| --- | --- |
| `careops-core/src/pathway/adapter.rs::build_ed_canonical_graph` and ED event/key helpers in `identity.rs` leak specialty assumptions | CareOps A0–A7: move the ED graph builder and ED-specific encodings behind `careops-ed` adapters; existing exact identity-byte fixtures must remain unchanged. Kairos gains no patient or ED terms. |
| `ConfigurablePathwayAdapter::run_simulation` scans patients for every tick to the horizon | CareOps pathway owner, K-C04 consumed contract: replace one transition family at a time with the single accepted scheduler; freeze equal-time/horizon/censoring behavior and measure sparse versus congested cases. No second production clock. |
| `pathway/streams.rs` implements an LCG while its comment says SplitMix | CareOps stream adapter with Kairos RNG owner: correct the label and specify an explicit algorithm/stream version before substitution. Map module/entity/purpose and checkpoint state; old fixtures characterize historical behavior, not a silently selectable alternative core. |
| `CompetingRiskModel` samples a unit-interval event probability and cause; Kairos has `CompetingHazards`/`SurvivalClock` | K-C04/Track69 plus CareOps: decide discrete versus continuous hazard semantics explicitly, including covariate update, residual work, tie order and censoring. Use constant-hazard analytic controls and model comparisons. A behavior-changing model revision is not cleanup. |
| `StagedAllocator` coordinates one-resource claims and conformance tests call `StagedResourceRuntime`; `kairo-ecs-des` is only a careops-core dev-dependency | K-C02 and CareOps allocation owner: exercise actual production grant/release/cancel, capacity, late grant, ties and revision handling through the public adapter. ED already uses the Kairos core scheduler. Staged coordination is not an atomic multi-resource grant. Eligibility/fairness/clinical interruption policy stays downstream. |
| ED→ward/community test runs complete ED then an independent single-bed oracle | CareOps A6 and K-C03/K-C04: add one live shared-scheduler fixture where downstream delay changes upstream occupancy while the run executes. Require no duplicated/lost person, leaked lease or causality violation; retain the independent oracle. |
| `careops-core/src/experiment/` owns generic replications, workers, cancel/checkpoints and atomic output; ED wraps domain results | K-C05/Tracks21/22/66: settle a small reusable library API and compare the existing Kairos CLI ensemble implementation before moving code. Keep ED result/metadata semantics downstream and do not depend on the CLI or calibration stack. |
| `careops-core/tests/logistics_consumer.rs` is an actual nonclinical experiment consumer | Reuse ED + logistics 1/2/4-worker, cancel and recovery cases for the narrow experiment transfer. These justify generic execution reuse, not a second healthcare consumer or whole-engine qualification. |

The authoritative downstream work remains
[`careops_core_domain_abstractions_20261020`](https://github.com/edithatogo/careops-sim/tree/main/conductor/tracks/careops_core_domain_abstractions_20261020)
and [`kairos_domain_contracts_20261107`](https://github.com/edithatogo/careops-sim/tree/main/conductor/tracks/kairos_domain_contracts_20261107).
Their bounded follow-up cards map onto A0–A7/R75–R77, preserving task IDs and
accepted receipts. K-C labels above retain upstream ownership; this table is
not another completion ledger or new approval process.

The initial generic experiment contract, ED helper cleanup and RNG semantic
inventory can progress independently of A7's second healthcare consumer. Each
implementation packet names at most five files and one distinguishing test;
unsettled numerical/model semantics receive one focused specialist decision
before local-model implementation. Routine work needs a diff and actual checks,
not a new risk dossier.

CareOps R76.5 is an optional Paignia operational-game profile. R76.6/R77 runtime
acceptance stays independent of it. R76.4's two valuations of the same neutral
usage record should expose price-year/perspective/discounting choices and bundled
cost double counting. Do not promote that fixture into a production HEOR engine.
Generic movement remains Kairos's optional transit lifecycle; healthcare route,
escort, staffing, handover and transport policy remain CareOps responsibilities.

Source references: [CareOps core](https://github.com/edithatogo/careops-sim/tree/a44a8c9f1461839c5306dbf44a1fa9a3b1981c50/crates/careops-core/src),
[CareOps tests](https://github.com/edithatogo/careops-sim/tree/a44a8c9f1461839c5306dbf44a1fa9a3b1981c50/crates/careops-core/tests),
[composition oracle](https://github.com/edithatogo/careops-sim/blob/a44a8c9f1461839c5306dbf44a1fa9a3b1981c50/crates/careops-ed/tests/ed_ward_community_composition.rs),
[Kairos hazard clock](https://github.com/edithatogo/kairos/blob/e76b97be2ea3712645933d903822ff6381c8883e/crates/kairo-ecs-des/src/hazard_clock.rs)
and [RNG/competing hazards](https://github.com/edithatogo/kairos/tree/e76b97be2ea3712645933d903822ff6381c8883e/crates/kairo-ecs-rng/src).
