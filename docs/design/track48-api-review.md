# API Review: Track 48 local event-owned optimistic runtime

## Intake and affected surface

| Field | Value |
|---|---|
| Proposed by | Track 48 implementation programme |
| Affected root | `crates/kairo-ecs-pdes` |
| Surface family | `rust_api` |
| Current status / release stage | Opt-in feature preview / alpha |
| Compatibility level | Compatible additive API; focused legacy defect repairs |
| Decision | Accepted for bounded local implementation; distributed/release acceptance pending |

This review uses the [API design form](../api/api-review-template.md) and
[compatibility form](api-review-template.md). The existing PDES crate is outside
the current protected-surface inventory; no inventoried Rust root, ABI, Arrow
schema or binding root changes. This record does not enroll a new protected
root, change compatibility policy or grant a release/dependency exception.

## Problem and proposed change

The legacy `TimeWarpRuntime` helper used additive state and tick-only rollback,
without owned downstream sends or surviving-input replay. It could lose initialized
components and revive stale tokens. The new `time-warp`-gated Rust API adds
`OptimisticProcess`, `OptimisticRuntime`, immutable logical/order/message types,
limits, progress/report/fossil records, typed errors, state tokens and a bounded
PDES-owned `GenerationBitset`. The [contract](../pdes/optimistic-runtime-contract.md)
defines signatures and semantics; the [ADR](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/adr-production-runtime.md)
records the design decision. C ABI, Arrow and Python/R/Julia/TypeScript/C#/Go APIs
are unchanged.

## Compatibility classification and migration

| Surface | Current root | Change type | Breaking? | Maturity / notes |
|---|---|---|---|---|
| Rust API | `crates/kairo-ecs-pdes` | Additive driver and bitset; helper defect repair | No source break | Feature preview; new driver ordering is explicit |
| C ABI | `include/kairo_ecs.h` | None | No | Existing maturity unchanged |
| Arrow schema | `schemas/arrow/event_log_v1.schema.json` | None | No | Existing maturity unchanged |
| Host APIs | Existing binding roots | None | No | No adapter or package pin change |
| Shared fixtures | `conformance/fixtures` | None | No | No published fixture output/version change |

The new driver supplies its own full-key ordering; it does not alter core
scheduler ordering, `SimTime` representation or `RemoteEvent`. No root is
renamed, split, merged or removed. Existing consumers need no source edits.
Legacy rollback now preserves initialized components and rejects stale tokens
that previously revived; callers relying on those defects observe corrected
behavior. Legacy logical-generation diagnostics remain compatible, and the
helper still has scaffold input-anti/tick-order semantics. No migration guide is
required for this additive alpha preview; beta/stable promotion requires a fresh
review if consumer edits, deterministic shared outputs or protected roots change.

## Memory ownership

The runtime owns model instances, retained pre-event snapshots, pending inputs,
replay markers, send records and cancellation metadata. Models must provide
independently owned snapshots containing values, membership, RNG and reversible
output state. Shared interior aliases and irreversible external handler effects
are unsupported. Bitset snapshots contain logical bits only; instance identity
and validity epochs remain outside reversible state. Restore bitsets before
publishing staged values/RNG and obtain fresh handles. Counts bound retained
objects; they do not establish total model memory bytes or allocation immunity.

## Error model and thread safety

Typed preflight errors reject malformed external inputs without queue/authority
mutation. Snapshot, restore or handler failures/panics and invalid complete
post-handler output batches poison the runtime without partial batch publication.
Checked identity/epoch limits avoid reuse; rebuild a poisoned model from trusted
input. Legacy nonfallible stamp exhaustion still panics, an explicit limitation.
The local driver mutates through exclusive `&mut self`; it introduces no concurrent
worker, shared-model synchronization or simultaneous CPU-execution guarantee.
Model Send/Sync properties do not establish distributed safety.

## Determinism, replay and fixtures

Ordering uses tick, actual emitter and structural ancestry/ordinal, rather than
first-arrival allocation. Replay preserves logical identity and assigns fresh
delivery incarnations; exact old antis retain original payload/routing. Outputs
have strictly later ticks. Complete snapshots restore initial state and RNG;
surviving inputs replay after suffix rollback. Caller-proven GVT prunes strictly
before the floor and preserves equality. Distributed in-flight accounting remains
external.

Independent [runtime held-outs](../../crates/kairo-ecs-pdes/tests/optimistic_runtime_heldout.rs)
and [bitset/model held-outs](../../crates/kairo-ecs-pdes/tests/optimistic_bitset_heldout.rs)
cover noncommutative ordering with a core Scheduler reference, parents 10/20
emitting children at30, cascade/replacement races, actual-source collisions,
initial/RNG restoration, stale authority, repeated budgets, GVT equality and
poison/complete-batch failures. These supplement crate integration fixtures;
they do not export or change the shared conformance catalog.

