# Held-out FIFO queueing comparison

The evidence covers two independent, capacity-one FIFO stations with impatient
customers. Customers arrive on the frozen tape, take service immediately when
the server is idle, and otherwise wait in order. A queued customer leaves at
the patience deadline unless already admitted. Service completion and its FIFO
grant take precedence over patience expiry, then arrivals. Within a station,
same-tick arrivals keep tape order. The run drains all arrivals and timers.

The tape, model and ten-repeat comparison protocol were selected before the
first run. The retained [tape](tape.tsv) was frozen at 2026-11-25 09:17:23 UTC
with CPython 3.15.2, seed `11120261125` and SHA-256
`d7071031457ee70f43b434730a3075992da1e32817bbdc4dd80bb23a14d15d04`.
Each row holds station, arrival tick, service ticks and patience ticks. The
runner regenerates the tape byte for byte from the seed before comparing.
Customer IDs are zero-based row numbers within each station. This tape was
separate from the ring and PHOLD workloads used to develop worker reuse.

## Oracle

Kairos runs the core `Scheduler` and the conservative persistent worker pool
with one LP per station. Independent SimPy 4.1.2 uses its real environment and
capacity-one `Resource` queues, releases and request cancellation. The
conservative runtime and SimPy encode three event phases per model tick
to reproduce the schedule priority. There is no interstation traffic.

The comparison checks the exact trace in each station. Each row carries the
model tick, transition, customer ID, active customer ID and the full FIFO
waitlist. It also checks each customer's terminal state, service start and
exit tick, whether service cancelled a patience timer, and the final station
state. The tape must exercise arrival ties, abandonment and same-tick service
admission before an expiring patience timer. Core Scheduler cancels pending
timer IDs and validates removal. Conservative PDES and SimPy process obsolete
timer callbacks as no-ops. SimPy removes an abandoning customer's queued native
request. Backend-specific counters verify these distinctions.

## Reproduce

Use a repository-local, isolated Python 3.15.2 environment. Obtain the
[SimPy wheel](https://pypi.org/project/simpy/4.1.2/#files) and verify SHA-256
`43071f84b6512c9b4fcb33ef057f240ccb1d1f3b263f9b4f9229d072e310b372`.
Install this wheel for the reproduction environment only with
`uv pip install --python .venv/bin/python --offline --no-deps "$wheel"`.
The receipt runner also checks its installation metadata against that hash.

From the repository root, set `target` to a private absolute Cargo output
directory, `wheel` to the exact wheel path and `receipt` to a fresh output
file. Run

```sh
.venv/bin/python -I scripts/python_runtime.py
compiler=$(rustup which --toolchain 1.99.1 rustc)
cargo_bin=$(rustup which --toolchain 1.99.1 cargo)
RUSTC="$compiler" CARGO_TARGET_DIR="$target" "$cargo_bin" build --locked --offline \
  --release -p kairo-ecs-pdes --features pdes-worker-pool --example track111_queueing
.venv/bin/python -I conductor/tracks/111-measured-performance/evidence/queueing/compare.py \
  "$target/release/examples/track111_queueing" "$wheel" "$receipt" \
  --compiler "$compiler" --cargo "$cargo_bin"
```

The commands select the repository's pinned Rust 1.99.1. To reproduce the
historical capture profile, pass the exact Homebrew Rust 1.100.0 compiler
(`0ef1bc386059596dada8fb65b50425aab2ea637a`) and keep Cargo 1.99.1.
The runner verifies the compiler revision embedded in the executable.

The runner launches a fresh process for each engine and validates trace and
state equality before accepting every sample. One warm-up triplet and ten
matched triplets are retained. The timed interval covers initialization of
the model and scheduling of arrivals, then drain and trace collection.
The work includes runtime-specific event and resource machinery. Input
parsing, process startup, Python import, output encoding and cross-engine
validation are outside the recorded interval. Run order rotates across the
engines. The receipt holds nanoseconds for setup, dispatch and their sum
and reports medians and descriptive percentile-bootstrap intervals for the
medians using 10,000 resamples. The interval reflects one shared host and
fixed tape. It cannot establish an executor choice or a regression threshold.

## Result

The three engines agreed in every triplet on 3,435 ordered transitions across
1,024 arrivals. The tape produced 370 service completions, 654 abandonments,
363 cancellations of patience timers, and 97 FIFO grants exactly at the
waiting customer's deadline. The core scheduler dispatched 2,048 events
and removed all 363 cancelled timer IDs. Conservative PDES processed
2,411 events, including the 363 obsolete deadlines, and used up to two
eligible LPs in a round. SimPy removed 654 queued native requests.

The shared Apple M1 Max ran Cargo 1.99.1 with Homebrew rustc 1.100.0 for
both Kairos executors, and the isolated CPython 3.15.2 installation with
SimPy 4.1.2. Serial Kairos had the lowest median total time in this
workload.

The original receipt queried rustup's compiler while Cargo resolved
rustc from the process path. The original executable's embedded revision
matched Homebrew rustc 1.100.0. The corrected receipt preserves the
original measurements and hash of the capture runner.

| Engine | Median total (ms) | Descriptive 95% bootstrap interval (ms) |
| --- | ---: | ---: |
| Kairos Scheduler | 0.5895 | 0.5754–0.6046 |
| Kairos conservative worker pool | 12.430 | 9.578–16.525 |
| SimPy Resource | 7.319 | 7.217–7.481 |

The [raw receipt](receipt.json) records the host, toolchains, source and
artifact hashes, event counts, and individual timings. The capture used
base `9d76ca7f4b7f4b5898f2ec77515a7a7639f1478b` plus the hashed example
and comparator, and exited successfully. The exact result digest is
`e6df5ef8bfc08969042d36d0de9cc74f5f42e791dfa542eaabca048a998d4dc9`.
A mutated trace failed the comparator and a malformed station ID failed
the Rust example before dispatch.
