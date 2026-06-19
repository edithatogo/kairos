# Track 55 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Benchmark smoke | `python benches/benchmark_smoke.py` | Review |
| Regression threshold coverage | `python benches/regression/compare.py` | Review |
| Scaling certification contract | `node scripts/validation/validate-hpc-scaling-certification.mjs --self-test` | Review |
| Weak scaling run | `python benches/hpc/run_scaling.py --mode weak --manifest conductor/tracks/55-end-to-end-weak-strong-scaling-certification/evidence.json` | Done |
| Strong scaling run | `python benches/hpc/run_scaling.py --mode strong --manifest conductor/tracks/55-end-to-end-weak-strong-scaling-certification/evidence.json` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
