# Matched PHOLD measurements

The production PDES benchmark's `--phold` mode starts 256 events and
dispatches 2,048 events over eight generations. Each event except the last
in its chain emits one successor with a seeded destination. Delays span
one to four ticks at one-tick lookahead and eight to eleven ticks at
eight-tick lookahead. Remote successors choose another logical process
uniformly from the configured neighbors. Local successors stay on the
emitting process.

The Rust serial scheduler and the conservative runtime start from the same
input tape and execute the same transition. The serial scheduler uses source
LP as the priority for equal timestamps. Both paths match the per-LP event
counts, order-sensitive checksums and remote emission counts on every pair.
All four profiles dispatch 2,048 events and emit 1,792 successors. The two
remote targets yield 445 and 1,356 actual cross-LP messages.

## Results

The table reports the median paired serial time divided by PDES time.
Values above one favor PDES. Brackets show descriptive 95% percentile
intervals from 10,000 paired bootstrap draws with seed 111.

| LPs | Lookahead ticks | Remote target | Mixes per event | Scoped | Pooled |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4 | 1 | 25% | 0 | 0.011 [0.010, 0.011] | 0.040 [0.039, 0.043] |
| 4 | 1 | 75% | 0 | 0.010 [0.010, 0.010] | 0.040 [0.039, 0.043] |
| 4 | 8 | 75% | 0 | 0.010 [0.010, 0.010] | 0.040 [0.039, 0.042] |
| 8 | 8 | 75% | 0 | 0.012 [0.012, 0.012] | 0.040 [0.039, 0.041] |
| 4 | 1 | 25% | 2,048 | 0.387 [0.375, 0.404] | 0.791 [0.770, 0.797] |
| 4 | 1 | 75% | 2,048 | 0.389 [0.384, 0.394] | 0.786 [0.769, 0.799] |
| 4 | 8 | 75% | 2,048 | 0.392 [0.388, 0.396] | 0.804 [0.794, 0.817] |
| 8 | 8 | 75% | 2,048 | 0.497 [0.493, 0.500] | 1.058 [1.026, 1.104] |
| 4 | 1 | 25% | 8,192 | 1.144 [1.140, 1.146] | 1.535 [1.527, 1.556] |
| 4 | 1 | 75% | 8,192 | 1.138 [1.133, 1.146] | 1.538 [1.521, 1.551] |
| 4 | 8 | 75% | 8,192 | 1.166 [1.144, 1.174] | 1.596 [1.547, 1.631] |
| 8 | 8 | 75% | 8,192 | 1.536 [1.529, 1.546] | 2.097 [1.991, 2.148] |

Each case uses one first run and ten fresh runtime pairs. Serial goes first
on even pairs and PDES on odd pairs, starting at zero. The three event costs
alternate which PDES build goes first. Raw samples retain setup, dispatch
and total nanoseconds separately. Dispatch includes lazy pool startup where
applicable. Peak RSS covers both executors' work within each benchmark
process. The local machine had other background activity, including a process
using about two cores. The intervals describe these runs on this host.
Dedicated-runner thresholds, held-out models and automatic executor
selection require separate qualification.
[Raw runs](matched.jsonl) · [Computed medians and intervals](summary.json)

## Execution receipt

- Host Apple M1 Max, 8 performance cores, 2 efficiency cores, 32 GiB RAM.
  macOS 27.2.1 build 26B131 and Rust/Cargo 1.99.1 on aarch64-apple-darwin.
  Captured 2026-11-24 from 19:23:20 to 19:23:38 UTC.
- Source base `2f01fd229d1896fdbc5a63695ac29ddadaa513a1`. The benchmark
  source in both binaries had Git blob
  `ac3ff986942e5abf6b7b4d15cc63b8a2e92ad4ca` and SHA-256
  `de48166905d1f263d952edf8ca3d0f7942695b56e991d09d4e1c6b5a494fd65e`.
  Cargo.lock SHA-256 was
  `df26476d07b6323f89bb02819aaff00a12f00c15bdd0114c91d082cd088f0637`.
- Build and execution ran from
  `/Volumes/PortableSSD/kairos-111-phold.1haixC/source`. Both optimized
  binaries used the same source and the `pdes` and `pdes-worker-pool`
  feature sets respectively. The six direct commands, binary hashes and zero
  exit statuses are in the raw records. The capture and analysis interpreter
  was repo-local CPython 3.15.2.
- Raw SHA-256
  `8bac643ce25d394dd85cb051697ac54efc213a3c9809cc75b9ea31b6ce1c06bd`.
  Summary SHA-256
  `242d6d6528d7cf717b707f69a63d765cabd2387edce66dac157da003bea91b56`.
- Both feature sets passed `production_parity`, `production_protocol` and
  Clippy for the benchmark. Rebuilding the pinned base confirmed both legacy
  output schemas and their deterministic contents for both executors after
  removing the timing and throughput fields. Invalid repetition count was
  rejected. A pooled run at seed `18446744073709551615` also passed parity.

Run from the source directory with Rust 1.99.1 selected. Repeat with
`pdes-worker-pool` in place of `pdes` and supply each recorded cost.

```sh
cargo bench --locked --offline -p kairo-ecs-pdes --features pdes \
  --bench production -- --phold --seed 472026 --repetitions 10 --event-work 8192
```
