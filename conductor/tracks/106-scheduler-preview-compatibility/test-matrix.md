# Track 106 test matrix

Statuses below distinguish completed local checks from hosted and reviewer-agent
gates that remain open. Python helper commands use the verified repo-local
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
| hosted PR checks | GitHub Actions at implementation head `29abb88718fb1e76b7dedf81766f7d49058dc5d2`; docs reconciliation commit `fec06b928845b44352a2ac1edcf3c81712b3175a` | All 18 workflow runs on the implementation head completed successfully. Fresh checks on the docs-only PR head are running; refresh before claiming hosted validation current. |
| core scheduler agent review | Reviewer agent/subagent assesses exact candidate from core scheduler lens; record SHA, findings and coordinator disposition | Prior head `89e713e` review found no correctness defect and flagged missing `Some(EntityId)` coverage; assertion added, exact final-head re-review pending |
| conformance agent review | Reviewer agent/subagent assesses exact candidate fixture/readiness from conformance lens; record SHA, findings and coordinator disposition | Prior head `89e713e` confirmed `planned` status is correct and raised descriptive fixture-field coverage; ordering and requirement assertions added, exact final-head re-review pending |
| API compatibility agent review | Reviewer agent/subagent assesses exact candidate from API compatibility lens; record SHA, findings and coordinator disposition | Prior head `89e713e` found ADR omitted affected root `crates/kairo-ecs-core`; root added, exact final-head re-review pending |
| CareOps consumer | Disposable CareOps checkout: `cargo check --locked --offline -p careops-ed-cli`, R10.3 references and rustfmt | Candidate-specific lock: passed; 2 reference tests passed. Original lock: locked check failed before compile because lock update is required. Candidate lock delta awaits acceptance. Parent pin and original lock restored unchanged. |
| git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Pending for documentation synchronization commit |

Unexecuted, unavailable, or reviewer-agent-pending gates remain Pending and block track
acceptance and the parent pin update.
