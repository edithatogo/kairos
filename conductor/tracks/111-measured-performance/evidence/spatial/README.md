# Held-out spatial agents with delayed events

Four regions each contain 96 agents on a bounded 16×16 lattice. The first
move happens at the frozen wake tick. Later moves occur two to five ticks
apart by agent ID, for eight moves per agent. An agent queries the native
continuous R-tree for the nearest other agent within radius two, breaking
equal-distance ties by entity ID. It takes a step toward the nearest agent,
with a fixed fallback for an empty neighborhood or coincident coordinates.
A move finding a neighbor sends a signal to that neighbor's numbered
counterpart in the next region of the ring. The signal arrives two ticks
later. An odd number of received signals reverses a later move.

The model, seed and timing protocol were fixed before comparative execution.
The [initial tape](tape.tsv) was frozen at 2026-11-25 09:56:11 UTC with
CPython 3.15.2 using seed `11120261126`. Its SHA-256 is
`fe8822c5b56c09331e7a736b45f7fe78ff0fbbe44475e3664b869e7635978a9a`.
Each row includes region, initial integer x and y, and wake tick. The agent
ID is its zero-based row number within a region.

## Comparison

Both engines use the same scratch Rust model and native spatial index. The
core Scheduler represents the same four regions serially. Conservative PDES
assigns one logical process to each and runs the persistent worker pool.
The model is one coupled simulation. Its ring delivers simulated signals
across LPs on a single host. Ties within each region are ordered by tick,
source region and insertion.

The [capture runner](capture.py) requires byte-identical per-region traces
and terminal states across both engines and all pairs. The trace carries
event time, source, destination, kind, agent, selected neighbor and updated
agent state. It also replays the result using independent integer distance
search, the fixed movement schedule and an exact multiset of expected
delayed signals. The Rust model checks all final positions against the
native index. Both processors drain all scheduled work through the model
horizon.

## Reproduce

From the repository root, use a fresh private Cargo target directory
`target` and a fresh receipt path. Set up the repository-local CPython
3.15.2 environment. Run

```sh
.venv/bin/python -I scripts/python_runtime.py
source_dir=conductor/tracks/111-measured-performance/evidence/spatial
compiler=$(rustup which --toolchain 1.99.1 rustc)
cargo_bin=$(rustup which --toolchain 1.99.1 cargo)
RUSTC="$compiler" CARGO_TARGET_DIR="$target" "$cargo_bin" build --locked --offline --release \
  -p kairo-ecs-abm -p kairo-ecs-core -p kairo-ecs-pdes -p kairo-ecs-types \
  --features kairo-ecs-pdes/pdes-worker-pool
"$compiler" --edition 2021 -D warnings -C opt-level=3 \
  -L "dependency=$target/release/deps" \
  --extern "kairo_ecs_abm=$target/release/libkairo_ecs_abm.rlib" \
  --extern "kairo_ecs_core=$target/release/libkairo_ecs_core.rlib" \
  --extern "kairo_ecs_pdes=$target/release/libkairo_ecs_pdes.rlib" \
  --extern "kairo_ecs_types=$target/release/libkairo_ecs_types.rlib" \
  -o "$target/spatial111" "$source_dir/model.rs"
.venv/bin/python -I "$source_dir/capture.py" "$target/spatial111" "$receipt"
```

The runner checks the compiler revision embedded in the executable. One
warm-up pair and ten measured pairs use fresh processes and alternate
execution order. Setup includes allocating agent state, building the
index and scheduling the initial tape. Dispatch includes all model
work and trace collection. Parsing inputs, process startup and result
verification are outside the timed interval.

Totals, per-pair times and an engine ratio use the same fixed workload.
Intervals use 10,000 percentile-bootstrap resamples of the respective
sample medians. These timings describe a shared host and one workload.
They do not establish a performance threshold or executor selection rule.

## Result

Every pair matched exact ordered traces and final positions and signal
counts for 384 agents. The model handled 6,097 events, comprising 3,072
moves and 3,025 cross-region signals. Signal state changed the destination
of 1,388 moves. The integer replay verified every transition and rejected
a corrupted neighbor in a negative control. The Rust input reader
rejected an invalid region ID.

The shared M1 Max capture ran locked Cargo 1.99.1
(`5f94df4789f005f9a352888e8355ffc645b7ed0e`) with explicitly selected
rustc 1.99.1 (`58cf8f9c35b355199727627d07ae34a4e2846d67`). The
[receipt](receipt.json) retains the original timings, tool and content
hashes, and the ordered result SHA-256
`45c72734ebfe3a90adee95fd510d53374066a9fdf8b40ccc592a9fd752fda95f`.

| Executor | Median total (ms) | Descriptive 95% bootstrap interval (ms) |
| --- | ---: | ---: |
| Serial | 4.307 | 4.186–4.353 |
| Conservative worker pool | 34.912 | 32.284–37.053 |

The median paired pooled-to-serial elapsed-time ratio was 8.42, with
a descriptive 95% bootstrap interval of 7.50–8.60. This synthetic
single-host case has short moves, small regions and frequent signals.
