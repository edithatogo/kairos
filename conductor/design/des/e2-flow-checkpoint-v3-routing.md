# E2 Flow checkpoint V3 routing owner contract

Status: Compatibility amendment for root and GPT-6.1-sol review; original reviewed document commit `755a0ddcb0d9fa6a8cd0bd9435fc7ebbefefce05`. Preparation only. Native V3 and explicit wire are accepted. Public wire qualification commit `81f889f66969c27048a8d23a58b47c5a75132807` joins core `3eae7b741d9957d651883bda646aead18961c220`; routing implementation still requires root acceptance of the complete wire evidence, qualification of the selected clean joined base, separately bound authority and ownership handoffs.

## Scope and authority

This contract defines lossless version routing after the explicit native and wire V3 contracts. It does not accept routing implementation, consumer behavior, ED bindings, E2 completion or release. Rust/Cargo 1.99 only. Existing native validation remains the single state authority; no second restore engine or V3 projection through V2 is permitted.

The future atomic implementation owns exactly these eight Rust paths:

1. `crates/kairo-ecs-des/src/flow/checkpoint.rs`
2. `crates/kairo-ecs-des/src/flow/checkpoint/v2.rs`
3. `crates/kairo-ecs-des/src/flow/checkpoint/wire.rs`
4. `crates/kairo-ecs-calibration/src/flow_bridge/checkpoint_wire.rs`
5. `crates/kairo-ecs-calibration/src/c2_portable_checkpoint/process_tests.rs`
6. New `crates/kairo-ecs-des/tests/flow_checkpoint_routing_v3.rs`
7. `crates/kairo-ecs-des/tests/flow_checkpoint_v3.rs` (only obsolete common-routing assertions in the existing legacy-capture proof)
8. `crates/kairo-ecs-des/tests/flow_deferred_allocation_v1.rs` (only obsolete common-routing assertions in the existing legacy-capture proof)

Together with result/logs this is an explicit ten-path bounded exception. Changing the common iterator item requires the two calibration pattern migrations in the same compilation unit. Preserve direct V1/V2 DTO command matchers. No mutation of `v3.rs`, runtime allocation, schemas, legacy layouts or unrelated consumers is authorized by this scope.

### Cross-track ownership prerequisite

Track03's agent contract requires dependencies on other track paths to be noted in `handoff.md`. Existing Track03/22 handoffs describe C2 ownership but do not grant this new change. Before source dispatch, the coordinator must separately claim and record the dependency in both:

- `conductor/tracks/03-flow-des-trajectory-abm-behavior/handoff.md`
- `conductor/tracks/22-experiment-runner-scenario-management/handoff.md`

The Track03 note must name the two calibration paths above, common command item migration, accepted native/explicit-wire source identities, eight-path atomic exception, preserving existing coherent checks and legacy consumers, and the separately bound implementation/qualification. Track22 records the consumed complete view, compilation compatibility and later behavioral consumer work. Hash-bind both updated notes in the source packet. These documents are outside the eight Rust writes and require coordinator ownership. A passing compile is not an ownership handoff.

## Public routing seam

Add `V3(FlowCheckpointV3)` to `FlowCheckpointImage` and `V3(&'a FlowCheckpointV3)` to `FlowCheckpointRef<'a>`. Preserve existing variants, method names, parameter types and lifetimes. Add V3 arms to `as_ref`, `restore_with_rebind`, scheduler, registration and context accessors using the complete native DTO. Restoration calls the accepted native V3 validator; it must not strip modes, Flush commands or scheduler entries to call V2.

Change the common `commands(self)` iterator item to `(EventId, FlowCommandV3)`, borrowing its source slices and producing values without allocation. V1 and V2 entries use their accepted lossless command upgrades; V3 entries are copied. Retain original event IDs, source order, every AllocationFlush, GrantEnabled and live stale Completion. No filtering, materialized vector, clone of owned DTO, RNG use or runtime mutation. A fresh iterator call starts at the first row. Do not retain a V2 iterator by dropping Flush.

