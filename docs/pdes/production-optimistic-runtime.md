# Local event-owned optimistic runtime

`OptimisticRuntime<P>` is the `time-warp`-gated event-owned driver. It executes
deterministic LP rounds beyond conservative incoming safe-time waits, detects
stragglers by full ordering key, restores affected state and replays surviving
inputs. It routes anti-messages for recorded downstream outputs. The compatible
`TimeWarpRuntime` helper retains its narrower scaffold semantics.

The [reviewed contract](optimistic-runtime-contract.md) defines the complete
local interface. [Held-out runtime tests](../../crates/kairo-ecs-pdes/tests/optimistic_runtime_heldout.rs)
and [bitset/model tests](../../crates/kairo-ecs-pdes/tests/optimistic_bitset_heldout.rs)
provide executable model and routing examples. At integrated source `3cd6d56`,
all 15 held-out tests and the combined 94-test `pdes,time-warp` crate lane
passed under the actual Homebrew Rust 1.99 compiler. Their original pinned
compiler labels were withdrawn after compiler-path verification. At `7a432ab`,
the explicitly bound Rust 1.98.1 workspace lane passed all 458 tests, core line
coverage (92.59%), formatting, Clippy, rustdoc and Rust security checks.
Matching Cargo/rustc 1.76.0 also passed the complete 94-test local PDES lane
at `7a432ab`, verified independently from source hashes and compiler cache.
Corrected benchmark provenance and hosted checks remain pending. No distributed acceptance or
Track48 Done follows from these local results.

## Model contract

Implement `OptimisticProcess` with independently owned snapshots containing all
handler-visible values, component membership, deterministic RNG state and
reversible output state. Initial and pre-event snapshots protect initialization
and ordering-sensitive state. Restore a bitset's logical snapshot before
publishing staged values/RNG, and obtain fresh component handles. Snapshots must
not restore validity epochs. Shared interior mutation and irreversible external
handler effects are unsupported.

Supply matching partition/process LP sets and a declared directed topology.
`schedule_initial(stable_sequence, event)` assigns a root envelope from the
actual event source and caller's stable sequence. Keep `RemoteEvent` unchanged:
source LP, destination LP, tick and payload bytes. Initial scheduling closes when
execution begins. `receive` accepts cloned positive/anti envelopes, validating
routes, source identity, exact-delivery metadata and GVT before queue mutation.

Outputs must declare the executing LP as source and have strictly later ticks.
Logical child identity uses the complete parent ordering key and vector ordinal.
Replay retains logical identity and assigns a fresh nonrollback incarnation;
payload, destination or tick may change with recomputed model state. Exact old
antis retain original metadata and cancel only their original incarnation.
Logical IDs and incarnation cancellation are namespaced by the actual emitter.

## Progress and failures

`run_until_with_budget` counts handlers, including replay, and queued anti steps.
At budget exhaustion, inspect progress and resume at the same or later horizon.
The driver drains queued antis before replacement positives. A rollback may
traverse one bounded retained history suffix. Limit LPs, pending/history events,
tombstones, output batches and causal depth; ancestry cannot exceed 128.

Rejected external input leaves delivery queues and validity counters unchanged.
Snapshot/restore/handler panics, failed restoration and invalid post-handler
outputs poison the runtime and publish no staged output batch. Inspect the typed
error and rebuild from a trusted model/scenario rather than continuing a
poisoned instance. State tokens bind runtime identity, LP and checked nonrollback
epoch; successful mutation/restore invalidates prior tokens for that LP.

`fossil_collect` accepts a caller-proven monotonic GVT bounded by all local pending
positive/anti/replay work. It collects strictly before the floor; equality stays
reversible and earlier arrivals reject. Distributed in-flight accounting is an
external contract. Reports expose executed/replayed work, rollback depth,
canceled sends, retained checkpoints/history/queues and tombstones. GVT lag is
the maximum executed/fossil LP frontier minus GVT, clamped at zero. Generic
snapshot counts do not establish total memory bytes.

## When optimism can cost more

A lightweight handler may spend more time saving state, maintaining identities
and queues, and retracting/replaying work than it saves by advancing early.
Frequent late input or dense dependent traffic can amplify cancellation cascades.
Large owned snapshots increase copying and retained-history costs. Measure the
chosen model, routing and arrival pattern with the same committed workload and
correctness oracle before selecting a mode.

The Track48 sparse/dense benchmark separates committed work from replay and
invalidated attempts. Its final timing boundary and raw results must be read
from the accepted evidence record; setup, validation and fossil collection need
separate cost accounting. These local measurements cannot establish simultaneous
CPU execution, MPI/gRPC rollback, distributed GVT, or a general scaling benefit.
The [distributed handoff proposal](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/distributed-interface-handoff.md)
retains those acceptance requirements and grants no Track49 dispatch.

## Historical local smoke measurements: compiler metadata superseded

Source `ec9828e`, actual Homebrew Rust 1.99.0 (the recorded Rust 1.98.1 label was incorrect), fixed seed482027; one warmup and five alternating repeats. Median run-call durations:

| Traffic / LPs | Conservative ms | Optimistic ms | Committed events | Extra executions | Replay executions |
| --- | ---: | ---: | ---: | ---: | ---: |
| sparse / 4 | 28.313708 | 0.044041 | 36 | 10 | 10 |
| dense / 4 | 119.983125 | 0.595792 | 141 | 134 | 92 |
| sparse / 8 | 63.305542 | 0.079209 | 71 | 24 | 23 |
| dense / 8 | 107.950417 | 1.826667 | 292 | 431 | 240 |

[Raw results and source binding](../../benches/pdes/evidence/track48-ec9828e/time_warp_evidence.json) preserve every sample. The lightweight model and conservative thread/null-message work make these fixture-specific durations; they do not establish a general performance advantage. Setup, validation/extraction and fossil collection are excluded from both timing intervals. Dense fixtures require substantially more rollback/replay in this model.

These raw files are preserved unchanged for audit. A compiler-bound collector rerun is required before accepting pinned-toolchain benchmark evidence.
