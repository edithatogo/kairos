# Handoff: Track 18 Comparative Benchmarks & Reproducibility

## Summary

The reproducibility surface now points at the real benchmark plan, the real
fixture manifest, and the actual smoke workflow path. The page is anchored to
the ready fixture IDs `scheduler_ordering_v1`, `scheduler_cancellation_v1`,
and `rng_reproducibility_v1`. Track 18 now has a public reproduction page and
a lightweight metadata validator that can be run without native benchmark
link tests.

## Files changed

`conductor/tracks/18-comparative-benchmarks-reproducibility/test-matrix.md`
`conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md`
`conductor/tracks/18-comparative-benchmarks-reproducibility/risk-register.md`
`docs/benchmarks/README.md`
`benches/README.md`
`benches/benchmark_reproducibility.py`
`docs/benchmarks/benchmark-policy.md`
`docs/benchmarks/reproduce-comparison.md`
`website/docs-link-manifest.json`
`website/src/index.md`

## Contracts consumed

`benches/benchmark-plan.md`
`conformance/fixtures/manifest.json`
`.github/workflows/benchmark-smoke.yml`

## Evidence commands

`python benches/benchmark_smoke.py`

Validates the smoke metadata against `conformance/fixtures/manifest.json` for
canonical scenario names, owners, canonical status, and smoke scales.

`python benches/benchmark_reproducibility.py`

Validates the Track 18 evidence boundary: ready fixture IDs, expected fixture
source files, fixture assertions, canonical benchmark scenarios, smoke scales,
and required docs artifacts.

`npm --prefix website run check:links`

Validates that the new reproduction page is reachable through the docs link
manifest and that local Markdown links remain valid.

`powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/18-comparative-benchmarks-reproducibility/validate-benchmark-reproducibility.ps1`

Validates the Track 18 docs and manifest boundary without running native
benchmark measurements: ready fixture IDs, canonical benchmark IDs,
metadata-gate caveat, and the benchmark smoke workflow reference.

## Local validation on 2026-05-06

| Command | Result | Evidence |
|---|---|---|
| `python benches/benchmark_smoke.py` | pass | Reported `status: ok` for six canonical scenarios and `requires_native_link_tests: false`. |
| `python benches/benchmark_reproducibility.py` | pass | Reported `status: ok` for ready fixtures `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1` and six canonical scenarios. |
| `npm --prefix website run check:links` | pass | Checked 20 required paths and 2 markdown sources. |
| `cargo bench --workspace --no-run` | blocked | Windows resolved `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`; linker exited with `0xc0000142` and Win32 error 5 while creating mappings/pipes. |

## Evidence boundary

The track treats committed benchmark-plan text, the fixture manifest, and the
benchmark smoke workflow as the sources of truth. A reproducibility claim must
name at least one real fixture ID or benchmark target and say what is being
compared.

## Release gates affected

Benchmark smoke checks and fixture determinism are the accepted gate inputs
for this track. The page does not invent a new harness; it explains how the
existing smoke workflow and fixture manifest support a reproducible claim.

## Risks and unresolved questions

The concrete risk is benchmark drift if fixture IDs, seed notes, or comparison
baselines change after publication. Keep `benches/benchmark-plan.md` and
`conformance/fixtures/manifest.json` versioned together. Also keep
`benches/benchmark-smoke.json` aligned with those contracts. Native performance
claims remain blocked until raw benchmark output, command capture, host
metadata, baseline versions are archived, and the Windows linker path is using
the expected MSVC build-tools linker rather than Git's Unix `link.exe`.

## Review-hardening update

Added a track-local offline validator so the Track 18 evidence boundary can be
checked even when native benchmark linking is unavailable.

## Contracts changed

No benchmark contracts changed in this scoped cleanup. The evidence boundary still depends on `benches/benchmark-plan.md`, `conformance/fixtures/manifest.json`, and `.github/workflows/benchmark-smoke.yml`.

## Tests added

No executable benchmark tests were added in this scoped cleanup. The track-local validator now also checks the public benchmark landing page.

## Known risks

Native performance claims remain blocked until raw benchmark output, command capture, host metadata, and baseline versions are archived with the expected MSVC linker path.

## Follow-up issues

Keep `benches/benchmark-plan.md`, `benches/benchmark-smoke.json`, and `conformance/fixtures/manifest.json` versioned together when benchmark IDs or fixture IDs change.

## Integration notes

Treat the current benchmark evidence as reproducibility and metadata-gate evidence, not as a native performance comparison.
