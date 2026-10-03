# Track 48 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Time Warp tests | `rustup run 1.98.1 cargo test -p kairo-ecs-pdes --features time-warp` | Implementation |
| Conservative regression | `rustup run 1.98.1 cargo test -p kairo-ecs-pdes --features pdes` | Review |
| Benchmark compile | `rustup run 1.98.1 cargo check --benches -p kairo-ecs-pdes --features pdes,time-warp` | Review |
| Local Time Warp evidence manifest | `node scripts/validation/validate-hpc-parity-evidence.mjs` | Evidence boundary |
| Full workspace | `rustup run 1.98.1 cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
The local evidence manifest gate validates manifest shape and claim boundaries
only. It is not a distributed optimistic rollback proof and must not be used to
advance Track 48 to `Done` without live distributed rollback artifacts.


Current native dispatch uses installed macOS Rust 1.98.1, with Rust 1.76.0 for MSRV verification. Historical Windows GNU commands above are replaced for local execution; no Windows native Rust pass is claimed. Record current command/cwd/source/toolchain/input/output hashes/exit/log per attempt. New runtime protocol, sequential parity and generation-bitset tests supplement the feature lane. Run actual sparse/dense optimistic-vs-conservative benchmark smoke after API acceptance. The live distributed rollback evidence gate and Track 49 handoff remain required for Done; local native/hosted checks cannot substitute for them.

Actual new-runtime behavioral evidence: independent 15 held-outs and the combined 94-test pdes,time-warp lane passed at 3cd6d56. A later audit of both Cargo target caches identifies actual Homebrew Rust 1.99.0, withdrawing the nominal Rust1.98.1/MSRV1.76.0 labels until explicit compiler-path reruns pass. The strict local benchmark collector `python3 -B benches/pdes/collect_time_warp_evidence.py` passed at clean ec9828e. Raw outputs and hashes are preserved in benches/pdes/evidence/track48-ec9828e/. Collector negative tests use `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s benches/pdes -p test_collect_time_warp_evidence.py -v`. At7a432ab the coordinator reports collector nine-case, local manifest, phase, DAG and RequireClean gates passing. Coordinator source-bound just ci at7a432ab passes with explicit Rust1.98.1 PATH/RUSTC, matching LLVM tools and fresh target (458 tests, zero skipped, core coverage92.59%, fmt/Clippy/rustdoc/deny/audit). Receipt: artifacts/track48-final-validation/receipt-resolved-pinned.json. Earlier LLVM/compiler-resolution failures remain failed attempts; actual Rust1.76 crate rerun and hosted checks are pending. Benchmark compiler metadata also awaits reconciliation. These local results are not distributed acceptance.

## Explicit MSRV evidence accepted — 2026-10-03

At source7a432ab, matching Cargo/rustc/rustdoc1.76.0 with explicit PATH/RUSTC/RUSTDOC and fresh target passed94 local pdes,time-warp tests plus doc tests. Independent review verified all bound source hashes, actual Cargo compiler cache and successful log SHA c9bca00d9a453414809181c1950294a4189169afd601ce77d77350dfd3078b01. Canonical unchanged copies are in benches/pdes/evidence/track48-msrv-7a432ab/. This resolves the local MSRV proof gap; historical compiler labels remain withdrawn. Benchmark compiler correction, exact-head hosted/security and live distributed gates remain pending.
