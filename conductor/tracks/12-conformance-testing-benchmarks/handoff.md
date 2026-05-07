# Handoff — 12 Conformance, Testing & Benchmarks

Last updated: 2026-05-07

## Summary

Documented the current ready fixture IDs and canonical benchmark scenario names so downstream tracks can validate against the stable manifest without re-stating the contract. Added a reusable bootstrap conformance runner with a direct local CLI, Track 07-13 hardening validator, and metadata-only benchmark smoke harness that do not require native binding link tests.

Added a metadata-only chaos experiment manifest covering the first required fault families: event corruption, entity exhaustion, telemetry loss, and ordering inversion. This records the resilience contract without claiming native fault injection, a checked-in chaos runner, or nightly execution.

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
`tests/conformance/runner-self-test.mjs`
`tests/conformance/conformance-check.mjs`
`tests/conformance/track07_13_hardening_check.mjs`
`conformance/chaos/manifest.json`
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

Manifest validation, runner checks, and benchmark-name checks are defined in `test-matrix.md`.

Current local checks:

```text
node tests/conformance/conformance-check.mjs
node tests/conformance/runner.mjs
node tests/conformance/runner.mjs --list
node tests/conformance/runner-self-test.mjs
node tests/conformance/track07_13_hardening_check.mjs
python benches/benchmark_smoke.py
cargo check -p kairo-ecs-bench
```

## Known risks

Fixture and benchmark names must stay stable once Track 01 and the binding tracks start consuming them.
The remaining planned fixture families are still future scope: `des_resource_queue_v1`, `abm_behavior_update_v1`, `hybrid_des_abm_v1`, `arrow_event_log_v1`, and `ffi_lifecycle_v1`.
Native chaos validation, nightly scheduling, and OSS-Fuzz language in the plan/spec remain future scope until checked-in runtime harnesses exist.

## Integration notes

Track 01 consumes the scheduler ordering, scheduler cancellation, and RNG fixtures.
Track 02 consumes the FFI lifecycle fixture once the facade contract is ready.
Tracks 06-11 should use the manifest instead of re-stating fixture semantics locally.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.