# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Conductor artifact and DAG integrity | `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` | Structural track registration check only |
| Phase registry | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Required before phase closure; a structural pass does not accept an implementation phase |
| Clean source publication | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Required after the reviewed implementation commit is pushed; pending for implementation closeout |
| Capability/claim consistency | `node scripts/validation/validate-frontier-evidence.mjs` | Structural check only; no new runtime claim |
| Neutral cohort transition and exposure schemas | Rust Arrow/Polars C Data round-trips, time limbs, valid state and visit ordering, unit negotiation and proximity nullability | Passed from the reviewed extracted source tree |
| CRN counterfactual substream coupling | Equal pre-intervention traces, deterministic replay, and a shared sampled exit after an extra intermediate state | Passed from the reviewed extracted source tree |
| Semi-Markov competing-risk convergence | Actual imported trajectories from 100,000 entities vs analytical early and late survival and cause-specific incidence | Passed within four binomial standard errors per outcome |
| Downstream `tobac` consumer fixture | Locked standalone fictional fixture and executable, including a retained buffer after early stream release | Five tests and executable passed from the reviewed extracted source tree |

Run both `cargo test --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml`
and `cargo run --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml`.
The Arrow platform workflow runs these commands on Linux, macOS and Windows.
