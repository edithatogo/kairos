# ADR-0017 — Flow checkpoint assembly and owner payloads

Date2026-10-09. Status: coordinator experimental design under continuing C2
implementation authority and internal owner-role review. ADR-0016 foundations
are integrated atfad0380. No full portable or stable API acceptance is declared.

## Complete target

Export/import actual Flow state, not reconstructed history: scheduler and world
allocator; every built-in component store; resource/request/actor/work ownership;
work descriptors, actor domains and context type manifest; commands/notifications;
pending release/despawn and budget/admission/lease/batch counters. Function
pointers, TypeId and FlowRuntimeIdentity never enter portable data. Model code
supplies approved registrations and named, versioned context/template codecs to
restore a fresh runtime. Unknown state/registrations, mismatched schema, invalid
references, aliases outside the codec ownership contract and exceeded aggregate
budgets reject before exposing a runnable restored runtime. Full C2 legal modes
and work/transit progress remain required; no fixed recipe substitution.

Runtime-only capture occurs after dispatch/callback code returns. The higher C2
coordinator must reconcile bridge observations and capture all owner payloads at
one coherent frontier. This runtime seam alone cannot attest external observation
or runner commit completion. All decoded data stages privately; callback/domain
registrations come from trusted current model code, never artifact-selected code.
Context codecs must preserve complete owned state, have no externally mutable
aliases, honor output/input budgets, return errors for malformed bytes and perform
explicit process-local identity rebind using the new runtime's identity.

## Parallel prerequisite contracts

### Component stores — Track01C

A doc-hidden `component_checkpoint` module provides
`ComponentStoreCheckpointV1<P> { version:u32, sparse_slots:usize, rows:Vec<(EntityId,P)> }`
and `ComponentCheckpointLimits { max_rows:usize, max_sparse_slots:usize }`.
`ComponentStore::checkpoint_state_with(limits, encode)` encodes borrowed values
without requiring T:Clone; `ComponentStore::from_checkpoint_state_with(image,
limits, decode)` reconstructs arbitrary registered values from owned payloads.
Errors separate version/count/index/duplicate/allocation failures from codec E.

Preserve exact dense row order and sparse length, reconstruct generation/positions
from validated unique row IDs. Validate all length/index/uniqueness constraints
before invoking decode or constructing destination storage. Bounded membership
scratch after caps is allowed; use linear validation and fallible reserves.
Do not require World membership at this generic store layer; Flow cross-validates
its component ownership separately. Codec failure exposes no partial store.
Also expose a doc-hidden native `ComponentRegistry::registered_types()` inventory
for fail-closed completeness checks. TypeIds are process-local comparison values,
not persistent identifiers. No type-erased value is silently dropped. No unsafe,
serde dependency or existing store mutation/iteration semantics change.

### Fidelity — Track03

Add doc-hidden native owned version1 policy/adapter DTOs: full current policy and
optional pending policy, canonical entity/subsystem/pair override records, all
admitted (EntityId,FidelityDecision) records, and whether runtime-bound. Preserve
exact frozen decisions and pending changes. Never persist runtime identity or
recompute old admitted decisions from the latest policy.

Capture validates source-runtime lineage and supported work references. Restore
takes a fresh Flow runtime plus an explicit caller-supplied WorkId mapping/list;
verify each recorded full generational ID resolves exactly to that runtime's work,
reject duplicate/missing mappings, then bind to its new identity. No raw WorkId
constructor API is introduced by this leaf. An empty unbound adapter remains
unbound. Current/pending policy schemas and decision policy versions must be1;
existing subsystem validation remains unchanged. Override records may reference
future actors; do not incorrectly require them all alive now. Caller limits bound
admission count, total override count and aggregate subsystem bytes before clone
or destination construction. Registered work can be pending/active/suspended/
terminal; do not call admit to reconstruct or discard its old decision. Existing
FidelityError/public policy behavior stay unchanged; a separate experimental
checkpoint error owns new failures. No renderer/FFI or byte codec is implied.

### Duration provider — Track21

Crate-private version1 DTO includes all provider strata in strict canonical key
order. Each distribution is explicit fixed ticks or its ordered weighted support;
recompute/verify cached weight total, preserving support order because it changes
seeded draws. Caller limits bound total strata, total support records and total
identifier bytes before hashing/constructor allocation/cloning. Full preflight
checks identifier rules, positivity, unique duration values and checked positive
weight totals; no partial provider returned. Fixed distributions consume zero
draws; weighted rejection-sampling behavior and existing provider errors remain
unchanged. New checkpoint errors remain private. Tests compare sample values and
actual next-draw/state/counters over multiple restored strata and purposes.
No provider configuration inferred from current sampled work only.

## Flow implementation ownership and validation

Root owns flow.rs, its descendant checkpoint module and the DES re-export. Use
native DTOs and registered codec bytes without adding engine serialization deps.
Registry completeness includes empty registered stores; named source registration
inventory must match the supplied restore registrations. Work context and restart
template codecs must both be present where state exists. Immutable registration
code may be reconstructed; mutable context/template values and every runtime
counter/queue/notification must be captured. No function pointer copied from the
image or process address treated as a stable key. Different callback/factory
registrations with the same display name are not implicitly compatible.

Validate source IDs/generations, component/request/resource/work/lease relations,
queue and capacity conservation, known stale-event semantics and scheduler-event
ownership. Preserve legitimate cancelled or stale events and historical references;
do not demand every queued reference is currently alive when source semantics
allow a stale dispatch. Stage restored contexts with fresh identity, validate all
cross-links, then expose the complete new runtime. Failures cannot mutate the
source or an existing destination. Outer envelope/version/compatibility and
fresh-process C2 coordinator tests remain required next; these are actual-state
payloads, not proof of general runner/C5 recovery.

## Evidence

Isolated disjoint writer packets with exact input hashes/leases, red/green tests,
Rust1.99only full toolchain binding and local strict checks. Include nonclone
component transport/codec failure, sparse holes and generations; frozen mixed
fidelity with pending changes and active/suspended busy-boundary behavior after
rebinding; multi-strata fixed/weighted sample and stream equivalence plus all
budget/schema/order/corruption negatives. Root independently reviews and integrates
before connecting payloads. Full Flow restore/fresh-process parity across all C2
mode/progress states remains the goal, not completion of these prerequisites.
