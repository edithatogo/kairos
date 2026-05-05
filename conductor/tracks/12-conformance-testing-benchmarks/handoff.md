# Handoff — 12 Conformance, Testing & Benchmarks

## Summary

Documented the scheduler, cancellation, RNG, and VVUQ fixture IDs plus benchmark scenario names so downstream tracks can validate against stable manifests. Added a reusable bootstrap conformance runner, Track 07-13 hardening validator, and metadata-only benchmark smoke harness that do not require native binding link tests.

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
`tests/conformance/runner.mjs`
`tests/conformance/conformance-check.mjs`
`tests/conformance/track07_13_hardening_check.mjs`
`benches/README.md`
`benches/benchmark-plan.md`
`benches/benchmark-smoke.json`
`benches/benchmark_smoke.py`
`crates/kairo-ecs-bench/src/lib.rs`

## Contracts consumed

`conductor/contracts/conformance-contract.md`
`conductor/contracts/arrow-schema-contract.md`
`conductor/workflow.md`

## Contracts changed

None.

## Tests added

Manifest validation and benchmark-name checks are defined in `test-matrix.md`.

Current local checks:

```text
node tests/conformance/conformance-check.mjs
node tests/conformance/track07_13_hardening_check.mjs
python benches/benchmark_smoke.py
cargo check -p kairo-ecs-bench
```

## Known risks

Fixture and benchmark names must stay stable once Track 01 and the binding tracks start consuming them.
Chaos and OSS-Fuzz language in the plan/spec remains future scope until checked-in harnesses exist.

## Integration notes

Track 01 consumes the scheduler and RNG fixtures.
Track 02 consumes the FFI lifecycle fixture once the facade contract is ready.
Tracks 06-11 should use the manifest instead of re-stating fixture semantics locally.
