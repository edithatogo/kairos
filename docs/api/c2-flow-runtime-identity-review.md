# API Review: Flow runtime ownership identity

## Problem

Runtime-local generational WorkIds can collide across Worlds. A calibration
adapter must reject another runtime before using its work to approve a policy
boundary. Moving a runtime must not invalidate its ownership token.

## Affected root and surface family

`crates/kairo-ecs-des`, experimental Rust Flow API. Existing published package
names and stable core/time/FFI roots are unchanged.

## Proposed API

Rust: opaque `FlowRuntimeIdentity`, private Arc payload, Clone/Debug/PartialEq/Eq;
`FlowRuntime::identity(&self) -> FlowRuntimeIdentity`. Equality is pointer identity
of retained Arc ownership; Debug is address-free. No public constructor, Default,
Hash/Ord, codec or serialization. Private fidelity types are not exported.
C ABI, Python, R, Julia, TypeScript, C# and Go: no change and no new token bridge.

## Compatibility matrix

| Surface | Current root | Change type | Breaking? | Status | Notes |
|---|---|---|---|---|---|
| Rust API | kairo-ecs-des | additive ownership token | No existing signature changes | experimental preview | Default engine Rust1.76; no new dependency |
| C ABI | kairo-ecs-ffi | none | No | existing | No layout/token export |
| Arrow schema | kairo-ecs-arrow | none | No | existing | Identity excluded from telemetry |
| Host-language API | bindings | none | No | existing | No cross-process identity claim |

## Migration notes

No existing consumer changes are required. Future in-process adapters retain an
identity clone and compare before using local WorkIds. They must not use it as
simulation data. Portable checkpoint restore needs Track22 rebinding, which is
not implemented by this token. A stable-release API baseline remains open.

## Memory ownership, errors and thread safety

Flow owns one Arc; each identity handle retains it. This prevents reallocation
from aliasing a live handle. Arc cloning has standard shared ownership; no mutable
global counter or unsafe code. Send/Sync follows Arc's standard auto traits; this
introduces no simultaneous mutable Flow execution. Identity construction/comparison
has no user error variant. Adapter mismatch fails InvalidWork before mutation.
Failed first admission does not bind; no-work policy replacement stays unbound.
Production FidelityAdapter does not implement Clone, preventing divergent policy
views from masquerading as one all-bound-work guard. Clone exists only for tests
that compare unchanged state. Decision lookup remains explicitly adapter-local.

## Determinism and replay impact

Identity affects ownership checks only. No identity field enters scheduler data,
events, ordering, seeds, Arrow records or checkpoint bytes. Separate runs cannot
compare these tokens. Existing queue and Flow outcomes must remain unchanged.

## Conformance fixtures added

`conformance/c21/manifest.json` indexes `flow-runtime-identity-v1.tsv`, consumed by
`fidelity_lineage_v1.rs` with actual Pending/Active/Suspended work and foreign
Completed work. Clone/move/Debug and failed-first-binding cases are also executed.
The bootstrap runner's fixed ready IDs are preserved; its portable/binding
fixtures cannot represent in-process Rust ownership. See the family README.

## Alternatives rejected and red-team objections

[ADR-0006](../../conductor/design/calibration/ADR-0006-flow-runtime-identity.md)
records rejected address keys, global counters, lifetime-long borrowing and a
private accessor that cannot serve cross-crate adapters. Independent review
identified the original cross-runtime boundary bypass and demanded this guard.
A foreign WorkId passed to decision(work) still cannot be attested by that frozen
signature; docs state the adapter-local boundary rather than claim protection.
Production adapter cloning is removed in response to divergent-binding objection.

## Decision

- [ ] accepted for stable release
- [ ] rejected
- [x] development API review accepted subject to exact-head native hosted gates

Architecture reviewed by root coordinator and independent d35_gate_review on
2026-10-05. This form is a review record, not a replacement for executed gates.
Full C2, portable continuation, ED MVP and clinical validity remain open.

Final independent d35_gate_review source review found no remaining concrete
findings after shared fixture consumption and production Clone removal. Local
DES suite 238 tests, clippy/fmt and Rust1.76 checks passed at the documented
interim integration; final task source and hosted acceptance remain separate.
The disposable guard-removal mutant exits101 with behavioral assertion failures
after its identical baseline exits0. Production source was not mutated.
