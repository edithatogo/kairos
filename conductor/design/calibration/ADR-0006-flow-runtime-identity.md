# ADR-0006 — Instance-bound Flow admissions

Date: 2026-10-05. Status: architecture reviewed by coordinator and independent gpt-6-luna reviewer;
implementation, behavioral and hosted gates remain open.
Owners: Track03 Flow, Track21 calibration adapter; Track25 compatibility review.
Extends the C2 execution admission v1 contract without rewriting scheduling.

## Demonstrated problem

Independent review found that generational WorkId values are local to a World.
Separate FlowRuntime instances can allocate identical work IDs. An adapter must
not use another runtime's terminal work to approve a policy boundary while its
original work remains Pending, Active or Suspended. Addresses alone cannot bind
an adapter: moving a runtime changes its address.

## Decision

Add experimental `FlowRuntimeIdentity`, with a private `Arc<()>` created in
`FlowRuntime::with_configs`. `FlowRuntime::identity(&self)` returns an owned clone.
Identity is Clone, Debug, Eq and PartialEq, but not Copy, ordered, hashed or
serializable. Equality uses `Arc::ptr_eq`; Debug prints only the type name,
never a memory address. Retaining an identity prevents address reuse from making
a later runtime appear equal. Moving a runtime preserves identity.

The token is solely an in-process ownership guard. It never enters scheduler
ordering, events, RNG seeds, Arrow telemetry or checkpoint bytes. There is no
portable identity, cross-process comparison or persisted continuation promise.
The default engine remains Rust1.76-compatible; no external dependency is added.

The private FidelityAdapter binds this identity only after its first successful
admission. Before duplicate checks, work reads or boundary scans, a bound adapter
rejects another runtime with InvalidWork. Failed admission does not bind an empty
adapter. Applying a pending policy with no admitted work may use any runtime and
does not bind it. NoPendingPolicy retains precedence when no policy is pending.
`decision(WorkId)` is an adapter-local lookup; it cannot attest the origin of an
arbitrary WorkId supplied by the caller. Public fidelity exports remain gated.

## Alternatives and compatibility

A private accessor would force tests into an internal module and leave the
future cross-crate calibration adapter without a safe ownership check. A process
counter introduces global mutable sequencing. Borrowing a runtime for the
adapter's entire lifetime prevents mutable execution between checks. Using raw
addresses fails when a runtime moves. Choose an owned opaque identity with
explicit in-process limitations.

This is an additive experimental Rust API on Flow, with no changes to existing
method signatures, event schemas, C ABI, wasm interfaces or serialized records.
FlowRuntime has no stable C layout guarantee. Public API review must include this
ADR, the conformance tests below, rustdoc, compatibility and objection response.
A stable-release API baseline is still an independent release blocker.

## Conformance and objection response

Tests must prove distinct runtimes compare unequal despite colliding WorkIds;
cloned and moved runtime identities remain equal; Debug contains no address;
failed initial admission permits later successful admission in another runtime;
foreign admission/boundary fails without changing either Flow or policy; and
foreign terminal work cannot bypass bound Pending/Active/Suspended work.
Existing Flow/queue tests must retain identical outcomes. No new identity is
compared across runs or mistaken for deterministic simulation data.

Red-team objection: a forged WorkId could still select another work. The runtime
guard prevents cross-runtime mutation and boundary bypass. The local decision
lookup is documented as local rather than advertised as an identity check.
Dropping all identity handles then allocating another runtime cannot fool a live
adapter, because the adapter retains its own Arc. Restoring portable checkpoints
will require Track22 to define explicit rebinding, rather than serialize this token.

Independent review: d35_gate_review, 2026-10-05, accepted this narrow bridge
and required failed-first-admission, moved-identity and foreign-terminal-work
regressions before implementation acceptance. No runtime acceptance from review.