At `3cd6d56`, the independent 15 held-outs and combined 94-test lane passed.
Cargo cache audit identifies actual Homebrew Rust1.99 despite nominal wrapper
labels; the prior Rust1.98/MSRV1.76 claims are withdrawn pending explicit reruns.
Coordinator `just ci` at `7a432ab` passes with explicitly resolved Rust1.98.1,
matching LLVM and fresh target: 458 tests, zero skipped, coverage92.59%,
fmt/Clippy/rustdoc/deny/audit. Source-bound receipt:
`artifacts/track48-final-validation/receipt-resolved-pinned.json`.
Actual Rust1.76 crate verification remains pending at this record.

The source-bound [benchmark evidence](../../benches/pdes/evidence/track48-ec9828e/)
preserves fixed inputs and five alternating raw repeats for sparse/dense4/8LP
cases, state/RNG/trace parity and real rollback/replay. It times only runtime run
calls; setup, extraction, validation and fossil collection are excluded. Its
Cargo-selected compiler metadata remains to be reconciled. No general scaling,
distributed rollback or simultaneous CPU-execution conclusion is accepted.

## Alternatives and red-team objections

- First-arrival numeric output allocation can reverse children of late parents;
  structural full-parent identity replaces it, tested with noncommutative digits.
- Clone alone can alias interior model state; independently owned snapshots are
  required, with initial values/RNG/bitset recovery fixtures.
- Input cancellation cannot retract downstream effects; send-log antis and
  surviving-input replay replace that scaffold behavior in the new driver.
- Old antis can cancel replacements or collide across emitters; source-scoped
  exact incarnations and retained old metadata have independent race tests.
- Rolling back epochs revives tokens; fresh nonrollback runtime/LP/bitset authority
  and cross-instance stale-handle fixtures address this objection.
- Local manifest/parity could be overstated as distributed proof; Track48 stays
  In Progress and the [Track49 handoff](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/distributed-interface-handoff.md)
  is a proposal. No dependency waiver or production dispatch follows.

## Release decision and reviewer record

Architecture and independent behavioral reviewers accepted the bounded local
contract, implementation and source-bound benchmark after the recorded fixes.
This assessment records those actual objections and scope; it does not fabricate
a human release-manager or protected-surface governance signoff. First allowed
stage is feature-gated alpha preview. Distributed/evidence-backed release claims
are held until Track49 integration supplies actual cross-process/rank rollback,
GVT and raw artifacts. Hosted exact-head CI, push/merge and release approval are
separate gates. Coordinator quality readback also reports unresolved main CodeQL/Scorecard security alerts, missing Codecov project status and pending Renovate refresh; no existing PR193-only exception applies to this delivery. Track48 remains In Progress; dependencies are unchanged.

The affected-crate release note and global Conductor synchronization are committed in d62891e/eb02a14. Actual matching1.98 workspace and1.76 PDES tests, plus compiler-bound68b8d7a benchmark evidence, resolve the local compiler-provenance gaps. The handoff/test matrix retain hosted/security/distributed gates and no human release signoff is inferred.


## Additive codec bridge assessment — 4 October 2026

Affected Rust root remains `crates/kairo-ecs-pdes`, outside the current protected-surface inventory; alpha preview under `time-warp`. The [bridge ADR](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/adr-codec-bridge.md) freezes borrowed full-parent inspection and checked order/message reconstruction. Existing public signatures, private immutable ancestry, scheduler ordering, rollback, GVT, C ABI, Arrow and host bindings are unchanged. No dependency or shared published fixture catalog changes; actual crate integration conformance fixtures are added. No consumer edits or migration adapter are required. The preview addition is documented in the [runtime contract](../pdes/optimistic-runtime-contract.md#checked-codec-bridge--alpha-preview).

Architecture and independent hostile-input roles accepted the bridge contract, requiring full ancestry validation, native u128 preservation and explicit authority namespace/owned outbox limits. Independent source review accepts worker2dbdd1b; root integrated exact blobs at c19fbf4. Final hashes, actual compiler cache and logs prove98 tests/Clippy/format at1.98.1; root matching1.76.0 passes98. Canonical [evidence](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/codec-bridge-evidence/README.md) preserves the actual source/base/file bindings and failed attempts. Independent held-out acceptance and hosted/new-PR checks remain pending.

Compatibility level: additive compatible local alpha API. No human release-manager, protected-root enrollment, stable release or publication signoff is inferred. Track48 stays In Progress. The native cancellation namespace and lack of owned-process accounted outbox remain explicit blockers to Track49 production admission; these have separate prepared decision records and cannot be waived by codec roundtrip tests.


Codec bridge combined local acceptance: integrated b8ff195 passes all five independent held-outs on explicitly bound Rust1.98.1 and1.76.0; the independent reviewer verifies source, raw log and actual compiler-cache hashes. Matching1.98.1/LLVM22.1.8 `just ci` passes467 tests, zero skipped, core coverage512/553 (92.59%) and fmt/Clippy/rustdoc/deny/audit. Architecture role independently accepts source/code/docs provenance. Canonical codec-bridge-evidence records preserve these actual snapshots. Hosted/new-PR, authority/fencing, owned/outbox, durable admission and actual distributed gates remain pending; Track48 stays In Progress.
