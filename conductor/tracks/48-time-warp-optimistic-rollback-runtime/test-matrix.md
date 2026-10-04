# Track 48 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Time Warp tests | `cargo test -p kairo-ecs-pdes --features time-warp` | Implementation |
| Conservative regression | `cargo test -p kairo-ecs-pdes --features pdes` | Review |
| Benchmark compile | `cargo check --benches -p kairo-ecs-pdes --features pdes,time-warp` | Review |
| Local Time Warp evidence manifest | `node scripts/validation/validate-hpc-parity-evidence.mjs` | Evidence boundary |
| Full workspace | `just ci` and `cargo test --workspace --all-features --doc --locked` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Resolve the installed toolchain at dispatch; a `rustup run ... just` wrapper does not bind its child compiler when Homebrew comes first in PATH. Before native commands, resolve `rustup which --toolchain 1.98.1 rustc` and matching Cargo/rustdoc, prepend that toolchain bin directory, set absolute `RUSTC`/`RUSTDOC` and `RUSTUP_TOOLCHAIN=1.98.1`; bind matching LLVM tools for `just ci`. Record resolved paths/versions and actual Cargo compiler cache. The benchmark collector performs its own reviewed resolution and rejects config/environment drift. For MSRV use matching1.76 Cargo/rustc/rustdoc with the same explicit binding.

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
The local evidence manifest gate validates manifest shape and claim boundaries
only. It is not a distributed optimistic rollback proof and must not be used to
advance Track 48 to `Done` without live distributed rollback artifacts.


Current native dispatch uses installed macOS Rust 1.98.1, with Rust 1.76.0 for MSRV verification. Historical Windows GNU commands are not the current dispatch; no Windows native Rust pass is claimed. Record current command/cwd/source/toolchain/input/output hashes/exit/log per attempt. New runtime protocol, sequential parity and generation-bitset tests supplement the feature lane. Run actual sparse/dense optimistic-vs-conservative benchmark smoke after API acceptance. The live distributed rollback evidence gate and Track 49 handoff remain required for Done; local native/hosted checks cannot substitute for them.

## Historical wrapper results and correction

Actual new-runtime behavioral evidence: independent 15 held-outs and the combined 94-test pdes,time-warp lane passed at 3cd6d56. A later audit of both Cargo target caches identifies actual Homebrew Rust 1.99.0, withdrawing the nominal Rust1.98.1/MSRV1.76.0 labels until explicit compiler-path reruns pass. The strict local benchmark collector `python3 -B benches/pdes/collect_time_warp_evidence.py` passed at clean ec9828e. Raw outputs and hashes are preserved in benches/pdes/evidence/track48-ec9828e/. Collector negative tests use `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s benches/pdes -p test_collect_time_warp_evidence.py -v`. At7a432ab the coordinator reports collector nine-case, local manifest, phase, DAG and RequireClean gates passing. Coordinator source-bound just ci at7a432ab passes with explicit Rust1.98.1 PATH/RUSTC, matching LLVM tools and fresh target (458 tests, zero skipped, core coverage92.59%, fmt/Clippy/rustdoc/deny/audit). Receipt: artifacts/track48-final-validation/receipt-resolved-pinned.json. Earlier LLVM/compiler-resolution failures remain failed attempts; actual Rust1.76 crate rerun and hosted checks are pending. Benchmark compiler metadata also awaits reconciliation. These local results are not distributed acceptance.

## Explicit MSRV evidence accepted — 2026-10-03

At source7a432ab, matching Cargo/rustc/rustdoc1.76.0 with explicit PATH/RUSTC/RUSTDOC and fresh target passed94 local pdes,time-warp tests plus doc tests. Independent review verified all bound source hashes, actual Cargo compiler cache and successful log SHA c9bca00d9a453414809181c1950294a4189169afd601ce77d77350dfd3078b01. Canonical unchanged copies are in benches/pdes/evidence/track48-msrv-7a432ab/. This resolves the local MSRV proof gap; historical compiler labels remain withdrawn. Benchmark compiler correction, exact-head hosted/security and live distributed gates remain pending.

## Final compiler-bound local delivery — 2026-10-03

Reviewed collector504aa4a integrated as68b8d7a. Absolute Cargo/rustc1.98.1, binary/configuration hashes, wrapper/selector refusal and fixed baseline flags resolve compiler drift. At clean68b8d7a,12 collector tests and strict fresh benchmark passed; independent review verified actual Cargo cache1.98.1/LLVM22.1.8,16 unchanged source hashes, raw logs and four parity cases with five alternating samples. Canonical evidence: benches/pdes/evidence/track48-68b8d7a/, SHAe7d08f74b0c448cac281cd95199e24cf9d34218a303a80ad0a66f8e1323cbc8c. Scope is tiny single-host lightweight-handler run-call smoke, with substantial timing variation and excluded setup/extraction/validation/fossil costs. No general speedup, CPU-concurrency or distributed claim.

Explicit1.98 workspace CI458/coverage92.59% and matching1.76 local PDES94 tests are accepted. The workspace doc-test command at7c35f2a exited0 across25 targets with zero runnable examples, separately preserved in benches/pdes/evidence/track48-doctests-7c35f2a/. CHANGELOG d62891e and global evidence synchronization eb02a14 are committed; previous pending statements describe historical review snapshots. Exact-head push/hosted security/package gates and live distributed acceptance remain pending. Track48 stays In Progress; Track49 dependency/production authority unchanged.

During compiler-fix coordination, an expired worker lease was explicitly recovered only after interruption; exact edits were preserved in stashes. A displayed cooperative lease token was revoked and rotated before continuation. No displayed token is committed or remains active. Original failed attempts and superseded raw evidence remain preserved.

## Final local gates and forward workspace MSRV

At c87df47, pinned1.98 benchmark compilation,12 collector tests, local manifest, phase/DAG and strict clean Git checks pass; immutable copies are in benches/pdes/evidence/track48-local-gates-c87df47/. Matching absolute Cargo/rustc/rustdoc1.76 also passes locked all-feature workspace lib/bin check excluding Wasm; actual compiler cache and log verified in benches/pdes/evidence/track48-workspace-msrv-c87df47/. This forward evidence resolves current compiler compatibility without retrospectively asserting compiler identity for old wrapper-labelled Track47 MSRV receipts. No Windows, foreign-package or distributed proof is claimed.

## Accepted owned native leaf — 4 October 2026

The [current acceptance record](owned-native-acceptance-20261004.md) supersedes earlier pending local/compiler snapshots: source9bf478e passes166 PDES tests on each explicitly bound1.98.1/1.76.0 compiler; fullCI530/530,coverage92.59%,locked doctests,benchmark smoke/12 collector tests,manifest and local phase/clean gates pass. Original failed receipts remain historical. Track48 stays In Progress pending hosted native delivery and live distributed acceptance. Changed interface semantics expire the previous conditional Track49 entry disposition; fresh specific human disposition and reviewed production packet are required. Global status/phase files and parent pin remain with their current parallel owners.
