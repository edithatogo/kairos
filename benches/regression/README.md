# Regression Baselines

This directory holds the comparison and validation material for Track 31.

- `compare.py` validates threshold coverage against
  `benches/benchmark-smoke.json` and `conformance/fixtures/manifest.json`.
- With `--base` and `--current`, `compare.py` compares benchmark mean durations
  against `conductor/performance-thresholds.md` and fails on blocking
  regressions. Result IDs must exactly match the canonical threshold table;
  duplicate, missing, or unknown IDs fail the guard before any timing claim is
  accepted.
- Without benchmark result files, `compare.py` runs the
  `threshold-definition-exists` validator only. This keeps the CI scaffold useful
  while Track 12 owns native harness integration.
- `sample-base.json` and `sample-current.json` are small local fixtures for
  validating the comparison path without running or editing the benchmark
  harness.
- `sample-current-regression.json` and `sample-current-unknown.json` are
  negative fixtures used by the Track 31 validator to prove that blocking
  regressions and unregistered result IDs fail before any performance claim is
  accepted.

Run the offline gate from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\31-performance-regression-guard\validate-track31.ps1
```
