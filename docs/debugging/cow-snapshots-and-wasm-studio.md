# Snapshots, provenance and Wasm inspection

The [Rust debug crate](../../crates/kairo-ecs-debug/src/lib.rs) provides an
in-process scheduler debugger and offline trace components.

## What the components do

| Component | Current boundary |
| --- | --- |
| `ReplaySession` | Restores and executes the Rust scheduler and caller-owned model, including captured RNG state. Shares immutable model data and supports copy-on-write model pages. |
| `EventTrace::snapshot` / `reconstruct_at` | Reconstructs caller-supplied string-keyed state from snapshots and deltas. |
| `CowWorldSnapshot` / `CowSnapshotStore` | Shares `Arc`-backed trace pages, replaces changed pages and indexes snapshots by tick. |
| `CausalProvenanceGraph` | Traverses explicitly supplied parent/child event edges; reliable engine-generated causal attribution still needs instrumentation and validation. |

`memory_savings_ratio()` reports the legacy trace store's page-count ratio.
Native checkpoint measurements use live allocated bytes, peak RSS and
measured restore and fork times.

The [Rust Wasm inspector helpers](../../crates/kairo-ecs-wasm/src/lib.rs) include
constructed provenance and constant bookkeeping. They do not establish historical
state restoration. The [TypeScript synthetic inspector](../../bindings/typescript/README.md)
is retired and fails explicitly; loading a Rust scheduler does not qualify a debugger.

## Qualification path

The [native replay ADR](../adr/0020-native-debugger-replay.md) describes the
checkpoint boundary and retention guarantee. Browser integration and
production model qualification remain open under
[completion item S15](../../conductor/scientific-completion.md).
