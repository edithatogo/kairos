# Dedicated conservative PDES qualification

The physical Linux x86-64 runner used an Intel Core i3-N300. A cgroup v2
partition reserved four non-SMT CPUs for the benchmark processes. Linux
reported the partition as `root` with effective and exclusive CPUs 4–7.
The benchmark process affinity was 4–7. The host's system slice reported
0–7 before, 0–3 during and 0–7 after each capture. The samples include
affinity, partition, host, CPU account and release records. Both captures
exited 0. The CPU reservation covers the benchmark processes on that host.
The runs share host memory and other machine resources.

## Matched results

Each tape has 12 fresh process triplets at each of two event costs, with
all six arm orders twice. Each executable runs serial and pooled PDES on
the same seeded inputs, with one warm-up and two timed repetitions per
profile. Five profiles vary LP count, lookahead and remote-event share.
Every triplet matches all ordered per-LP state, tape hashes and runtime
counters before the timing analysis. The timed end-to-end samples include
engine setup, scheduling, dispatch and state collection. Setup and
dispatch times are also retained. GNU time reports peak RSS in KiB for
each whole process.

The primary comparison uses the 32-LP, eight-tick, 75%-remote, zero-mix
profile. The frozen-baseline rule requires at least 20% lower median
elapsed time, an upper 95% ratio below 1 and at most 5% higher median time
in any other profile. The incremental rule requires an upper 95% ratio
below 1 against the previous runtime and the same other-profile limit.
The interval resamples the 12 paired process ratios with 10,000 draws.
The runtimes include the published frontier fold and the batch candidate.

| Tape | Comparator | Candidate median ratio and 95% interval | Highest other-profile ratio |
| --- | --- | --- | --- |
| 472026 | Frozen | 0.448 [0.443, 0.454] | 0.924 |
| 472026 | Previously published | 0.846 [0.817, 0.874] | 0.971 |
| 24301 | Frozen | 0.447 [0.438, 0.458] | 0.922 |
| 24301 | Previously published | 0.837 [0.801, 0.861] | 0.967 |

The calibration protocol was frozen before its first capture. The crossover
decisions for the exact ten profiles were then recorded before seed 24301
was run. Serial led at zero mixes and the pool led at 8,192 mixes in every
profile on both tapes. Every paired interval excluded one in the predicted
direction on the second tape. These choices apply to the measured workload
and four-CPU runner. Both experiment costs, slower regimes and RSS medians
are retained in the [calibration](calibration/result.json) and
[held-out](heldout/result.json) summaries. The raw archives contain 72
process observations per tape with output, status, RSS and host telemetry.
[Calibration protocol](calibration/protocol.json) ·
[Recorded held-out decisions](heldout/executor-choices.json).

## Build and replay

See [provenance](provenance.json) for the three source identities and the
exact Linux build image. Build the two reference revisions with their
original library source, and build the candidate by applying
`git apply --unidiff-zero` with [round-batching.patch](round-batching.patch)
to the published reference. Apply [phold-32-lp.patch](phold-32-lp.patch) to
all three source trees. It adds only the target benchmark profile. Use
scratch directories named `base`, `published` and `candidate`. The
[build script](build-linux.sh) uses these directories mounted at `/session`
and the locked, cached dependencies in the pinned Linux x86-64 Rust image.
The timed execution took place on the physical Linux runner. The
[toolchain](linux-toolchain.txt) and [executable hashes](linux-binaries.sha256)
identify the actual builds. The build script prevents stale benchmark reuse
between checkouts and all timed executables had distinct hashes.

On the matching runner with the same partition configuration, place the
three executables, their checksum file and one protocol in a fresh working
directory and run `bash capture.sh`. The
[capture script](capture.sh) creates a temporary kernel CPU partition and
verifies that it releases the allocation afterward. Each tape used its own
temporary capture directory. The published archives can be analyzed from
the repository root using the verified checkout-local CPython interpreter.

```sh
evidence=conductor/archive/111-measured-performance/evidence/dedicated
scratch=$(mktemp -d)
for tape in calibration heldout; do
    mkdir "$scratch/$tape"
    cp "$evidence/$tape/protocol.json" "$scratch/$tape/"
    tar -xzf "$evidence/$tape/samples.tar.gz" -C "$scratch/$tape"
    .venv/bin/python -I "$evidence/analyze.py" "$scratch/$tape"
    cmp "$scratch/$tape/result.json" "$evidence/$tape/result.json"
done
```

## Explicit executor selection

The production PHOLD benchmark accepts `--executor serial` or
`--executor pooled` before generating its measurements. The selected mode
includes all five qualified profiles and reports the actual executor, input
hashes, per-LP results, worker count, and durations. The pooled selection
requires the `pdes-worker-pool` build. Serial can be selected at either
workload cost with either build. The existing paired benchmark remains the
default. The source exposes no automatic runtime crossover threshold.

From the repository root, use the checked-out implementation and Rust
1.99.1. Select the benchmark argument before starting the run.

```sh
compiler=$(rustup which --toolchain 1.99.1 rustc)
export PATH="$(dirname "$compiler"):$PATH"
cargo bench --locked -p kairo-ecs-pdes --features pdes-worker-pool --bench production -- \
  --phold --seed 24301 --event-work 0 --repetitions 2 --executor serial
cargo bench --locked -p kairo-ecs-pdes --features pdes-worker-pool --bench production -- \
  --phold --seed 24301 --event-work 8192 --repetitions 2 --executor pooled
```

The [selection receipt](executor-verification.json) compares the observed
state and tape hash for all five profiles at each cost with the dedicated
held-out capture. It also verifies the prior four-profile paired output,
the serial override at 8,192 mixes in both builds, and rejected inputs.
The selected-mode correctness runs used macOS ARM64.

## Held-out model replay

The Apple M1 Max replays exercised the same candidate runtime as the Linux
capture, using Rust 1.99.1 and Python 3.15.2. The optimized Rust spatial
fixture and independent integer replay reproduced every ordered transition
and final state. The three-engine FIFO comparison with SimPy 4.1.2
reproduced the archived trace and terminal states for 1,024 arrivals.
Neither fixture is part of the dedicated timing gate.
[Spatial receipt](spatial-replay.json) · [Queueing receipt](queueing-replay.json).

The queueing receipt's `source_base` identifies the comparison-script
checkout. The executable was built in the Track 111 worktree at
`5ab0c0e4a7ee639f10a41db403ddce89723d1083` with
`cargo build --locked --offline --release -p kairo-ecs-pdes --features pdes-worker-pool --example track111_queueing`.
The receipt records the resulting executable hash with the oracle and wheel
hashes. The [queueing](../queueing/README.md) and
[spatial](../spatial/README.md) fixtures have the reproduction commands.
