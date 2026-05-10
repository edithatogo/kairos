# Test Matrix: Track 18 Comparative Benchmarks & Reproducibility

| Check | Validation command | Required by alpha | Required by beta | Required by 1.0 |
|---|---|---:|---:|---:|
| Track-local benchmark reproducibility validator passes | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/18-comparative-benchmarks-reproducibility/validate-benchmark-reproducibility.ps1` | yes | yes | yes |
| Benchmark landing page exists | `Test-Path docs/benchmarks/README.md` | yes | yes | yes |
| Benchmark plan exists | `Test-Path benches/benchmark-plan.md` | yes | yes | yes |
| Fixture manifest exists | `Test-Path conformance/fixtures/manifest.json` | yes | yes | yes |
| Ready fixture IDs are named | `rg -n "scheduler_ordering_v1|scheduler_cancellation_v1|rng_reproducibility_v1" conformance/fixtures/manifest.json conductor/tracks/18-comparative-benchmarks-reproducibility/plan.md conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md` | yes | yes | yes |
| Measurement inputs are explicit | `rg -n "seed|fixture|baseline|comparison|repeatable" benches/benchmark-plan.md conductor/tracks/18-comparative-benchmarks-reproducibility/plan.md` | yes | yes | yes |
| Smoke workflow matches the real contract | `rg -n "Benchmark smoke|test -f Cargo.toml|test -d benches|python benches/benchmark_smoke.py|cargo check -p kairo-ecs-bench" .github/workflows/benchmark-smoke.yml` | yes | yes | yes |
| Artifact existence check | `Test-Path docs/benchmarks/README.md; Test-Path conformance/fixtures/manifest.json; Test-Path benches/benchmark-plan.md; Test-Path .github/workflows/benchmark-smoke.yml` | yes | yes | yes |
| Comparison criteria are explicit | `rg -n "comparison|baseline|fixture|seed|host" conductor/tracks/18-comparative-benchmarks-reproducibility/plan.md conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md` | no | yes | yes |
| Reproducibility claim is tied to a real fixture or benchmark target | `rg -n "scheduler_ordering_v1|scheduler_cancellation_v1|rng_reproducibility_v1|benchmark" benches/benchmark-plan.md conductor/tracks/18-comparative-benchmarks-reproducibility/plan.md conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md` | yes | yes | yes |
| Red-team objections about host variance are answered | `rg -n "host variance|unstable host|seed|determin" conductor/tracks/18-comparative-benchmarks-reproducibility/plan.md conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md` | yes | yes | yes |
| Benchmark smoke metadata validates against manifest | `python benches/benchmark_smoke.py` | yes | yes | yes |
| Track 18 reproducibility evidence validates against manifest | `python benches/benchmark_reproducibility.py` | yes | yes | yes |
| Benchmark metadata gate is machine-checked | `python benches/benchmark_reproducibility.py` | yes | yes | yes |
| Raw-results policy gate is machine-checked | `rg -n "raw-results-policy|raw_output_path|baseline_version|public-performance-claim" benches/raw-results-policy.json docs/benchmarks/benchmark-policy.md docs/benchmarks/reproduce-comparison.md` | yes | yes | yes |
| Benchmark docs include maturity and expected output | `rg -n "Maturity: preview metadata gate\|Expected output\|\"status\": \"ok\"" docs/benchmarks/README.md docs/benchmarks/reproduce-comparison.md` | yes | yes | yes |
| Public reproduction page is linked into docs manifest | `npm --prefix website run check:links` | yes | yes | yes |
| Aggregate Track 12-20 evidence gate keeps benchmark evidence wired | `node tests/conformance/track12_20_evidence_check.mjs` | yes | yes | yes |

## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