Migrate only the common iterator matches: three in coherent checkpoint validation (Domain, Submit and DomainControl) and one GrantEnabled match in portable process tests. Direct V1/V2 DTO matchers remain version specific. This is compilation compatibility, not proof that those consumers handle Deferred workloads.

## Capture choice and legacy preservation

Compute whether runtime state requires V3 before a single capture, without invoking codecs or constructing a checkpoint DTO. Inspect modes and pending Flush state/commands sufficiently to detect malformed state; do not silently treat a bad mode/cache as Eager.

| Valid runtime state | Selected image |
| --- | --- |
| All resources Eager, flags true, no pending GrantEnabled or Flush | V1 |
| All Eager, any false flag or any pending GrantEnabled, including enable=true | V2 |
| Any live Deferred resource, including empty Deferred with no Flush | V3 |
| Valid pending AllocationFlush | V3 |
| Invalid mode, cache, Flush target/join or applicable bound | Error before codecs; no fallback |

Selection must preserve legacy capture limits and error precedence. Do not call `capture_checkpoint_parts(..., false)` unconditionally just because the internal Parts is now V3. The representable Eager path uses the legacy-format bounds/precedence; the V3 path uses strict V3 inventory and manifest accounting. Both produce complete private Parts. Validate mode exact coverage, generations, pending indexes and scheduler joins before codecs or DTO emission. Old DTO emission remains private and fallible. Malformed Eager-target Flush must fail validation, never disappear through an old image.

Use one scheduler snapshot and one validated cache derivation. Capture once with the chosen validation policy and emit the selected complete DTO. No V1 attempt followed by V2/V3 retry, duplicated codec calls or error-based downgrade. Positive codec controls prove one invocation per captured context/restart payload. Existing V1/V2 capture methods and explicit native/wire APIs remain unchanged.

The V3 native stricter registration manifest aggregate is intentional only for V3. Include a real Eager restart fixture at the old accepted registration limit to prove routed legacy capture still succeeds, with bytes/errors equal to explicit legacy capture. Determine the exact threshold from source and freeze literal assertions; do not invent a limit or weaken the fixture to empty manifests. The corresponding Deferred state may require a larger strict V3 aggregate bound and must reject at the documented smaller bound before codecs.

## Existing-test compatibility correction

The two existing legacy-capture tests contain three temporary assertions that
common `capture_checkpoint_image` rejects Deferred state. Once routing is added,
only those common-route expectations must become positive V3 controls. Preserve
both existing test names and every explicit V1 `capture_checkpoint` and V2
`capture_checkpoint_v2` rejection. In each fixture reset encoder/decoder counters,
assert both explicit failures and zero codec calls BEFORE routed capture, then
assert routed `Image::V3` with exact Deferred mode coverage. Empty Deferred capture
keeps encoder calls zero; the one-context payload fixtures require encoder exactly
one and decoder zero. The pending-Flush fixture retains the exact Flush event and
resource; restore with fresh counters proves one-context decoding and retained
Deferred/Flush state. Reset counters before later legacy/Eager controls. These
updates advance the intended common API; they must not dilute explicit legacy
rejection, true legacy goldens or native validation. No other changes in those two
test files are authorized.

The owned wire.rs private header/primitive proof also currently expects the common
router to reject V3 bytes. Replace only that obsolete common-router assertion
with exact positive V3 decode equality; keep explicit V1/V2 reader rejection of
V3 bytes and all malformed/primitive assertions unchanged.

The focused source packet requires the six public routing families plus private
`routing_v3_capture_cache_corruption_rejects_before_codecs` in checkpoint.rs.
Public DTO corruption cannot substitute for malformed live runtime-cache capture
checks. Use real positive codec controls for both private and public failures.

## Wire routing

`FlowCheckpointImage::encode_wire` dispatches to the exact variant's explicit encoder. `decode_wire` inspects only the known eight-byte V1/V2/V3 magic, then invokes the matching explicit decoder with unchanged limits. Short input remains Truncated; unknown magic remains InvalidTag; known magic errors propagate from that decoder. No trial decoding, fallback on malformed known-version bytes or speculative version upgrade. Preserve all V1/V2 bytes and explicit error behavior.

