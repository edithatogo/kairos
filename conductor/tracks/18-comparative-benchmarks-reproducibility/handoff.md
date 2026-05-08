# Handoff: Track 18 Comparative Benchmarks & Reproducibility

## Summary

The reproducibility surface now points at the real benchmark plan, the real
fixture manifest, and the actual smoke workflow path. The page is anchored to
the ready fixture IDs `scheduler_ordering_v1`, `scheduler_cancellation_v1`,
and `rng_reproducibility_v1`. Track 18 now has a public reproduction page and
a lightweight metadata validator that can be run without native benchmark
link tests. The raw-results policy gate is now machine-checkable through
`benches/raw-results-policy.json` and the Track 18 validators.

## Files changed

`conductor/tracks/18-comparative-benchmarks-reproducibility/test-matrix.md`
`conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md`
`conductor/tracks/18-comparative-benchmarks-reproducibility/risk-register.md`
`docs/benchmarks/README.md`
`benches/README.md`
`benches/benchmark_reproducibility.py`
`benches/raw-results-policy.json`
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
required docs artifacts, benchmark metadata, and raw-results policy fields.

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

## Review update on 2026-05-08

`$conductor-review` found one in-scope gate gap before closeout: the
`benchmark-metadata` and `raw-results-policy` gates were named in the registry
but the Track 18 validator did not enforce the raw-results policy. The accepted
fix added `benches/raw-results-policy.json`, wired it into
`benches/benchmark_reproducibility.py`, and documented it from the public
benchmark pages. No Track 12 benchmark harness changes were made.

## Local validation on 2026-05-08

| Command | Result | Evidence |
|---|---|---|
| `python benches/benchmark_smoke.py` | pass | Reported `status: ok` for six canonical scenarios and `requires_native_link_tests: false`. |
| `python benches/benchmark_reproducibility.py` | pass after wording fix | Reported `status: ok` for three ready fixtures, six canonical scenarios, and `benches/raw-results-policy.json`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/18-comparative-benchmarks-reproducibility/validate-benchmark-reproducibility.ps1` | pass | Reported `track18_status=ok`, five ready fixture IDs, and six canonical benchmarks. |
| `rg -n "raw-results-policy\|raw_output_path\|baseline_version\|public-performance-claim" benches/raw-results-policy.json docs/benchmarks/benchmark-policy.md docs/benchmarks/reproduce-comparison.md` | pass | Found policy references in docs plus required fields in the policy manifest. |
| `node tests/conformance/track12_20_evidence_check.mjs` | pass | Reported `status: ok` for Tracks 12-20 evidence inventory. |
| `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | blocked outside Track 18 | Failed on pre-existing closed ledger entries for Tracks 06, 07, 08, 09, 11, 14, 16, 19, 20, 26, 28, and 29 missing 40-character commit SHAs. |

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
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
