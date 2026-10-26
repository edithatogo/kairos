# Track 106 test matrix

Statuses below distinguish completed Track 106 acceptance from separate
downstream migration work. Python helper commands use the verified repo-local
`.venv/bin/python` (3.15.1); Rust checks use repository Rust/Cargo `1.99.1`.

| Gate | Oracle | Status |
|---|---|---|
| dependency-106-preview-behaviour | `cargo test --locked -p kairo-ecs-core --test track106_scheduler_preview` checks empty, ordered, cancelled-only, live-cancellation, repeat-peek and subsequent dispatch behavior using the shared fixture | Passed locally on rebased candidate; 3 Track 106 tests |
| core-library regression | `cargo test --locked -p kairo-ecs-core` | Passed locally on rebased candidate: 51 tests across unit, integration, conformance, property and Track 106 suites |
| API rustdoc | `cargo doc --locked -p kairo-ecs-core --no-deps` | Passed locally; generated `target/doc/kairo_ecs_core/index.html` |
| formatting | `cargo fmt --all --check` | Passed locally |
| shared fixture contract | `pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1` | Passed locally; fixture remains planned in the generic runner manifest until a runner adapter is registered |
| Conductor registration | `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`; `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Artifact and DAG passed in prior verification; phase-gate validator is being rerun after this reconciliation |
| track coverage | `pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo` | Passed after adding the required machine-readable Track 106 row |
| hosted PR checks | GitHub Actions at PR head `e3c7dc2f9f66e3c980b9d3cfb631be31842368c5` | Required checks passed on the current head, including Rust 1.99.1 workspace/verification/core-quality, Linux and Windows validation, docs quality, regression, benchmark, dependency review and CodeQL. Optional Pages deployment and Codecov OIDC upload were skipped. |
| core scheduler agent review | Reviewer agent/subagent assesses exact candidate from core scheduler lens; record SHA, findings and coordinator disposition | Passed on source commit `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb`; the earlier `Some(EntityId)` coverage gap is fixed; no remaining scheduler defect reported. Review only; reviewer did not run tests. |
| conformance agent review | Reviewer agent/subagent assesses exact candidate fixture/readiness from conformance lens; record SHA, findings and coordinator disposition | Passed on source commit `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb`; fixture `planned` status is correct and ordering, requirement and entity projection assertions are present. No blocking finding. |
| API compatibility agent review | Reviewer agent/subagent assesses exact candidate from API compatibility lens; record SHA, findings and coordinator disposition | Passed on source commit `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb`; ADR names `crates/kairo-ecs-core` and retained CareOps API shape matches. Reviewer did not assess tests or CI. |
| CareOps consumer and lock-delta review | Candidate source `e3c7dc2`, retained pin `44261eb5`, original lock hash `9ad9d38b...` | Candidate source compiled and 2 reference tests passed with regenerated lock hash `24747479...`. Reviewer-agent found this candidate pin also migrates RNG behavior and lacks old-pin replay parity; do not adopt its broad lock/pin delta here. Retained pin plus original lock: locked check passed, both reference tests passed, focused Rust 1.99.1 rustfmt passed. Original pin and lock restored unchanged. |
| git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Pending for archive-closeout commit |

Track 106's upstream API, review and CI gates pass. The unrelated RNG migration is
not accepted for a CareOps pin update and remains a separate downstream task.
