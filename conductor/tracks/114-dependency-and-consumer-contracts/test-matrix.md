# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Bounded task integrity | `python scripts/maturity_tasks.py check`; `python -m unittest discover -s tests -p test_maturity_tasks.py -v` | Structural task/path/dependency/target checks only; no runtime or scientific promotion |
| Scheduled movement | `kairo-ecs-abm` tests `movement` and `petgraph_routing_oracles` | Exact arrival and tie order, cancellation and stale route handling verified with Rust 1.99.1 |
| Allocation and usage | `kairo-ecs-des` tests `request_cleanup`, `usage_intervals` and feature-gated `optimistic_usage` | Request cleanup, atomic occupancy and committed rollback export verified with Rust 1.99.1 |
| Movement and allocation | `kairo-ecs-abm` test `movement_allocation` | Public Rust fixture verified with Rust 1.99.1 across arrival, cancellation, replanning, retirement and failed departure |
| Installed CareOps movement | Append the [receiving probe](careops_movement.rs) to the archived `allocation_kairos_conformance` test and run it against the extracted SDK | The original contention case and two movement cases passed. Arrival used the step-free route and released the porter at tick 10. Cancellation retained the origin and released the porter at tick 5. [Exact receipt](handoff.md#installed-careops-movement--23-november-2026) |
| Ecosystem direction | Pure game packages independent of Kairos; optional adapter imports both; HEOR and domains stay outside kernel | Accepted direction; package/consumer execution pending |
| Native scheduler checkpoint | `cargo test --locked -p kairo-ecs-core --test scheduler_checkpoint` verifies exact replay, sibling cancellation and version admission | K01.checkpoint candidate. Paignia T17.16 separately owns policy, world binding and RNG replay |
| Dependency direction | Kairos production manifest/source cannot require CareOps; minimal consumer fixture runs without the CareOps repository. | Standalone Rust case passed on both native profile targets. [Native install run](https://github.com/edithatogo/kairos/actions/runs/45051171243) |
| Consumer behaviour | Installed receivers exercise native dispatch, replay, schema and lifetime contracts with immutable provider selection. | Reuse the [accepted Track 108 evidence](../108-scientific-ecosystem-alignment/handoff.md#installed-consumer-qualification--23-november-2026). CareOps and Innovate own production integration and pin selection. |
| Feature closure | Minimal core excludes optional data/GPU/MPI/HTTP/binding stacks; optional profiles compile and function. | Pending for this track |
| Adoption | Old and new implementations match scientific/runtime contracts and measured value; licensing/security accepted. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.

