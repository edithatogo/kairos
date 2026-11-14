# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Conductor artifact and DAG integrity | `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` | Structural track registration check only |
| Phase registry | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Required before phase closure; a structural pass does not accept an implementation phase |
| Clean source publication | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Required after the reviewed implementation commit is pushed; pending for implementation closeout |
| Capability/claim consistency | `node scripts/validation/validate-frontier-evidence.mjs` | Structural check only; no new runtime claim |
| Neutral cohort transition Arrow schema | Positive and negative Rust Arrow/Polars batch round-trip fixtures (monotonic clock, valid state IDs, rejected negative dwell times) | Pending implementation |
| CRN counterfactual substream coupling | Paired-seed intervention test proving zero divergence before policy activation time $t_0$ even when post-$t_0$ event counts differ | Pending implementation |
| Semi-Markov competing-risk convergence | Large-sample ($N \ge 10^5$) microsimulation vs analytical semi-Markov cohort survival/incidence oracle | Pending implementation |
| Downstream `tobac` consumer fixture | Standalone public fictional cohort fixture running without `tobac` repository access | Pending implementation |
