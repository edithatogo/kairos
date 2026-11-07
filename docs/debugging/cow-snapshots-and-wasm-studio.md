# Snapshots, provenance and Wasm inspection

The [Rust debug crate](../../crates/kairo-ecs-debug/src/lib.rs) provides trace and
snapshot components. Full engine rollback, branching and interactive historical
inspection remain experimental under [completion item S15](../../conductor/scientific-completion.md).

## What the components do

| Component | Current boundary |
| --- | --- |
| `EventTrace::snapshot` / `reconstruct_at` | Stores and reconstructs caller-supplied string-keyed trace state from snapshots and deltas; this is not a complete scheduler/world/RNG restore contract. |
| `CowWorldSnapshot` / `CowSnapshotStore` | Shares `Arc`-backed trace pages, replaces changed pages and indexes snapshots by tick. |
| `CausalProvenanceGraph` | Traverses explicitly supplied parent/child event edges; reliable engine-generated causal attribution still needs instrumentation and validation. |

`memory_savings_ratio()` reports a page-count bookkeeping ratio. It does not
measure allocation bytes, retained history or peak RSS. The former claim of
greater than 80% memory savings is withdrawn; no measured saving is established here.

The [Rust Wasm inspector helpers](../../crates/kairo-ecs-wasm/src/lib.rs) include
constructed provenance and constant bookkeeping. They do not establish historical
state restoration. The [TypeScript synthetic inspector](../../bindings/typescript/README.md)
is retired and fails explicitly; loading a Rust scheduler does not qualify a debugger.

## Qualification path

The [Track 110 plan](../../conductor/tracks/110-scientific-method-completion/plan.md)
requires real engine checkpoints, forward/back/forward replay equivalence and
branch isolation, including pending events, cancellations, world state and RNG.
Compare allocation bytes and peak RSS against full snapshots over the same
workload/history; include restore cost and bounded retention. Record measured
results under the [performance programme](../../conductor/performance-programme.md).
