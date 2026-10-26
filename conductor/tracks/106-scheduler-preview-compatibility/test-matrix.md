# Track 106 test matrix

All commands are pending until actually executed. Python helper commands use
the verified repo-local `.venv/bin/python` (3.15.1); Rust checks use the
repository toolchain (`1.99.1`).

| Gate | Oracle | Status |
|---|---|---|
| dependency-106-preview-behaviour | `cargo test --locked -p kairo-ecs-core --test track106_scheduler_preview` checks empty, ordered, cancelled-only, live-cancellation, repeat-peek and subsequent dispatch behavior using the shared fixture | Passed locally: 3 Track 106 tests; baseline first failed only on the intended missing API |
| core-library regression | `cargo test --locked -p kairo-ecs-core` | Passed: 47 tests across unit, integration, conformance, property and Track 106 suites |
| API rustdoc | `cargo doc --locked -p kairo-ecs-core --no-deps` | Passed; generated `target/doc/kairo_ecs_core/index.html` |
| formatting | `cargo fmt --all --check` | Passed |
| shared fixture contract | `pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1` | Passed locally; fixture remains planned in the generic runner manifest until a runner adapter is registered |
| track registration | `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`; `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Passed: artifacts 0 errors/0 warnings, phase gates 0 errors/0 warnings, DAG 0 errors/0 warnings |
| owner review | Core scheduler, conformance, and API Governance reviewers confirm the exact candidate; retain review receipt | Pending |
| CareOps consumer | Isolated consumer checkout: `cargo check --locked -p careops-ed-cli`, focused R10.3 references, `rustfmt --check`; exact parent commit and lockfile recorded | Pending |
| git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Pending |

Unexecuted, unavailable, or owner-pending gates remain Pending and block track
acceptance and the parent pin update.
