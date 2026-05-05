# Handoff — 12 Conformance, Testing & Benchmarks

## Summary

Documented the scheduler, cancellation, and RNG fixture IDs plus benchmark scenario names so downstream tracks can validate against stable manifests.

## Files changed

`conductor/tracks/12-conformance-testing-benchmarks/spec.md`
`conductor/tracks/12-conformance-testing-benchmarks/plan.md`
`conductor/tracks/12-conformance-testing-benchmarks/test-matrix.md`
`conductor/tracks/12-conformance-testing-benchmarks/agent-contract.md`
`conductor/tracks/12-conformance-testing-benchmarks/risk-register.md`
`conductor/tracks/12-conformance-testing-benchmarks/handoff.md`
`conformance/README.md`
`conformance/fixtures/README.md`
`conformance/fixtures/manifest.json`
`tests/conformance/README.md`
`benches/README.md`
`benches/benchmark-plan.md`

## Contracts consumed

`conductor/contracts/conformance-contract.md`
`conductor/contracts/arrow-schema-contract.md`
`conductor/workflow.md`

## Contracts changed

None.

## Tests added

Manifest validation and benchmark-name checks are defined in `test-matrix.md`.

## Known risks

Fixture and benchmark names must stay stable once Track 01 and the binding tracks start consuming them.

## Integration notes

Track 01 consumes the scheduler and RNG fixtures.
Track 02 consumes the FFI lifecycle fixture once the facade contract is ready.
Tracks 06-11 should use the manifest instead of re-stating fixture semantics locally.
