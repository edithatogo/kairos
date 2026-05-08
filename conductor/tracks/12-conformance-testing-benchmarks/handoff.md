# Handoff — 12 Conformance, Testing & Benchmarks

Last updated: 2026-05-08

## Summary

Documented the current ready fixture IDs and canonical benchmark scenario names so downstream tracks can validate against the stable manifest without re-stating the contract. Added a reusable bootstrap conformance runner with a direct local CLI, Track 07-13 hardening validator, and metadata-only benchmark smoke harness that do not require native binding link tests.

Added a metadata-only chaos experiment manifest covering the first required fault families: event corruption, entity exhaustion, telemetry loss, and ordering inversion. This records the resilience contract without claiming native fault injection, a checked-in chaos runner, or nightly execution.

2026-05-08 stabilization update: fixed the JavaScript conformance runner's RNG replay path to mirror the Rust `kairo-ecs-rng` SplitMix64/domain-separated entity seed algorithm. The previous JS runner used an older 32-bit approximation, which made `rng_reproducibility_v1` fail even though the Rust fixture consumer passed. The runner self-test now asserts the same stream as `conformance/fixtures/rng_replay.json`.

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
node tests/conformance/track12_20_evidence_check.mjs
python benches/benchmark_smoke.py
cargo +stable-x86_64-pc-windows-gnu check -p kairo-ecs-bench
cargo +stable-x86_64-pc-windows-gnu test --workspace
pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo
```

Previous local checks:

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
The JavaScript runner now duplicates the Rust RNG seed derivation constants. Any future Rust RNG contract change must update both the Rust fixture consumer and `tests/conformance/runner.mjs` in the same change.

## Integration notes

Track 01 consumes the scheduler ordering, scheduler cancellation, and RNG fixtures.
Track 02 consumes the FFI lifecycle fixture once the facade contract is ready.
Tracks 06-11 should use the manifest instead of re-stating fixture semantics locally.
The JavaScript runner now validates `rng_reproducibility_v1` against the same expected stream as the Rust Track 01 fixture consumer, so binding tracks can rely on the runner for the bootstrap RNG replay check.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

2026-05-08 stabilization review:

- `$conductor-review` scope: current Track 12 diff in `tests/conformance/runner.mjs`, `tests/conformance/runner-self-test.mjs`, and this handoff.
- Findings: no correctness, regression, ownership, or test-coverage findings after the RNG runner fix.
- Accepted fixes: replaced the stale JavaScript 32-bit RNG approximation with the Rust-compatible SplitMix64/domain-separated entity-seed derivation and updated the runner self-test expected stream.
- Deferred or blocked fixes: none for this stabilization slice.
- Validation commands: recorded in `Current local checks` above, including `cargo +stable-x86_64-pc-windows-gnu test --workspace`.
- Cleanup state: local working tree was clean after the Track 12 stabilization commit.
- Commit SHA / pushed ref: local commit created for this pass; pushed ref not recorded in this pass.
- Next-phase decision: keep Track 12 `In Progress`; do not promote until the local commit is pushed and a closed closeout entry is recorded in `conductor/phase-closeout.yaml` with strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` passing.
