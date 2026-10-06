# C2 in-memory continuation v1

## Purpose and boundary

This contract fills the C2 requirement to preserve resolved fidelity decisions,
pending policy, purpose-stream positions, and bridge work progress across an
in-process resume. It defines an opaque owning handoff of live Rust values. It
does not define a portable format, process-crash recovery, or reconstruction of
a new `FlowRuntime`; those remain Track22/Track04 gates.

The per-work continuation bundle owns the exact `FlowRuntime`, `FidelityAdapter`,
and one post-bind `BoundIntrinsicWork<T,C>` value. This v1 shape supports one
bound bridge work item in the bundled runtime; callers must not infer that it
captures other bridge values held outside the bundle. Capture consumes these
values; while the bundle exists, no caller can advance or mutate that runtime or
adapter through their engine APIs. Resume consumes the bundle and returns those
same values. `FlowRuntime` is moved, never cloned or serialized, so
scheduler/world/registry/requests/works/contexts, transit state, queued events,
and runtime identity remain exactly as held at capture. A distinct runtime
cannot be supplied to resume.

This is a crate-private Track21 continuation seam, not a DES snapshot API and not
a public or stable library promise. Track03's fidelity adapter is preserved by
ownership as a complete value; calibration must not inspect or reconstruct its
private policy, pending policy, admitted-decision map, or runtime binding.

## Snapshot boundary and retained state

Only a successfully bound `BoundIntrinsicWork` may enter the bundle. Earlier
preparation/creation stages hold a borrowed, non-cloneable admission permit and
are part of a synchronous transaction. Capture is allowed only between completed
bridge method calls, when no callback or submit operation is in flight and no
returned `FlowDispatch` awaits `observe_transit_dispatch`. Tests capture after
transit start and after a pause has been dispatched and observed. The
bundle retains the exact bound bridge value, including its frozen fidelity
decision, expected Service key, live Service stream and draw position, sampled
intrinsic duration, acquire intent, transit request, runtime identity, work and
carrier IDs, pending/owned/stale/consumed event IDs, controls, retryable dispatch,
and arrival request/time.

The current bridge owns the Service-purpose stream. Transit runtime state is
held by the bundled live `FlowRuntime`; the current `TransitContext` has no RNG
field. No Transit or Behavior stream is claimed unless a future bridge version
actually owns one. Macro and explicit-zero Micro continue to have no transit
event or Transit draw.

The bundle is one-shot by Rust ownership: it cannot be cloned or restored twice.
Resume has no partial-failure path because the exact runtime, adapter, and bridge
objects are returned together without reconstruction. Policy staging, admissions,
Flow progress, callbacks, event ownership, and random draws cannot change through
their engine APIs while the bundle holds the only owned values. Externally aliased
interior-mutable values stored inside custom Flow contexts are outside this
ownership guarantee; callers must not mutate such aliases while detached.

## Ownership and exclusions

- Track21 owns the crate-private bundle in `crates/kairo-ecs-calibration/src/flow_bridge.rs`.
- Track03's `FidelityAdapter` remains opaque and unchanged; the bundle carries it whole.
- Track22 reviews this boundary and retains ownership of portable codec and process restart.
- No serde, byte encoding, filesystem persistence, new public API, new dependency,
  RNG algorithm change, clone, replay, or cross-runtime rebinding is permitted.

## Required behavioral oracles

1. For the single bundled work item, capture and resume actual bound Macro,
   explicit-zero Micro, and nonzero Micro work at completed bridge-call
   boundaries. The returned runtime retains equal
   work progress, contexts, queued state and clock; adapter decisions and pending
   policy are unchanged.
2. A Service stream with deterministic draws before capture produces the same
   next draw, key and draw position after resume as uninterrupted execution.
3. A nonzero Micro route captured after transit start and after a committed pause
   preserves exact event ownership and resumes the accepted arrival callback once,
   without duplicate submission or elapsed-time accounting.
4. Bundle ownership prevents access to its runtime/adapter while detached; resume
   returns the original runtime identity, not a replacement runtime.
5. Macro and explicit-zero Micro retain their no-transit/no-Transit-draw behavior.

These are local same-process in-memory assertions only. They do not satisfy Q4,
portable checkpoint/restart, hosted exact-head checks, Track03 owner/phase review,
release readiness, or clinical acceptance.
