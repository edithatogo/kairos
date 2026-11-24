# Track 55 two-host ring measurement

Source base is 2f01fd229d1896fdbc5a63695ac29ddadaa513a1. The experiment adds
one example to a Git-free archive, declares two existing workspace
dependencies for the example, and adds one lock edge. The production
model, deterministic tape, serial scheduler oracle, native dispatcher and
framed transport are the existing Kairos code at that base.

## Method

The Mac always hosts the coordinator. One-host cases run both logical
processes in a single Mac worker. Two-host cases place one worker on the
Mac and one on a separate Linux Intel i3-N300 NAS. Both workers execute
the same Rust event model through the native scheduler. After all tick 0
replies, the coordinator sends the actual ordered follow-ups to their
destination workers for tick 1. Each worker includes the complete final
processed-count/checksum state for its logical processes. The two-host
view exchanges four framed TCP messages over loopback SSH forwards per
case. One-host uses the same Kairos transport on loopback. The transport
is GrpcSocketTransport's framed TCP profile.

The Rust coordinator's monotonic clock measures dispatch submission
through the final replies, including synchronization, serialization,
and transport. Worker startup, loading the serial oracle and the final
parity assertions occur outside the measured interval. The native
worker timers include local queue setup and dispatch. The workload uses
two logical processes, seed 472026, 8,192 dependent mixes per event and
an equal number of roots and causal follow-ups. One warm-up pair is
discarded for each group. The ten measured pairs alternate host order.

Strong scaling uses 4,096 total events in both layouts. Weak scaling
uses 2,048 events per physical worker. The native serial oracle accepted
all 44 rows, including warm-ups. A separate Python reference reproduced
both tape digests and both final state vectors exactly.

## Results

| Workload | One host median ms | Two hosts median ms | Median paired T1/T2 | Nominal efficiency |
| --- | ---: | ---: | ---: | ---: |
| Fixed 4,096 events | 144.214 | 156.026 | 0.929 | 46.4% |
| 2,048 events per host | 72.988 | 158.758 | 0.462 | 46.2% |

Two hosts took longer by the median on both workloads. The strong
paired ratio ranged from 0.781 to 1.033. Ratios are descriptive for the
heterogeneous hosts, recorded load and tunneled path. One-minute load
was 24.93 to 22.63 on macOS and 1.68 to 1.87 on Linux. The integrated
checkpoint/restart, rollback, MPI, scheduler, NUMA, I/O, GPU and
co-simulation matrix remains open.
[Raw measurements](evidence/measurements.jsonl) ·
[Summary](evidence/summary.json) · [Hashes](SHA256SUMS).

## Reproduction

Extract the Cargo workspace at the cited revision into a fresh directory.
Copy `track55_physical.rs` into `crates/kairo-ecs-grpc/examples` in that
workspace. Apply `manifest-overlay.diff` and `lock-overlay.diff` there
with `git apply`. The macOS build used Rust 1.99.1 with
`CARGO_PROFILE_RELEASE_DEBUG=0` and this command.

    rustup run 1.99.1 cargo build --locked --offline --release -j 3 -p kairo-ecs-grpc --example track55_physical

The Linux x86-64 build used Apple's isolated AMD64 container with
Rosetta, cached rust:1.99.1-slim-trixie and offline Cargo inputs. It set
RUSTFLAGS="-C target-feature=+crt-static" and CARGO_PROFILE_RELEASE_DEBUG=0.

    cargo build --locked --offline --release -j 4 --target x86_64-unknown-linux-gnu -p kairo-ecs-grpc --example track55_physical

Run `run_track55.py` with the repo-local CPython 3.15.2 interpreter from
a private copy of this packet. Place the Mac build under `mac-target`
in that copy. Stage the x86-64 static PIE on the remote host at a private
unprivileged directory recorded in `remote-stage.private`. Verify the
uploaded executable's SHA-256 before running. The script uses the
`pleiadesnas` SSH alias and loopback-only forwarding. Both native workers
and the controller exited zero.

Run `analyze_track55.py` with the same interpreter to independently
replay the retained `evidence/measurements.jsonl` and reproduce the
summary. The analysis uses the Python standard library.
