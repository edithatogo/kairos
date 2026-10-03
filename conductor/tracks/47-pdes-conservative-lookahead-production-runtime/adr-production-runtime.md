# ADR 47: Event ownership and conservative single-host execution

Status: accepted for the preview Rust interface, subject to implementation review.

## Decision

Add `ConservativeRuntime<P>` behind `pdes` rather than silently changing the
Track 34 callback scheduler. The runtime owns timestamp queues and channel
bounds. `ConservativeProcess: Send` handles one event and returns event outputs;
scoped workers execute independent LP work, then a coordinator validates all
outputs before delivery. Positive lookahead and exclusive incoming bounds
ensure a recipient cannot execute an event before an earlier causal message.
All runtime output is deterministic under fixed initial insertion order,
partition, topology and deterministic model callbacks.

The caller-owned Track 34 `LogicalProcess` cannot reveal its next queued event
or the timestamp that produced an outbound event. Reinterpreting its batch-time
callback as production ownership would either admit causality errors or reject
valid output. Preserve that compatibility API and document explicit migration.

## Compatibility and language assessment

This is an additive experimental/preview Rust surface. No C ABI, Python, R,
Julia, TypeScript, C#, Go or Arrow schema changes occur. Bindings must not infer
production PDES support until their own contracts and conformance fixtures are
implemented. The API is batch friendly through initial scheduling and bounded
`run_until`; ordered event payloads remain application defined. Versioned
Rust conformance fixtures in `production_parity.rs` compare DES, ABM and mixed
final state against the actual sequential core scheduler.

Typed errors implement `std::error::Error` without adding a dependency. This
local exception to the preferred `thiserror` style retains the dependency-light
hot path; error enums and formatting are covered by protocol tests. Benchmarks
capture raw timing and do not make superiority claims; fair comparative HPC
certification remains Track 55 work.

## Red-team objections and responses

- A null bound is inclusive: rejected. It promises no event *below* the bound;
  processing at equality would race a legal remote event.
- A worker can emit an earlier event within a large batch: preserve event-time
  provenance and deterministic queue order; validate remote lookahead against
  each event that produced output, not the final LP horizon.
- A handler mutates state then returns invalid output: poison the runtime and
  reject the entire outbound batch before delivery. No recovery is claimed.
- A panic can leave workers or LPs partially advanced: join all workers, surface
  a typed error and poison subsequent execution.
- Local benchmark success proves HPC parity: rejected. Record actual host,
  thread counts, seeds, checksums and raw profiles; label capability and
  distributed/scaling certification boundaries explicitly.

## Consequences

Models own one LP state and require `Send`. Positive lookahead is mandatory for
remote routing. Zero-delay local chains are allowed but must terminate through
model logic. Deterministic ordering is destination-local; a globally ordered
model must encode causal dependencies explicitly. New distributed transports
need a separate proof of channel bounds, in-flight accounting and failures.
