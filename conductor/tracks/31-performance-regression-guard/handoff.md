# Handoff: Track 31 Performance Regression Guard

Last updated: 2026-05-07

## Summary

Defined the performance regression detection framework for KairoECS. Worker 2
replaced preview-only thresholds with canonical scenario thresholds tied to
`benches/benchmark-smoke.json` and `conformance/fixtures/manifest.json`, added a
metadata-aware local validator/comparator, and expanded the CI workflow scaffold
so PRs touching benchmark-sensitive paths validate threshold coverage before the
native benchmark outputs are promoted.

## Files changed

`conductor/tracks/31-performance-regression-guard/test-matrix.md`,
`conductor/tracks/31-performance-regression-guard/risk-register.md`,
`conductor/tracks/31-performance-regression-guard/handoff.md`,
`conductor/performance-thresholds.md`, `benches/regression/README.md`,
`benches/regression/compare.py`, `benches/regression/sample-base.json`,
`benches/regression/sample-current.json`,
`benches/regression/sample-current-regression.json`,
`benches/regression/sample-current-unknown.json`,
`conductor/tracks/31-performance-regression-guard/validate-track31.ps1`,
`.github/workflows/bench-regression.yml`

## Contracts consumed

- `benches/` and `crates/kairo-ecs-bench/` — benchmark harnesses from Track 12 (read-only).
- `docs/benchmarks/` — comparative benchmark metadata from Track 18 (read-only).
- Existing CI workflow patterns from Track 13 (read-only reference).

## Release gates affected

- **benchmark-regression-check**: Blocks PR merge if any benchmark exceeds its threshold. Not release-gating.
- **threshold-definition-exists**: Blocks PR merge if any active benchmark lacks a threshold entry. Not release-gating.
- Both gates are in `conductor/quality-gates.md`. This track is explicitly non-critical for release — a documented and accepted regression does not block release.
- Current Alpha behavior is threshold/comparator scaffold: `.github/workflows/bench-regression.yml`
  validates threshold coverage and compiles benchmark targets. It does not yet
  run a native base-branch versus PR-branch benchmark pair until Track 12 emits
  stable benchmark artifacts with runner metadata.
- Energy-efficiency reporting is not yet part of the blocking regression gate;
  the current track covers timing thresholds and comparator behavior only.

## Worker 2 validation

- `python benches/regression/compare.py --report .tmp/track31-threshold-report.json`
  validates threshold coverage, owner alignment, and measure alignment against
  the canonical metadata. Result: pass.
- `python benches/regression/compare.py --base benches/regression/sample-base.json --current benches/regression/sample-current.json --report .tmp/track31-compare-report.json`
  exercises the regression comparison path with owned local JSON fixtures.
  Result: pass.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\31-performance-regression-guard\validate-track31.ps1`
  exercises both positive and negative gate fixtures. Result: pass. The negative
  checks prove that a missing canonical threshold, an unknown benchmark result
  ID, and a blocking `schedule_1m_events` regression all fail with report
  details before any performance claim is accepted.
- `python benches/benchmark_smoke.py` verifies the consumed Track 12 metadata
  remains valid. Result: pass.
- `python -m py_compile benches/regression/compare.py` verifies Python syntax.
  Result: pass.
- `cargo bench --workspace --no-run` was attempted to mirror the workflow
  compile step. Result: fail in this Windows shell because Cargo invoked
  `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`, which failed
  with Win32 error 5 while creating mappings/pipes. This is a local linker
  environment blocker, not a Track 31 script failure.

## Risks and unresolved questions

- CI runner heterogeneity is the primary risk to baseline stability. Controlled-runner pinning may not be feasible on GitHub Actions free tier — consider self-hosted runners or nightly full-suite runs with PR-only sampling.
- Threshold tuning is an ongoing activity. Initial values (3-10% depending on category) are estimates and should be reviewed quarterly against accumulated benchmark history.
- Benchmarks added by Track 12 or Track 18 after this track ships must include threshold entries; the `threshold-definition-exists` gate enforces this at the PR level through the metadata validator.
- Native benchmark result artifact shape is still a Track 12 handoff item. Until
  it lands, the workflow validates metadata and threshold coverage and compiles
  benchmark targets with `cargo bench --workspace --no-run`.
- Local Windows benchmark compilation currently needs the MSVC linker path
  resolved before `cargo bench --workspace --no-run` can be used as a local
  pass/fail signal.

## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
