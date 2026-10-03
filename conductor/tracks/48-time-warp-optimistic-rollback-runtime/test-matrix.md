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

Actual new-runtime evidence: independent 15 held-outs pass on Rust1.98.1 and1.76.0; combined pdes,time-warp crate94 tests pass at3cd6d56. The strict local benchmark collector `python3 -B benches/pdes/collect_time_warp_evidence.py` passed at clean ec9828e. Raw outputs and hashes are preserved in benches/pdes/evidence/track48-ec9828e/. Collector negative tests use `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s benches/pdes -p test_collect_time_warp_evidence.py -v`. Workspace, phase and hosted gates remain pending; these local passes are not distributed acceptance.
