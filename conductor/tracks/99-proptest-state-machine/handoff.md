# Handoff: Track 99 — proptest-state-machine qualification

Updated: 2026-10-24. Maturity: Done (Evaluation Complete — Rejected / No-Go).

## Summary

Track 99 evaluated the `proptest-state-machine` crate (v0.9.0) as an optional dev-only extension to the proptest property testing baseline established in Track 96.
Following the [Robustness-first dependency acceptance policy](../../dependency-value-policy.md), the evaluation reached an explicit **REJECT (NO-GO)** outcome at Phase 0:
1. **Transitive bloat & Non-determinism**: `proptest-state-machine` forces on `proptest` default features (`fork`, `timeout`, `bit-set`), pulling in POSIX `nix`, background timers, `tempfile`, and `regex-syntax`. This introduces process forking and non-deterministic timeouts which violate Kairo's deterministic discrete-event simulation requirements and break portability on WASM.
2. **Negative maintainability benefit**: Plain `proptest` (v1.12.0) already supports sequential property testing via standard Rust enums and vector generators with 40% less code than `proptest-state-machine`'s macro (`prop_state_machine!`) and dual trait requirements (`ReferenceStateMachine`, `StateMachineTest`).
3. **No runtime changes**: In accordance with the value policy, no dependency was installed, and Phase 1/2 adoption tasks remain unchecked. Plain proptest from Track 96 remains the authoritative solution.

## Files changed

- `conductor/tracks/99-proptest-state-machine/metadata.json`: updated status to completed, adoption_decision to rejected.
- `conductor/tracks/99-proptest-state-machine/plan.md`: marked Phase 0 tasks complete; Phase 1 and 2 tasks retained unchecked.
- `conductor/tracks/99-proptest-state-machine/evidence/phase0-qualification.md`: comprehensive evaluation report and comparison matrix.
- `conductor/tracks/99-proptest-state-machine/review-report.md`: conductor review report and closeout disposition.
- `conductor/tracks/99-proptest-state-machine/handoff.md`: updated closeout handoff.
- `conductor/phase-closeout.yaml`: updated Track 99 ledger entry to closed.
- `conductor/tracks.yaml`: updated Track 99 status to Done.
- `conductor/tracks.md`: updated Track 99 status to Done.

## Contracts consumed

- `conductor/dependency-value-policy.md`: mandatory value gate, no-go criteria.
- `conductor/tracks/96-proptest-invariants/`: Track 96 baseline proptest receipt.
- `kairo-ecs-core`: Scheduler total ordering and event conservation invariants.

## Contracts changed

- None. No public API, ABI, schema, or runtime manifests modified.

## Tests added

- None required in runtime code. Planning and evaluation validators executed:
  - `scripts/validate_conductor_phase_gates.ps1`
  - `scripts/validate_conductor_dag.ps1`
  - `scripts/validate_conductor_artifacts.ps1`
  - `cargo test -p kairo-ecs-core` (re-verifying existing Track 96 invariant tests)

## Known risks

- None. Rejection preserves zero new dependencies and maintains 100% deterministic replay.

## Follow-up issues

- None. Evaluation track concluded.

## Integration notes

- Plain `proptest` (Track 96) remains the project standard for property testing and sequential invariant testing.

## Phase closeout evidence

- Conductor review: `$conductor-review` completed in `review-report.md`.
- Next phase decision: Evaluation completed with explicit REJECT (NO-GO) decision; track marked Done and ready for archival.
