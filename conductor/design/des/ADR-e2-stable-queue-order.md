# ADR: explicit stable secondary ordering for native Flow queues

Status: Independently reviewed design, pending coordinator freeze and bound packet; runtime acceptance is pending.

Authority: Root coordinates Track03 des-api-agent under the user’s full CareOps delivery instruction; independent Sol design review required. Source baseline: Kairos 5c57dc807cf6a6524d174adbb88502a92cbf5673, Rust 1.99.0 only. Public-preview final qualification remains in progress.

Problem: An ED request ready at tick1 but delayed by room eligibility until tick5 must outrank a same-urgency request ready2 admitted2, including when the latter is suspended. Native admission order cannot express this contract. ED fields stay in parent policy; the engine receives only an opaque immutable secondary key.

## Reviewed ordering decisions

Scope: generic native queue key preserving E2 logical ordering. This design requires an exact reviewed packet before execution.

The engine's existing smaller i32 priority wins; preserve this full-width strict comparator for admission and preemption. Parent maps descending full i32 urgency with bitwise complement, a bijection without arithmetic overflow. Secondary key never permits equal-priority preemption.

Proposed opaque bounded key: FlowQueueOrderKey::new(bytes: &[u8]) -> Result<Self, FlowQueueOrderKeyError>, private boxed slice, 1..=128 bytes, lexicographic unsigned byte order, read-only as_bytes. Generic bytes have no malformed ED-specific encoding; reject only empty/oversize. Parent E2 uses fixed-width big-endian ready tick followed by ASCII request ID (1..64 bytes). Engine never parses either. Public clone/equality/order traits are deliberate. No mutable rekey API.

Mode per resource: FlowQueueOrdering::{AdmissionOrder, StableKey}; existing constructors keep AdmissionOrder. Explicit new constructor accepts total, allocation mode, queue ordering. Builder optional queue_order_key must be Some for StableKey, None for AdmissionOrder. Validate before any ID, request, work link, scheduler counter or event mutation. Legacy callback Acquire remains valid on AdmissionOrder and rejects atomically on StableKey; no new callback command needed for current no-service-callback parent adapter.

Recommend duplicate key support, not rejection: comparator (priority, supplied key, native admission sequence, request ID). Exact duplicates retain stable admission order, matching conventional stable sort and avoiding unnecessary global key lifecycle state. Parent request IDs ensure its logical keys are unique among live segments. Terminal handover segment and replacement may retain same logical key safely; no key reuse prohibition is needed. Contract must explicitly distinguish reproducibility for a fixed input sequence from insertion-permutation invariance, which applies only to distinct complete priority/key values.

Store immutable keys for each keyed request through terminal audit state; resource mode persists even empty. Existing public ResourceRequest, PriorityKey and FlowAcquireCommand struct literals must compile unchanged. Internal composite key/sidecar plus explicit request_queue_order_key accessor preferable. All live queue, preempting and removal paths use the same comparator; deadline chronological priority stays semantically unchanged. Existing victim tie policy unchanged.

Native V1/V2/V3 capture rejects any StableKey resource before codec execution, even if empty or all keyed requests terminal. Existing admission-only capture still selects existing versions and preserves literal bytes. V4 schema must include exact resource mode coverage and request-key membership, validate before codecs/allocation, and reconstruct/verify every derived queue/index. Need reviewed V4 wire bounds before dispatch; do not change V3 in place. Core scheduler unchanged.

Required actual proofs: delayed ready1 admitted5 outranks ready2 admitted2; suspended ready2 vs delayed ready1 chooses ready1 after urgent holder completion; equal ready ASCII order; permutation invariance for distinct keys; exact duplicate key admission fallback; strict i32 min/max priority and equal-priority no preemption; nested Suspend/resume with one-unit capacity; nonpreemptible holder unaffected; key requirement/mode/empty/oversize rejection atomicity; immutable key across states; reprioritize changes primary only; legacy admission behavior and V1-V3 bytes unchanged.

Later Flow V4 persistence proofs (excluded from native runtime packet): interior continuation includes pending, queued, active, suspended and terminal joins, stale completions and deferred flush.

Open implementation review: internal queue type should not silently replace public registry component layout assumed by old checkpoint code. Owner inventory will identify every cache/validator site. May split native runtime + native V4 + wire V4 + consumers with explicit capability barriers, but no parent native binding until combined accepted.

## Independent review resolution

Sol design GO with retained-key clarification: V1-V3 capture rejects either any surviving StableKey resource OR any retained immutable keyed-request audit record, including after its resource was removed. Check this before caller codecs and output allocation. A mode accessor returns mode for valid live resource; a request-key accessor follows existing RequestId validity and retains terminal audit keys consistently with request lifetime.

ResourceSnapshot.queued projects effective keyed order for StableKey and unchanged admission order for legacy mode. Deadline same-tick expiration order and holder-victim order remain legacy ordering. Callback Acquire missing a required key is rejected during batch preflight before IDs, counters, work links or any sibling command admission; prove whole-batch rollback with a valid earlier sibling.

