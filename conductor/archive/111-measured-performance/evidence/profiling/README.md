# Native CPU, storage and checkpoint profiles

The captures use Cargo 1.99.1 with Homebrew rustc 1.100.0 on a shared
Apple M1 Max. The binaries identify the compiler through the linked Rust
standard library source revision.
The [receipt](receipt.json) identifies the source, binaries, commands and
output digests. The earlier
[allocator experiment](../../handoff.md#direct-event-handoff-experiment)
counts allocation and reallocation requests separately from wall time.
The [cancellation profile](../../handoff.md#scheduler-cancellation-compaction)
supports retaining the binary heap with compaction.

## Thread creation and synchronization

On the matched PHOLD input at seed 472026 and zero mixes per event,
`sample PID 3 1` captured the scoped and pooled processes for three seconds
at one-millisecond intervals. The scoped stack includes startup and join
under the conservative round and shows 535 `__bsdthread_create`, 155
`_pthread_create`, 813 `__ulock_wait` and 15 `_pthread_join` leaf samples.
The pooled stack shows 2,685 `__psynch_cvwait` leaf samples. These are aggregate leaf counts across sampled threads. The prior thread-ID oracle isolates reuse across
two horizons. [Scoped stacks](scoped-stack.txt.gz) · [Pooled stacks](pooled-stack.txt.gz).

## Storage and checkpoint boundaries

The layout comparison inserts the same generated `u64` components through
both public stores, then scans the complete column 64 times. It runs one warmup
per size and layout and ten fresh process pairs in alternating order. Each
process verifies the wrapping sum and the comparison checks paired equality.
The existing query oracle checks ordered rows. The table gives medians
and descriptive 95% intervals from 10,000 paired bootstrap draws (seed 111).
Column bytes include row
padding. RSS is the whole process peak recorded by macOS `/usr/bin/time -l`.
[Source](layout-and-checkpoint.rs) · [Runner](measure-layout.py) · [Samples](state.csv).

| Rows | Tight/aligned scan time, median paired ratio | Tight/aligned peak RSS, median paired ratio | Aligned and tight column bytes |
| ---: | ---: | ---: | ---: |
| 4,096 | 0.135 [0.133, 0.287] | 0.916 [0.916, 0.916] | 262,144 / 32,768 |
| 262,144 | 0.072 [0.065, 0.082] | 0.635 [0.630, 0.649] | 16,777,216 / 2,097,152 |

The small component benefits from the existing opt-in tight layout in this
scan. Keep the aligned default for other workloads. This comparison uses
wall time and RSS as its layout metrics.

For the entity allocator, the same churned slot set is used for an ordered
live snapshot, native JSON encoding, byte transfer via a local Unix socket
pair and native decoding. The transfer includes receiver synchronization.
Each fresh process verifies the transferred bytes, restored live handles and
the next allocated handle. Medians over ten runs follow one warm-up per size.
[Raw checkpoint times](checkpoint.csv).

| Starting slots | Bytes | Snapshot ms | Encode ms | Socket transfer ms | Decode ms |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4,096 | 11,286 | 0.057 | 0.028 | 0.077 | 0.084 |
| 262,144 | 793,138 | 4.711 | 1.513 | 1.054 | 5.306 |

The [Time Warp capture](time-warp-raw.json) compares committed work with the
conservative reference and records rollback/replay counters. Its timed scope
is the runtime call, which includes the in-process optimistic snapshot path.
The isolated snapshot table above describes world handles, and its transfer
latency describes local IPC.

## Event batch boundary

The archived experiment lets each LP drain eligible events below its safe
frontier in one selection. The existing same-LP follow-up oracle fails with
observed order `[1, 7, 3]` against expected `[1, 3, 7]`. The handler can emit
a local child before a previously selected event. Keep the existing one-event
selection per LP and the ordered, round-atomic outbound commit. A broader
batch needs a stronger execution contract. The frontier optimization leaves
these semantics alone.
[Rejected patch](drain-safe-frontier.patch) · [Failure](batch-counterexample.log).

## Reproduce the state comparison

From the repository root, extract the receipt's baseline to a new scratch
checkout. Copy `layout-and-checkpoint.rs` to
`crates/kairo-ecs-state/examples/track111_layout.rs` inside that checkout
and use the receipt's Cargo build with a private output directory. Run the
comparison on macOS with the qualified repository-local CPython 3.15.2.

```sh
# Select the verified Homebrew rustc 1.100.0 and rustdoc before running.
export RUSTC="$(command -v rustc)" RUSTDOC="$(command -v rustdoc)"
"$RUSTC" -Vv
repo="$PWD"
evidence="$repo/conductor/tracks/111-measured-performance/evidence/profiling"
scratch=$(mktemp -d)
git archive --format=tar --output "$scratch/source.tar" "$(jq -er .baseline "$evidence/receipt.json")"
tar -xf "$scratch/source.tar" -C "$scratch"
mkdir -p "$scratch/crates/kairo-ecs-state/examples"
cp "$evidence/layout-and-checkpoint.rs" "$scratch/crates/kairo-ecs-state/examples/track111_layout.rs"
(
  cd "$scratch"
  CARGO_TARGET_DIR="$scratch/target" rustup run 1.99.1 cargo build --locked --offline \
    --release -p kairo-ecs-state --features checkpoint-json --example track111_layout
)
"$repo/.venv/bin/python" -I "$evidence/measure-layout.py" \
  "$scratch/target/release/examples/track111_layout" "$scratch"
```
