# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Conductor artifact and DAG integrity | `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` | Structural track registration check only |
| Phase registry | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Required before phase closure; a structural pass does not accept an implementation phase |
| Clean source publication | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Provider Phases 0–2 are integrated. The receiving PR has its own publication gate. |
| Capability/claim consistency | `node scripts/validation/validate-frontier-evidence.mjs` | Structural check only; no new runtime claim |
| Neutral cohort transition and exposure schemas | Rust Arrow/Polars C Data round-trips, time limbs, valid state and visit ordering, unit negotiation and proximity nullability | Passed from the reviewed extracted source tree |
| CRN counterfactual substream coupling | Equal pre-intervention traces, deterministic replay, and a shared sampled exit after an extra intermediate state | Passed from the reviewed extracted source tree |
| Semi-Markov competing-risk convergence | Actual imported trajectories from 100,000 entities vs analytical early and late survival and cause-specific incidence | Passed within four binomial standard errors per outcome |
| Downstream `tobac` consumer fixture | Locked standalone fictional fixture and executable, including a retained buffer after early stream release | Five tests and executable passed from the reviewed extracted source tree |
| Standalone Arrow IPC output | Emit the baseline, policy and exposure streams through the fixture CLI and read typed columns, requiredness, units and original provenance | Seven locked fixture tests and an independent PyArrow 26.0.1 readback passed. Both cohort arms shared all 1,451 pre-intervention rows. |
| Installed Tobac summary and ownership | Read the three IPC streams in the isolated receiving wheel, verify the inclusive summary and retain the primitive buffer across producer release | macOS ARM64 with CPython 3.15.2 passed at `ce9b621`. Both arms accounted for all 4,096 initial members. [Exact receipt](handoff.md#installed-tobac-candidate--24-november-2026) |
| Signed receiving inventory | Tobac signs the exact candidate inventory through its existing authority, then all hosted lanes and receiving-main readback pass | Three jobs stopped at the inventory check before testing. [Reproduced blocker](handoff.md#signed-inventory-renewal--25-november-2026) |

Run both `cargo test --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml`
and `cargo run --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml`.
The Arrow platform workflow runs these commands on Linux, macOS and Windows.
