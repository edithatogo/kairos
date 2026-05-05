# Handoff: Track 18 Comparative Benchmarks & Reproducibility

## Summary

The reproducibility surface now points at the real benchmark plan, the real
fixture manifest, and the actual smoke workflow path. The page is anchored to
the ready fixture IDs `scheduler_ordering_v1`, `scheduler_cancellation_v1`,
and `rng_reproducibility_v1`.

## Files changed

`conductor/tracks/18-comparative-benchmarks-reproducibility/plan.md`
`conductor/tracks/18-comparative-benchmarks-reproducibility/test-matrix.md`
`conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md`
`conductor/benchmarks-reproducibility.md`

## Contracts consumed

`benches/benchmark-plan.md`
`conformance/fixtures/manifest.json`
`.github/workflows/benchmark-smoke.yml`

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
`conformance/fixtures/manifest.json` versioned together.
