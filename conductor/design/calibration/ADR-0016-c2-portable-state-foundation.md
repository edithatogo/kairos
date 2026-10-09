# ADR-0016 — C2 full-state portable checkpoint foundation

Date: 2026-10-09. Status: coordinator decision for experimental component state
interfaces under the user's continuing C2 implementation authority and parent
ADR-0009 internal engineering-role review. This is not full checkpoint acceptance.

## Decision and owner disposition

Internal Track22, Track03 and Track21 reviews inspected exact source `bc98ae5`.
Select owner-defined full-state export/import, supported versioned typed context
codecs and explicit handler/identity rebinding. Reconstruction of one fixed recipe
or arbitrary configuration replay is not the general checkpoint representation.
All states permitted by the C2 policy/work/transit contract must eventually be
supported; unknown user extension types require registration, never silent loss.
Internal Track22 accepts this direction with follow-up for integration and its
runner ledger. Root accepts Track01 scheduler/state and Track21 seed-registry
foundation interfaces below. No external maintainer response is claimed.

Checkpoint capture eventually occurs after callbacks/dispatch/submission return
and bridge observations reconcile. Restore stages detached payloads, validates
cross-references and compatibility before exposing a runnable new runtime, and
rebinds process-local FlowRuntimeIdentity rather than serializing it. Scheduler,
world, built-in Flow components, resources/requests/leases/work, typed contexts,
callbacks/registrations, policy and pending changes, stream/current draw state,
provider/configuration and collision registry all belong to that complete image.
Runner candidate/replication completion and checkpoint/result commit ordering,
1/2/N workers and crash recovery remain Track22/C5 obligations, not waived here.

## First parallel implementation contracts

No dependencies, manifests, RNG/scheduler algorithms or existing public semantics
change. Expose engine-native owned DTOs without serde or byte parsing in the core.
Track22 later defines the bounded canonical envelope/codecs for these DTOs.
Doc-hidden Rust-public engine methods are experimental inter-crate seams with no
stable ABI/API or FFI promise. Calibration state remains crate-private.

### Scheduler — Track01B

Add doc-hidden `checkpoint` module, `SchedulerCheckpointV1`, event record,
`SchedulerCheckpointLimits { max_entries: usize }` and typed error. Methods
`Scheduler::checkpoint_state(limits)` and
`Scheduler::from_checkpoint_state(state, limits)` return Results.
Version1 contains logical now; next event index/generation and insertion sequence;
scheduled/dispatched/cancelled counters; every physical heap entry's
ScheduleRequest, EventId, insertion sequence and live/cancelled flag. Persisting
all heap entries preserves physical state without relying on a tombstone-pruning
normalization. Export sorts records strictly by insertion sequence for canonical
order and does not prune/mutate the source. No callbacks or process addresses.

Before clone/allocation, enforce entry count limit. Validate schema1, strict
sequence order and unique IDs/sequences, event index/sequence identity and
expected generation (index modulo2^32), all indexes/sequences below next values,
next index=next sequence=scheduled count and next generation=next index modulo
2^32. Validate scheduled=dispatched+cancelled+live count with checked arithmetic,
all live entries represented and physical tombstones no greater than total
cancelled count. Do not invent a monotonic scheduling-time restriction: the
current generic scheduler allows scheduling requests earlier than now. Rebuild
heap and pending membership only after validation. Preserve exact next generated
IDs and tie order; no fresh IDs or resequencing on import.

### Entity allocator — Track01C

Add doc-hidden `checkpoint` module, `WorldCheckpointV1`, slot record,
`WorldCheckpointLimits { max_slots: usize }` and typed error. Methods
`World::checkpoint_state(limits)` and
`World::from_checkpoint_state(state, limits)` return Results.
Version1 includes every slot's generation/alive state, free-index stack in its
original order and dense live-entity order. Reconstruct sparse positions from
these validated authoritative arrays. The existing sorted WorldSnapshot is
telemetry, not a restore image. Preserve free-stack order because it controls
future recycled entity IDs. No registry/component serialization is implied.

Bound all three vector lengths before allocation. Require schema1; unique live
IDs/indexes, exact alive-slot and generation match, unique in-range free indices
which address dead slots, and exhaustive disjoint live/free partition of slots.
Use checked conversions and reject contradictions without installing state.
No arbitrary constraints on generation values or stack/dense order. Import
allocates only after complete validation. Future spawn/despawn behavior, stale
handle rejection, dense order and reused generations must match the control.

### Calibration collision registry — Track21

Add crate-private owned version1 seed-map state DTOs and capture/import methods
in seed_map.rs. Include map version, root seed, study ID and every registered
(seed, full SeedIdentity) entry, canonically ascending by seed. Preserve every
purpose/logical identity and registry entry; never rebuild from currently owned
streams only. Enforce caller-supplied entry and identifier-byte limits before
cloning/allocation. Import verifies supported version, existing identifier rules,
strict ordering/no duplicates, each identity's map version/root/study coherence
and recomputed derived seed. Reject tampered or colliding identities and expose
no partly populated map. Original source remains untouched on rejection.
This leaf does not encode portable stream state or claim draw-state verification;
those owner payloads remain required. No seed derivation or RNG algorithm change.

## Required proof and progression

Each worker owns only its module/root declaration and focused tests; one writer
per path, isolated worktree and bounded hashed lease. Native tests use Rust1.99
only. Tests must transport independently reconstructed DTOs, compare continued
behavior/IDs against uninterrupted originals, exercise all declared invariant and
limit/schema negatives, and prove malformed import/source preservation. Scheduler
cases include ties, cancellation, pruning, post-dispatch frontier and near-wrap
generation; allocator cases include fragmented slots, generation recycling and
nontrivial free-stack/dense order. Seed-map cases retain multiple purposes and
registry entries across restore plus negative identity/hash/order/version/limits.
Run affected-package tests, strict Clippy, formatting and scope checks; record
actual red/green evidence. Root reviews/integrates independent packets before
subsequent Flow/context/adapter codecs. Existing sealed demo and its failures
remain unchanged historical evidence. None of these component leaves alone
satisfies portable C2, C-06, phase, clinical or release acceptance.
