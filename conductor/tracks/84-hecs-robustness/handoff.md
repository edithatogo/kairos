# Handoff: Track 84 — hecs ECS storage qualification

Updated: 2026-10-25. Maturity: Done (Evaluation Complete — Rejected / No-Go).

## Summary

Track 84 evaluated the `hecs` crate (v0.11.2) as a candidate replacement for custom sparse component storage, query planning, and entity management in `crates/kairo-ecs-state/`.
Following the [Robustness-first dependency acceptance policy](../../dependency-value-policy.md) and [spec.md](spec.md), the evaluation reached an explicit **REJECT (NO-GO)** outcome at Phase 0:
1. **Safety Violation**: `kairo-ecs-state` enforces `#![forbid(unsafe_code)]`, whereas `hecs` depends critically on internal `unsafe` pointers and memory operations.
2. **Generational ID Truncation**: `hecs::Entity` packs a 32-bit ID and 32-bit generation into 64 bits, violating Kairo's requirement for 64-bit index (`index: u64`) and breaking multi-million agent ABM capacity and FFI/Diplomat contracts.
3. **Replay Non-Determinism**: `hecs` shuffles entity positions via archetype `swap_remove`, destroying the deterministic `(index, generation)` query iteration invariant essential for bit-for-bit replay.
4. **Hardware Alignment Loss**: `hecs` lacks cache-line alignment (`CacheLineLayout`) for SIMD kernels and unit-stride Arrow layout (`TightLayout`).
5. **No Runtime Changes**: Per the value policy, no dependency was installed, and Phase 1/2 adoption tasks remain unchecked. Native `kairo-ecs-state` remains the authoritative ECS engine.

## Files changed

- `conductor/tracks/84-hecs-robustness/metadata.json`: updated status to completed, adoption_decision to rejected.
- `conductor/tracks/84-hecs-robustness/plan.md`: marked Phase 0 tasks complete; Phase 1 and 2 tasks retained unchecked.
- `conductor/tracks/84-hecs-robustness/evidence/phase0-qualification.md`: comprehensive evaluation report and comparison matrix.
- `conductor/tracks/84-hecs-robustness/review-report.md`: conductor review report and closeout disposition.
- `conductor/tracks/84-hecs-robustness/handoff.md`: updated closeout handoff.
- `conductor/phase-closeout.yaml`: updated Track 84 ledger entry to closed.
- `conductor/tracks.yaml`: updated Track 84 status to Done.
- `conductor/tracks.md`: updated Track 84 status to Done.

## Contracts consumed

- `conductor/dependency-value-policy.md`: mandatory value gate, no-go criteria.
- `conductor/tracks/84-hecs-robustness/spec.md`: stop condition on net complexity and replay survival.
- `kairo-ecs-state`: deterministic query order, `CacheLineLayout`, `#![forbid(unsafe_code)]`.

## Contracts changed

- None. No public API, ABI, schema, or runtime manifests modified.

## Tests added

- None required in runtime code. Planning and evaluation validators executed:
  - `scripts/validate_conductor_phase_gates.ps1`
  - `scripts/validate_conductor_dag.ps1`
  - `scripts/validate_conductor_artifacts.ps1`
  - `cargo test -p kairo-ecs-state` (re-verifying existing Track 64 tests)

## Known risks

- None. Rejection preserves zero external dependencies, `#![forbid(unsafe_code)]`, and bit-for-bit replay determinism.

## Follow-up issues

- None. Evaluation track concluded.

## Integration notes

- Native `kairo-ecs-state` remains the project standard for generational entity storage and cache-aligned component memory.

## Phase closeout evidence

- Conductor review: `$conductor-review` completed in `review-report.md`.
- Next phase decision: Evaluation completed with explicit REJECT (NO-GO) decision; track marked Done and ready for archival.
