# Track 111 review

## Phases 1–3

The independent review verified that the pooled batch recomputes
readiness after every ordered round commit. Logical time, per-LP event
limits, null messages and the event budget are evaluated each round.
The review requested an explicit budget-stop recovery check. The
extended worker identity test now exhausts its budget, resumes the same
horizon and drains another horizon with the original worker.

The reviewer recomputed both captures from 144 observed processes and
720 exact semantic rows. Both baseline and incremental thresholds passed
on both tapes, and all ten predeclared executor choices held on the unseen
tape. The selected mode now includes the 32-LP primary profile, with its
tape and state checked against the held-out sample. The serial override
was exercised in both builds. The reviewer also independently reproduced
the FIFO and spatial result hashes and verified the distinct queueing
oracle and executable source identities.
[Raw capture and provenance](evidence/dedicated/README.md).

Final local validation ran in the owning worktree with Rust 1.99.1 and
pinned Clippy. The focused commands are the same for `pdes` and
`pdes-worker-pool`.

| Check | Outcome |
| --- | --- |
| `cargo test --locked --offline -p kairo-ecs-pdes --features FEATURE --test production_parity --test production_protocol --test track47_conservative_runtime` | Exit 0, 19 scoped and 20 pooled cases |
| `cargo clippy --locked --offline -p kairo-ecs-pdes --features FEATURE --bench production -- -D warnings` | Exit 0, both builds |
| `cargo fmt --all --check` | Exit 0 |
| Locked `cargo bench --no-run` and direct selected and paired runs | Exit 0, five selected cases per cost plus four negative inputs. [Receipt](evidence/dedicated/executor-verification.json) |
| Extract both raw archives and rerun the analyzer with CPython 3.15.2 | Exit 0, both result JSON files reproduced byte for byte |

The review accepts Phases 1–3 for publication. Keep the measured
selection explicit and use the existing ownership for broader distributed,
device and allocation claims.

## Phase 0

The independent reviewer verified the frontier reduction against the
exclusive bound and event-budget tests, reconstructed the raw PHOLD
statistics and checked the ordered state and reported counters. Both
compiler captures met the predeclared local adoption rule. The pinned
32-LP target took 23.4% less median paired end-to-end time. The largest
other pinned median increase was 4.7%.

The reviewed baseline includes native worker stacks, allocation and queue
profiles, component layout and RSS, and separate snapshot, encode,
local transfer and decode timings. The independent review verified the
FIFO trace with a separate tick-level reference and the spatial trace
with independent integer neighbor search and signal replay.

The reproduction instructions now identify the repository and scratch
paths explicitly. The original local binaries identify Homebrew rustc
1.100.0 with Cargo 1.99.1. The receipts were corrected and the primary
comparison was repeated with explicitly selected rustc and Cargo 1.99.1.
The spatial model already used the pinned toolchain.

The test that drains the whole eligible queue produced `[1, 7, 3]` in
place of `[1, 3, 7]`. Keep one eligible event per LP and the ordered
round commit. Phase 0 is accepted against
[the verified publication in PR #470](https://github.com/edithatogo/kairos/pull/470).

## Local validation

The reviewed evidence commit is `30ce055aa8cd1ffb403f4c40d8a9d1fab73b86b0`.
The checks used the same runtime source and lockfile on the Apple M1 Max in
the [owning checkout](handoff.md#remaining-acceptance).
The results below use Rust 1.99.1. The SimPy comparator used its
qualified checkout-local CPython 3.15.2 and the verified SimPy 4.1.2 wheel.

| Command | Outcome |
| --- | --- |
| `cargo test --locked -p kairo-ecs-pdes --features pdes --test production_parity --test production_protocol --test track47_conservative_runtime` | Exit 0, 19 cases |
| `cargo test --locked -p kairo-ecs-pdes --features pdes-worker-pool --test production_parity --test production_protocol --test track47_conservative_runtime` | Exit 0, 20 cases |
| `cargo test --locked -p kairo-ecs-pdes --all-features` | Exit 0, 212 cases |
| `cargo clippy --locked -p kairo-ecs-pdes --all-targets --all-features -- -D warnings` | Exit 0 |
| `cargo test --locked -p kairo-ecs-state --test query_determinism` | Exit 0, eight cases |
| `cargo test --locked -p kairo-ecs-cli --lib parallel_ensemble_runner_parity` | Exit 0, one case |
| `cargo fmt --all --check` | Exit 0 |
| `just ci` | Exit 101 during workspace lint compilation because this Mac lacks the HDF5 headers |

The exact-base hosted coverage check initially counted four fewer covered
lines with the same uncovered count after the frontier simplification. The
directed topology regression now asserts the typed errors for missing LP
entries and unknown sources. A pinned Rust 1.99.1 instrumented run hit both
constructor guards that the original hosted profile missed.
The final hosted workspace comparison passed at 38,091 of 43,065 lines
against 38,093 of 43,069 on the protected base. The tested merge tree
matched the published commit.

The pinned queue-consumer rebuild ran the same three engines and returned
the archived ordered result SHA-256
`e6df5ef8bfc08969042d36d0de9cc74f5f42e791dfa542eaabca048a998d4dc9`.
The original queueing timings remain attached to their observed compiler.
[Queueing and wheel receipt](evidence/queueing/README.md).

The [frontier](evidence/frontier-fold/pinned-receipt.json),
[profiling](evidence/profiling/receipt.json),
[spatial](evidence/spatial/receipt.json) and
[backend](evidence/backends/receipt.json) receipts identify the commands,
toolchains, sample hashes and measurement boundaries. The local Rust
validation logs and pinned queue replay receipt are retained under
`/Volumes/PortableSSD/kairos-track111-closeout.gzk4Uv`.
