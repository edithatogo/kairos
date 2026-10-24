# Handoff: Track 91 — rust-fmi adapter qualification

Updated: 2026-10-25. Maturity: Done (Evaluation Complete — Rejected / No-Go).

## Summary

Track 91 evaluated the `rust-fmi` crate (`fmi = "0.8.0"`) as a candidate whole-adapter replacement for the hand-maintained FMI 2.0/3.0 function tables and dynamic loading in `crates/kairo-ecs-fmi`.
Following the [Robustness-first dependency acceptance policy](../../dependency-value-policy.md) and the stop criterion defined in `spec.md`, the evaluation reached an explicit **REJECT (NO-GO)** outcome at Phase 0:
1. **Critical Security Vulnerabilities in Transitive Dependency**: `fmi 0.8.0` pulls in `zip 8.6.0`, which is afflicted by three high-severity security vulnerabilities (`RUSTSEC-2026-0367` for SUID/SGID preservation privilege escalation, `RUSTSEC-2026-0368` for symlink path traversal, and `RUSTSEC-2026-0369` for unconfined symlink permissions).
2. **License Rejection**: Transitive dependency `zlib-rs 0.6.8` uses the `Zlib` license, which is rejected by the workspace license policy in `deny.toml`.
3. **Build Toolchain Fragility**: `fmi-sys` requires `bindgen` and `clang-sys`, imposing a hard host requirement for `libclang` across all platforms and CI runners, breaking Kairos pure-Rust cross-compilation contracts.
4. **Extreme Dependency Bloat**: Introduces 80 new packages including heavy ICU Unicode tables, duplicate `syn` versions, cryptography crates, and date-time libraries, versus the existing lightweight `libloading` approach.
5. **No Runtime Code Changed**: In accordance with the value policy, no dependency was installed, and Phase 1/2 adoption tasks remain unchecked. The existing dynamic loader in `crates/kairo-ecs-fmi` is retained.

## Files changed

- `conductor/tracks/91-fmi-robustness/metadata.json`: updated status to completed, adoption_decision to rejected, accepted_phases to ["0"].
- `conductor/tracks/91-fmi-robustness/plan.md`: marked Phase 0 tasks complete; Phase 1 and 2 tasks retained unchecked.
- `conductor/tracks/91-fmi-robustness/evidence/phase0-qualification.md`: comprehensive evaluation report and vulnerability analysis.
- `conductor/tracks/91-fmi-robustness/adr-api-compatibility-and-redteam.md`: architectural decision record and red-team security assessment.
- `conductor/tracks/91-fmi-robustness/review-report.md`: conductor review report and closeout disposition.
- `conductor/tracks/91-fmi-robustness/handoff.md`: updated closeout handoff.
- `conductor/phase-closeout.yaml`: updated Track 91 ledger entry to closed.
- `conductor/tracks.yaml`: updated Track 91 status to Done.
- `conductor/tracks.md`: updated Track 91 status to Done.

## Contracts consumed

- `conductor/dependency-value-policy.md`: mandatory value gate, no-go criteria.
- `crates/kairo-ecs-fmi`: existing FMI 2.0 / 3.0 dynamic loader and test suites.

## Contracts changed

- None. No public API, ABI, schema, or runtime manifests modified.

## Tests added

- None required in runtime code. Planning and evaluation validators executed:
  - `cargo test -p kairo-ecs-fmi --all-features` (29 tests pass)
  - `cargo deny check advisories sources licenses bans` (clean)
  - `cargo audit` (clean)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`
  - `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`

## Known risks

- None. Rejection preserves zero new dependencies and protects the workspace from active CVEs.

## Follow-up issues

- Advance to Track 79 (`zip FMU archive replacement`) and Track 80 (`quick-xml FMU metadata replacement`) to qualify bounded, safe, pure-Rust archive extraction and XML parsing.

## Integration notes

Track 91 is an evaluation track closed at Phase 0 with a NO-GO decision.

## Phase closeout evidence

- Conductor review: Passed (`$conductor-review` in `review-report.md`).
- Accepted fixes: No outstanding findings; 3 findings documented and resolved in review report.
- Validation commands:
  - `cargo test -p kairo-ecs-fmi --all-features` (29/29 tests pass)
  - `cargo deny check advisories sources licenses bans` (clean)
  - `cargo audit` (clean)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` (pass)
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` (pass)
  - `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` (pass)
- Closeout ledger: commit SHA `25f07a6a76726b58f30d921bb5a6edd06709987c` recorded upon evaluation closeout.
- Pushed ref: pushed ref `origin/feat/track-91-rust-fmi-qualification`.
- Next-phase decision: Track 91 is Done (Rejected); advance to Track 79 (`zip FMU archive replacement`).
