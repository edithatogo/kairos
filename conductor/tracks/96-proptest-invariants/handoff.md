# Handoff: Track 96 — proptest robustness

Updated: 2026-10-24. Maturity: Done.

## Summary

Track 96 qualified the proptest property testing framework (v1.12.0) across `kairo-ecs-core` and verified core simulation invariants:
1. Total ordering contract: strict lexicographical dispatch order `(SimTime, priority, insertion_sequence)`.
2. Global and step-wise event/resource conservation: `scheduled = dispatched + cancelled + pending`.
3. Idempotent cancellation and zero-dispatch for cancelled events.
4. Generational ID coverage across boundary ranges (64-bit indices and 32-bit generations).
5. Reproducible fixed-seed replay (`RngSeed::Fixed`) and bounded minimal counterexample shrinking.
6. Negative controls demonstrating that mutation and broken ordering/conservation are detected by the property test runner.

## Files changed

- `conductor/tracks.yaml`: mark Track 96 In Progress -> Done.
- `conductor/tracks.md`: mark Track 96 In Progress -> Done.
- `conductor/tracks/96-proptest-invariants/plan.md`: complete all phase tasks.
- `conductor/tracks/96-proptest-invariants/handoff.md`: document implementation and closeout evidence.
- `conductor/phase-closeout.yaml`: record Track 96 closeout ledger entry.
- `crates/kairo-ecs-core/tests/track96_phase0_proptest_baseline.rs`: seed replay, shrinking, and negative controls.
- `crates/kairo-ecs-core/tests/track96_phase1_proptest_invariants.rs`: total ordering, event conservation, cancellation, and generational ID range properties.

## Contracts consumed

- `kairo_ecs_types`: `SimTime`, `Priority`, `EventId`, `EntityId`, `EventKind`, `ScheduleRequest`, `StepOutcome`.
- `kairo_ecs_core`: `Scheduler` total ordering, step loop, stats, and cancellation.
- `proptest`: `1.12.0` (`std` feature, zero default features) with reproducible seeds and bounded test budgets.

## Contracts changed

- None. No public API, ABI, or schema contracts changed. Dev-dependency and property test harness expansion only.

## Tests added

- `crates/kairo-ecs-core/tests/track96_phase0_proptest_baseline.rs`:
  - `reproducible_seed_replay_guarantee`: identical case generation across runs.
  - `bounded_shrinking_finds_minimal_counterexample`: shrinks failing value to exact threshold.
  - `negative_control_detects_broken_conservation`: catches simulated lost event.
  - `negative_control_detects_priority_inversion`: catches simulated priority inversion.
- `crates/kairo-ecs-core/tests/track96_phase1_proptest_invariants.rs`:
  - `proptest_total_ordering_and_conservation_invariant`: BTreeSet reference oracle and step-wise conservation.
  - `proptest_cancellation_and_conservation_invariant`: idempotent cancellation and zero-dispatch for cancelled events.
  - `proptest_generational_entity_and_event_id_ranges`: boundary value testing for EntityId and EventId.

## Known risks

- Property tests run with fixed seeds and bounded budgets (128 cases, 1,000 max shrink iters) to ensure determinism and prevent test timeouts.

## Phase closeout evidence

- Commands run:
  - `cargo test -p kairo-ecs-core` (51 passed, 0 failed)
  - `cargo clippy -p kairo-ecs-core --all-targets -- -D warnings` (passed, 0 warnings)
  - `cargo fmt --check -p kairo-ecs-core` (passed)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`
  - `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`
