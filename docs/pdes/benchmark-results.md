# PDES benchmark evidence

Maturity: preview local wall-clock evidence.

Track 47 now has a bounded wall-clock benchmark for the single-host
`ConservativeRuntime`. The executable compares the Rust core `Scheduler` with
the PDES runtime using the same event transitions and final-state check. Each
timing includes scheduler/runtime construction, initial event scheduling,
dispatch, and state extraction; PDES timing also includes scoped OS thread
creation and round synchronization.

The workload matrix has four LP counts (4, 8, 16, 32) and two profiles:

- **Strong scaling:** 2,048 initial events total at every LP count.
- **Weak scaling:** 128 initial events per LP.

Each initial event updates its source LP and emits one event to its successor
on a directed ring at the next tick. A fixed SplitMix64 seed generates event
payloads. Each case has one warm-up per implementation and alternating measured
order across repetitions. Raw elapsed nanoseconds, derived processed events per
second, final state parity, processed and emitted event totals, null messages,
scheduling rounds, final GVT, and observed worker count are retained per case.
The baseline is a single-threaded scheduler using the same process callback and
event payload; it is not a distributed or third-party simulator comparison.

Build and run the bounded executable directly:

```sh
rustup run 1.98.1 cargo bench -p kairo-ecs-pdes --bench production --features pdes -- --seed 472026 --repetitions 5
```

For immutable Track 46 evidence, run the collector from a clean tested commit
after pushing that commit. The remote tracking ref must resolve to the supplied
commit SHA:

```sh
python3 benches/pdes/collect_evidence.py \
  --commit-sha "$(git rev-parse HEAD)" \
  --pushed-ref refs/remotes/origin/BRANCH \
  --evidence-class live-hpc \
  --seed 472026 --repetitions 5
```

The collector refuses dirty source inputs, checks that the source tree remains
unchanged during collection, validates all eight cases and their timing,
throughput, event-count, worker, GVT, null-message, and parity fields, then
creates a new directory under `benches/pdes/evidence/`. It never overwrites a
prior bundle. The manifest records the checked-out commit and remotely verified
ref, CPU model and topology, memory size and topology, operating system, Rust
compiler/toolchain, exact command and selected environment variables, topology,
seed, feature flag, raw result path, and its SHA-256 checksum. Set
`--evidence-class scaffold --pushed-ref local-only` for a local collection that
is not tied to a remotely pushed ref.

The measured runtime uses real OS threads on one host. Its worker count records
the maximum number of LP workers active in a round. These runs do not establish
multi-node behavior, cluster-scheduler behavior, HPC parity, or certified
weak/strong scaling. Thread count can exceed physical core count at 32 LPs; the
collector records the host core topology so those timings remain interpretable.
No speedup or superiority claim is made from this benchmark family. Track 55
owns the cross-runtime and HPC scaling certification rollup.

Collector guard self-checks:

```sh
python3 -m unittest benches.pdes.test_collect_evidence -v
```
