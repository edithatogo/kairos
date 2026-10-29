# Track 107: CareOps staged allocation flow compatibility

Status: Blocked pending exact-head PR #334 source integration and separate CareOps acceptance. The combined runtime/dependency candidate resolves the source audit blocker; Arrow crates.io publication remains separately disabled. Mainline integration base: `797e969b4a3cd3605a617a32d0134d0a093aa06a`; API prototype was reviewed on Track106 descendant `d3a6d831ddbe5886078541468e3fbf08b6f8c4c9`.

## Objective

Provide an explicitly named experimental DES runtime for the narrow staged resource-allocation contract consumed by CareOps R10.3. Do not restore the removed general-purpose `FlowRuntime` or imply its former surface is compatible.

## Inputs and outputs

Inputs: Kairos DES/core/types contracts; CareOps test `crates/careops-core/tests/allocation_kairos_conformance.rs` from parent commit `87439c3`.

Outputs: `StagedResourceRuntime`, deterministic contention/lease/queue behavior, focused conformance test and fixture, API review/ADR, compatibility and migration notes, and exact validation evidence.

## Owned paths

- `conductor/tracks/107-careops-staged-flow-compatibility/`
- `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml`, `conductor/status.md`, `conductor/track-map.md`, `conductor/implementation-readiness.md`
- `crates/kairo-ecs-des/src/lib.rs`, `crates/kairo-ecs-des/src/staged_runtime.rs`
- `crates/kairo-ecs-des/tests/careops_staged_allocation_compatibility.rs`
- `conformance/fixtures/manifest.json`, `conformance/fixtures/track107-careops-staged-allocation.json`
- `tests/conformance/runner.mjs`, `tests/conformance/runner-self-test.mjs`, `.github/workflows/conformance.yml` for registering and validating the new ready fixture
- `docs/adr/adr-track107-careops-staged-flow.md`, release compatibility/migration notes, `CHANGELOG.md`

## Blocked paths and exclusions

No scheduler ordering, RNG, parent pin/lock, ED model, allocation policy, FFI/binding, publication, or release changes. Excludes callbacks, checkpointing, preemption, async, AllOf/AnyOf, fidelity modes, and any claim to restore the old `FlowRuntime` API.

## Acceptance criteria

1. Capacity-one requests submitted at tick 1 produce one active lease and one FIFO queued request.
2. Lease identity includes request identity and a revision; release promotes the queued request through scheduled event dispatch.
3. Runtime snapshots expose active/queued state and resource counts needed by CareOps conformance.
4. Test executes behavior through Kairos `Scheduler`, with fixture agreement; no test-only fake state machine.
5. Rustdoc, API review, ADR, compatibility/migration notes clearly mark this API experimental and bounded.
6. Focused DES/core tests, conformance fixture validation, workspace checks, and reviewer-agent assessment pass on the exact candidate.

## Normative behavior

- IDs are monotonically allocated from zero within each runtime and never reused. Allocation uses checked arithmetic; exhaustion returns an error and leaves state unchanged.
- Opaque handles carry a process-unique runtime namespace so a handle from a different runtime cannot alias a same-index local handle. This namespace is identity protection only; it is excluded from simulation scheduling, serialized fixture values, and deterministic model outputs. Runtime namespace exhaustion makes construction fail.
- Actors and resources are created synchronously. Resource capacity must be positive; zero is rejected.
- `submit(resource, actor, at)` schedules one submit event. The scheduler's existing `(at, priority, sequence)` order determines same-tick arrival; equal-priority requests therefore enter FIFO by successful submission sequence.
- Request states are `Scheduled -> Queued -> Active -> Released`. A request is queued when its submit event dispatches and no slot is free; otherwise it becomes Active immediately. A queued request cannot be released directly.
- Every successful activation creates a lease with revision 1. A request has at most one activation and one lease for its lifetime; reacquisition is excluded.
- Submission time must be at or after scheduler time. Release requires an exact currently active lease belonging to this runtime. Foreign, stale, duplicate, queued, or not-yet-active releases return an error and change no state. The first valid release sets a pending flag synchronously; duplicate release attempts fail even before its event dispatch. Release time must be at or after scheduler time. A valid release schedules a release event at the supplied time; slot release and FIFO promotion occur only when that event dispatches.
- Same-time submit/release events use existing scheduler order `(at, priority, sequence)` with priority zero. Because insertion sequence is the final tie-break, an event scheduled earlier dispatches first; no special release-over-submit precedence is added.
- `run_for(n)` delegates event counting to Scheduler: at most `n` events are dispatched; zero dispatches none. Events at future times may dispatch and advance virtual time, matching core scheduler behavior.
- Resource snapshots report configured capacity, available slots, active leases in activation order, and queued request IDs in FIFO order. Request snapshots report actor, resource, state, and optional active lease. Readback has no side effects.
- Invalid handles and unsupported lifecycle operations return typed errors without partial mutation. This contract does not promise rollback for process termination.

## Dependencies

The implementation uses Scheduler dispatch directly and does not depend on Track106 `peek_next`; it is only an ancestor of this candidate. CareOps R10.3 must separately migrate its consumer and run its locked acceptance gates before changing the parent pin.

## Quality gates

Run focused crate tests, fmt, workspace check/test, fixture validation and conductor validators listed in `test-matrix.md`. The Rust root `crates/kairo-ecs-des` gains an experimental API; this does not reinstate the removed FlowRuntime compatibility promise and requires ADR/API review. No beta-or-later compatibility claim is authorized by this track.

## Release implications

This is an additive experimental Rust API in `crates/kairo-ecs-des`. It does not restore the removed `FlowRuntime` surface. CareOps must migrate source call sites and pass its own locked consumer acceptance before updating its parent pin. No beta-or-later release claim is authorized here.
