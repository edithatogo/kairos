# Handoff: Track 89 — Tract inference qualification

Updated: 2026-10-25. Maturity: Done (Evaluation Complete — Rejected / No-Go).

## Summary

Track 89 evaluated the `tract` crate (v0.23.9 / `tract-onnx`) as a candidate replacement for machine learning model execution and inference scaffolding in `crates/kairo-ecs-ml/`.
Following the [Robustness-first dependency acceptance policy](../../dependency-value-policy.md) and [spec.md](spec.md), the evaluation reached an explicit **REJECT (NO-GO)** outcome at Phase 0:
1. **Stop Criterion Triggered**: Track 89 [spec.md](./spec.md) mandates: *"No adoption without a required model or if supported operators/targets fail. Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity."* Neither Kairos nor CareOps-Sim currently has an active ONNX model or production ML inference requirement.
2. **Dependency Bloat Without Justified Consumer**: Adding `tract-onnx` pulls in over 50 transitive crates, increasing compile time, binary footprint, and supply-chain review surface without an active production workload to justify it.
3. **Safety & Architecture Invariant Preservation**: `crates/kairo-ecs-ml` maintains `#![forbid(unsafe_code)]` and provides complete tensor validation, shape verification, tick hooks, and robust fallback policies (`HoldLastOutput`, `UseOriginalSystem`, `FailTick`).
4. **Existing Production Scaffold**: Dependency-free `OrtSession`, `NeuralSystem`, `InferenceTickHook`, and multi-world parallel vector environment rollouts pass all 17 unit and integration tests.
5. **No Runtime Changes**: Per the value policy, no dependency was installed, and Phase 1/2 adoption tasks remain unchecked. Native `kairo-ecs-ml` remains authoritative.

## Files changed

- `conductor/tracks/89-tract-robustness/metadata.json`: updated status to completed, adoption_decision to rejected.
- `conductor/tracks/89-tract-robustness/plan.md`: marked Phase 0 tasks complete; Phase 1 and 2 tasks retained unchecked.
- `conductor/tracks/89-tract-robustness/evidence/phase0-qualification.md`: comprehensive evaluation report and comparison matrix.
- `conductor/tracks/89-tract-robustness/adr-tract-inference-evaluation.md`: architectural decision record for rejection.
- `conductor/tracks/89-tract-robustness/evidence/cargo-deny.txt`: license and advisory scan.
- `conductor/tracks/89-tract-robustness/evidence/cargo-audit.txt`: security audit scan.
- `conductor/tracks/89-tract-robustness/evidence/test-results.txt`: ML crate test suite output.
- `conductor/tracks/89-tract-robustness/review-report.md`: conductor review report and closeout disposition.
- `conductor/tracks/89-tract-robustness/handoff.md`: updated closeout handoff.
- `conductor/phase-closeout.yaml`: updated Track 89 ledger entry.
- `conductor/tracks.yaml`: updated Track 89 status to Done.
- `conductor/tracks.md`: updated Track 89 status to Done.

## Contracts consumed

- `conductor/dependency-value-policy.md`: mandatory value gate, no-go criteria.
- `conductor/tracks/89-tract-robustness/spec.md`: stop condition on absent required model.
- `kairo-ecs-ml`: `#![forbid(unsafe_code)]`, `Tensor` validation, `InferenceTickHook`, `FallbackPolicy`.

## Contracts changed

- None. No public API, ABI, schema, or runtime manifests modified.

## Tests added

- None required in runtime code. Planning and evaluation validators executed:
  - `scripts/validate_conductor_phase_gates.ps1`
  - `scripts/validate_conductor_dag.ps1`
  - `scripts/validate_conductor_artifacts.ps1`
  - `cargo test -p kairo-ecs-ml`

## Known risks

- None. Rejection preserves zero external dependencies, `#![forbid(unsafe_code)]`, and minimal compile times.

## Follow-up issues

- None. Evaluation track concluded.

## Integration notes

- Dependency-free `kairo-ecs-ml` remains the project standard for neural system tick integration and fallback policies.

## Phase closeout evidence

- Conductor review: `$conductor-review` completed in `review-report.md`.
- Next phase decision: Evaluation completed with explicit REJECT (NO-GO) decision; track marked Done and ready for archival.

### Reproducible validation provenance (AGENTS.md L58)

- **Tested Commit:** `0a8bd814f56d561d9ded4ce274c7c708277fc481`
- **Working Directory:** `/Volumes/PortableSSD/github/kairos`
- **Toolchain:** `rustc 1.99.1 (target: aarch64-apple-darwin), cargo 1.99.1`
- **Command Receipts:**
  - `cargo test -p kairo-ecs-ml` -> Exit Status: `0` (17 tests passed, 0 failed) -> [evidence/test-results.txt](file:///Volumes/PortableSSD/github/kairos/conductor/tracks/89-tract-robustness/evidence/test-results.txt)
  - `cargo deny check advisories sources licenses bans` -> Exit Status: `0` (clean) -> [evidence/cargo-deny.txt](file:///Volumes/PortableSSD/github/kairos/conductor/tracks/89-tract-robustness/evidence/cargo-deny.txt)
  - `cargo audit` -> Exit Status: `0` (0 vulnerabilities) -> [evidence/cargo-audit.txt](file:///Volumes/PortableSSD/github/kairos/conductor/tracks/89-tract-robustness/evidence/cargo-audit.txt)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 -TrackDir conductor/tracks/89-tract-robustness` -> Exit Status: `0`
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` -> Exit Status: `0`
  - `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` -> Exit Status: `0`
  - `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` -> Exit Status: `0`