The V3 encoder and scanner remain the accepted explicit implementations. No new wire schema, tag, field ordering, mode representation, ownership boundary or native join validation is introduced here.

## Required acceptance matrix

The bound source packet must name and execute focused routing tests covering all of the following, with deterministic literal assertions and positive controls:

1. Capture truth table: plain Eager V1, closed Eager V2, pending Enable(true) V2, empty Deferred V3 and real pending Flush V3. Assert exact variants, complete fields and source nonmutation.
2. Legacy parity: routed V1/V2 native DTO and wire bytes match explicit APIs. Preserve existing full-envelope goldens. Exercise the real restart registration boundary described above and compare exact legacy failure precedence as well as success. Valid codec controls establish one capture, not retries.
3. Actual Deferred continuation: capture at a started current tick with pending Flush, route encode/decode, restore with rebind, recapture, and compare complete bounded suffix and literal useful/remaining work. Prove the Flush survived and affects actual allocation. Include halt state and current-tick legality through existing accepted native/wire fixtures rather than fabricating partial DTOs.
4. Complete iterator: all version arms retain event ID/order and exact command count; V3 exposes Flush and stale live Completion11. Repeat iteration and assert unchanged source. Allocation-free implementation is established by direct source review plus a bounded test mechanism where available, without adding a broad allocator framework.
5. Fail closed: missing/stale/extra allocation mode, mode-cache mismatch, bad Flush cache/scheduler/resource/Remove join and exceeded bounds fail before context/restart codecs. Include valid codec-positive controls, exact native error strings and full source-state nonmutation. Unknown magic and malformed known V3 bytes must not decode as another version.

Preserve native V3's nine named proofs, explicit wire core's seven proofs, public wire's five proofs, all old native/wire/runtime fixtures, 19 legacy envelope goldens and 22 portable fresh-process cases. A common-view type migration must not replace or dilute those oracles.

Development feedback is prospective and captured: compile the complete owned dependency set early, then focused routing runtime tests before final qualification. The bound packet resolves exact commands and separately specifies failure ceilings; no historical failure is reclassified. Final qualification uses three immutable Rust1.99 gates: tests of DES/ABM/core/calibration with Flow enabled, strict Clippy for the same packages/targets, and formatting of owned Rust paths. Record actual argv, cwd, head, before/after source and protected-input hashes, status and raw-log hashes. Stop at the declared ceiling, freeze truthful results and retain every attempt. No commit before independent root/Sol source and evidence review; clean committed-head qualification follows scoped commit.

## Dispatch and successors

Routing source dispatch requires accepted native V3 plus complete explicit-wire acceptance including public fresh-checkout qualification, a clean joined qualified base, both ownership handoff notes, reviewed hash-bound packet and a narrow claim. Resolve actual APIs/commands/hashes at that dispatch. Core-only wire acceptance is insufficient. The routing writer alone owns all eight source paths; root reviews, integrates and accepts.

After routing acceptance and clean qualification, prepare three disjoint consumer packets on the same accepted base:

- Preview: `shadow_native.rs` and `c5_native.rs`. Prove real Deferred/pending-Flush preview, no advancement/mutation and budget halt. Preserve stale live completion observation before actual completion. Do not claim C5 configuration admits gate inputs.
- Coherent: inline tests in `flow_bridge.rs`. Prove actual V3 transit, arrival Pending Submit and frontier with independent negative controls for fields the coherent validator owns. Existing type compatibility alone is not V3 coherent behavior acceptance.
- Portable: `c2_portable_checkpoint.rs` and nested `process_tests.rs`. Prove real V3 interior drain and pending-Flush full-envelope fresh-process continuation, complete trace/state parity, raw versus clock-derived progress, and preserve all V1/V2 goldens/process proofs.

These packets can run in parallel only after the shared routing join is accepted, each with disjoint claims and bounded reviewed contexts. Root deterministically joins and requalifies consumers before updating the parent Kairos pin. Single-Flow parent ED integration, complete logical admission provenance, the 88 frozen E2 fixture bindings and E2.3 composition remain later required work. This document defines routing scope only; no routing implementation or whole-delivery acceptance.