A private derived sidecar is selected to preserve public PriorityKey/ClaimQueue DTOs. Maintain main effective-order and preempting effective-order indexes transactionally, with checked membership/length parity against legacy queues and authoritative immutable keys. Do not scan/sort the full queue at every grant; preserve logarithmic updates. Rebuild/validate only at capture/restore and explicit invariant checks. Native runtime can be a separate accepted experimental boundary with failclosed persistence guards; V4 capture remains unsupported until separately implemented and qualified.

## Exact native API proposal

- `pub struct FlowQueueOrderKey(Box<[u8]>);` private field; Clone, Debug, Eq, PartialEq, Ord, PartialOrd. `pub const MAX_FLOW_QUEUE_ORDER_KEY_BYTES: usize = 128`. `new(bytes: &[u8]) -> Result<Self, FlowQueueOrderKeyError>` copies only after length validation. `as_bytes(&self) -> &[u8]` immutable borrow. Error enum `Empty`, `TooLong { limit: usize, actual: usize }` with Display/Error. This separate error type does not alter FlowError exhaustive matches.
- `pub enum FlowQueueOrdering { AdmissionOrder, StableKey }` Clone/Copy/Debug/Eq/PartialEq; no public setter. Existing constructors continue to AdmissionOrder.
- `FlowRuntime::create_resource_with_queue_ordering(&mut self, total: u32, allocation_mode: FlowAllocationMode, ordering: FlowQueueOrdering) -> Result<ResourceId, FlowError>`.
- `FlowRuntime::resource_queue_ordering(&self, resource: ResourceId) -> Result<FlowQueueOrdering, FlowError>` follows existing invalid-resource validation.
- `AcquireBuilder::queue_order_key(self, key: FlowQueueOrderKey) -> Self` moves key into declarative builder; submit validates resource-mode/key relationship atomically.
- `FlowRuntime::request_queue_order_key(&self, request: RequestId) -> Result<Option<&FlowQueueOrderKey>, FlowError>` validates existing request identity and returns immutable borrowed authoritative key. Legacy returns None. No public mutation.
- Missing or forbidden key at submit uses existing `FlowError::InvalidState`, documented as mode/key mismatch; malformed key length fails in constructor before runtime access. Existing `submit`, `submit_work`, callback Acquire remain unchanged signatures and yield InvalidState on StableKey. No public enum or struct literal addition to legacy request/command DTOs.

Native packet source candidates: flow.rs, new flow/queue_order.rs, lib.rs re-exports, a bounded actual-runtime integration test, checkpoint.rs for precodec guard entrypoints and legacy restore initialization, owner behavior note. Size exception requires actual guard-path inventory before binding. Native V4 is explicitly absent, not stubbed as accepted; any current version-aware capture reaching StableKey/retained-key state must fail closed. The exact guard error and entrypoints are resolved below.

## Governance boundary

Protected Rust root crates/kairo-ecs-des, surface family rust_api, experimental alpha. Preserve existing public PriorityKey, ResourceRequest, FlowAcquireCommand and FlowError shapes; new types and methods are additive. Existing resource defaults and legacy checkpoint bytes remain unchanged. Opt-in keyed state intentionally fails current persistence until explicit V4 implementation. C ABI include/kairo_ecs.h, Arrow schemas/arrow/event_log_v1.schema.json and host bindings remain unchanged. No root rename or promotion. Exclusive builder/runtime borrowing and immutable copied configuration preserve current concurrency model; no new sharing promise.

Ownership: Track03 DES source/API and native checkpoint guards; future V4 wire/consumer work requires reviewed dependency handoffs. User documentation must state key ordering, duplicate fallback, strict-priority preemption, mode mismatch errors, lifetime persistence guard and alpha status. No beta/stable/release approval, clinical validation, parent pin or E2 acceptance follows from design or native-stage implementation.

Red-team controls: no lossy urgency packing, no equal-priority preemption, no ready-time reset, no dummy resource, no full-queue sort at every allocation, no transient unpersisted keys silently captured in old layouts. Atomic callback rejection and sidecar corruption cases must have actual positive controls.

## Capture guard resolution

Use existing FlowCheckpointError::InvalidState("V1/V2/V3 cannot represent stable queue ordering"). Guard any live StableKey mode OR retained request key, before caller codecs and allocation. Check capture_checkpoint(V1) before ensure_legacy_checkpoint_supported, capture_checkpoint_parts before legacy branching (coversV2/V3), and capture_checkpoint_image before version selection. All entrypoints live in checkpoint.rs; no v2.rs change is needed just for guard. Legacy restore initializes AdmissionOrder and derived equivalent ordering once. Actual proof includes removed resource with retained terminal keys.

Native-stage proposed five source paths are flow.rs, flow/queue_order.rs, lib.rs, flow/checkpoint.rs and tests/flow_queue_order_v1.rs. Coordinator-owned ADR/handoff freeze precedes implementation. Additional existing module changes require a reviewed scope change; no worker expansion. Native tests/strictClippy/ownedformat are proposed source gates; exactcommands bind after docscommit and final preview acceptance. Full combined clean-head qualification remains separate.
