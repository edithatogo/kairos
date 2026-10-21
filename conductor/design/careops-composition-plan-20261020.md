# careops-sim-driven composition: bounded upstream follow-up plan

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

1. Audit exact current contracts/source and owner status; freeze K-C01/K-C02/K-C03
   semantics first. Reject stale base/input hashes when binding.
2. Implement the smallest neutral bundle and transfer fixtures under existing
   owners. K-C04 follows only after required event/identity contracts are settled.
3. Adapt CareOps ED → ward → community and birthing → bounded theatre → linked
   destinations. Two downstream consumers plus neutral fixtures qualify reuse.
4. Qualify K-C05/K-C06 with existing experiment/observation facilities and independent
   readers. Keep unrelated GPU/FMI/cluster work outside the headless dependency gate.
5. Run owner tests, conformance, compatibility and applicable Conductor/CI gates;
   record exact commits, inputs, outputs, tools and limits. Only then promote a
   downstream pin. API promotion/release remains separately reviewed.

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
