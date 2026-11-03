# Handoff

Updated: 2026-10-22. Maturity: planned; adoption decision pending.

## Summary

Separate dependency track registered. No runtime change, installation or accepted implementation phase.

## Files changed

Planning folder and coordinated registry/policy surfaces only.

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

Prospective replacement boundaries only; no runtime API changed.

## Tests added

Independent future tests specified in test-matrix.md; planning validators only this pass.

## Known risks

Do not adopt a graphics-oriented sequence if precision/dimension requirements fail; record a bounded alternative search, never silently lower statistical requirements.

## Follow-up issues

Record exact existing consumer, baseline and adopt/no-go decision in Phase 0. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Pending: `$conductor-review`, accepted fixes, rejected findings, command/cwd/commit SHA/toolchain/seed/input hashes, exit codes/artifacts, pushed ref or blocker and next-phase decision. Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.

## Numerical qualification — 1 November 2026

**Reject sobol_burley 0.5.0 as a drop-in replacement.** Its documented maximum is
65,536 points in 256 dimensions. Its scalar writer uses 23 fractional bits in
an f32, and seed zero is still scrambled. The current Kairos public generator
returns f64 on a 32-bit fractional grid. Its count is u64, but the 32 direction
columns limit nonempty sequences to 2^32 points. The last first-coordinate value
at index 2^32−1 was exactly 2^-32, below the candidate's smallest positive
grid step. Widening f32 or padding dimensions with new seeds cannot preserve
precision, dimensions or the existing seeded/replay contract.

**The existing generator fails the conventional 2-D reference.** At zero-based
index 1 it yields (0.5, 0) instead of (0.5, 0.5). Its first coordinate agrees
with all 4,096 unscrambled SciPy reference points. At index 2^32 it panics
on an out-of-bounds direction column. These findings are independent of the
decision on the candidate. Keep existing results identifiable as the legacy
stream; a corrected public stream needs versioned provenance, checked bounds,
migration fixtures and the experiment owner's acceptance. Do not certify the
legacy sequence for standard multidimensional QMC.

Prerequisite Track 72 merged in PR #316 at 68b5543. Source inventory found
the named generator in ensemble.rs and a smoke test that previously checked
only output length. The sensitivity analyzer builds its A/B matrices with
Latin hypercube samplers seeded 42 and 1337. Its native Rayon bit-invariance
test passed separately, so this decision neither fixes nor changes an estimator.

The [numerical receipt](qualification-results.json) covers 16, 256 and 4,096
points for the analytic integral of x*y on the unit square (exact 1/4).
All three sets occupied every cell in a 4x4 grid at 16 points, illustrating
why a coarse coverage test alone misses the legacy error. At 4,096 points
the absolute errors were 0.00018305 (legacy), 0.00011823 (unscrambled reference)
and 0.000003815 (candidate, seed 92). This one integrand and seed cannot
establish a general ranking. Candidate seeds 92 and 93 gave different samples;
repeated seed 92 was bit-identical. The last supported sample/dimension was
exercised, along with invalid dimension 256 and debug-build index 65,536.
The candidate's out-of-range index is unsupported in release builds as well;
the probe only asserts a panic in debug.

Reproduce with repo-local CPython 3.15.2, NumPy 2.5.4, SciPy 1.18.1 and Rust
1.99.1 on Darwin arm64. Run `.venv/bin/python -B conductor/tracks/92-sobol-burley-robustness/qualify.py <empty-directory>`.
The driver compiles the original generator extracted at immutable Kairos
41e58e0c012e3f2253559cbefb239db71ee7486c alongside the locked, no-default-feature
candidate and compares the results with scipy.stats.qmc.Sobol. Expected panic
messages from caught negative controls go to stderr. Review added the exact
final-grid-point assertion, disabled-assertion refusal, empty-output-directory
check and licence metadata for the isolated probe. The rerun reproduced the
numerical receipt exactly.

The candidate was the latest stable crate at readback, MIT OR Apache-2.0 with
separate Joe–Kuo direction-number attribution. The archive declared no rust-version.
Its source repository was unarchived; last observed push 20 August 2026.
The isolated candidate passed repository cargo-deny advisories, bans, licences
and sources. Other targets, SIMD, historical MSRV and performance budgets were
not qualified after the stop criterion was reached.

**Bounded alternative search.** sobol 1.0.2 (BSD-3-Clause) with
`Sobol::<f64>::new(2, &JoeKuoD6::minimal())` matched all 4,096 independent
reference rows exactly. The repository licence gate rejected Zlib for
adler32 1.2.0 through libflate 1.4.0. No exception was added. Scrambling,
higher dimensions, independent direction-number provenance, replay migration
and the full compatibility matrix remain open for any successor.

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns the track directory in
/Users/doughnut/.codex/worktrees/track48-rollback/kairos on codex/track92-sobol-qualification.
Native scope is isolated local numerical and policy qualification and read-only
consumer/PR inspection. The existing CareOps handoff is unresolved/pending;
no consumer pin or production dependency was promoted. The integrator retains
registry/phase acceptance. Next operation is protected integration.

Raw experiment directory: /Volumes/PortableSSD/codex-builds/kairos-track92-eSoVwm. SHA-256 receipts:

- `boundary.json` — 30fc637cfa843cba8f59b4f3679e8fab1740c7960ef4201e62c6f0d05305cda4
- `boundary/points.csv` — 5a5367e06564d5a2f6d66a61124591112771c5e962a9a1f1ac6967316a2f5722
- `candidate.tgz` — 09f37cae1d97c4078377153ede7a26f7813b689ad5c6b76ff45dc52e53afe1d1
- `deny-final.log` — 9ae088edb72e5d3c077ee4d4ef1b3e4d7a8d716b3dda70904610546b4e34f8b6
- `alt.csv` — 014a2171aac721fea0b27b3db49f62257da24edc3536ba037f618b7fc0eedf4f
- `alt-deny.log` — d1a110ba0b2752f074d2ada75dc878b5f68aedcd83d5573f9e77ee5ae983f845
- `estimator.log` — 3176c39cef86a231aefd770e8f4577a5b91d2efd0ba30be4423f42a1c155631e

## Current readback — 3 November 2026

Chat `01a19bed-7060-7910-af22-38cb31f51446` owns this bounded handoff readback in `/Users/doughnut/.codex/worktrees/kairos-track98-review/kairos`, branch `codex/conditional-tracks-reconcile-20261103`, based on `d7bdcd3`. Shared registry and runtime changes remain with their owners.

The earlier instruction to integrate the evidence is historical: [PR #356](https://github.com/edithatogo/kairos/pull/356) merged as `b5763f3c12c8b1fc8bbf0d1c25ec19c1d0ab6282`. This integrates the recorded no-go evaluation, not acceptance of a production replacement. Numerical, consumer and phase-review gates remain open.
