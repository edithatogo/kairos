# Handoff — 12 Conformance, Testing & Benchmarks

Last updated: 2026-05-08

## Summary

Documented the current ready fixture IDs and canonical benchmark scenario names so downstream tracks can validate against the stable manifest without re-stating the contract. Added a reusable bootstrap conformance runner with a direct local CLI, Track 07-13 hardening validator, and metadata-only benchmark smoke harness that do not require native binding link tests.

Added a metadata-only chaos experiment manifest covering the first required fault families: event corruption, entity exhaustion, telemetry loss, and ordering inversion. This records the resilience contract without claiming native fault injection, a checked-in chaos runner, or nightly execution.

2026-05-08 stabilization update: fixed the JavaScript conformance runner's RNG replay path to mirror the Rust `kairo-ecs-rng` SplitMix64/domain-separated entity seed algorithm. The previous JS runner used an older 32-bit approximation, which made `rng_reproducibility_v1` fail even though the Rust fixture consumer passed. The runner self-test now asserts the same stream as `conformance/fixtures/rng_replay.json`.

2026-05-08 PR review update: accepted the Qodo review finding that JavaScript `Number` parsing can silently lose precision for Rust-side `u64` fixture inputs. The runner now rejects unsafe integer RNG seeds and entity IDs before converting them to BigInt, and the self-test covers both direct caller and fixture-runner rejection paths.

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
node tests/conformance/chaos-check.mjs
node tests/conformance/track07_13_hardening_check.mjs
node tests/conformance/track12_20_evidence_check.mjs
python benches/benchmark_smoke.py
cargo +stable-x86_64-pc-windows-gnu check -p kairo-ecs-bench
cargo +stable-x86_64-pc-windows-gnu test --workspace
pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
Test-Path .github/workflows/conformance.yml
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
- PR review fixes: added safe-integer validation for RNG fixture fields and direct `deterministicStream` inputs so JavaScript precision loss cannot silently change `u64` replay semantics.
- Deferred or blocked fixes: none for this stabilization slice.
- Validation commands: recorded in `Current local checks` above, including `cargo +stable-x86_64-pc-windows-gnu test --workspace`.
- Cleanup state: local working tree was clean after the Track 12 stabilization commit.
- Commit SHA / pushed ref: local commit created for this pass; pushed ref not recorded in this pass.
- Next-phase decision: superseded by the 2026-05-08 review closeout below.

2026-05-08 review closeout:

- Review scope: Track 12 conductor files, the Track 12 central status rows, `conformance/`, `tests/conformance/`, `benches/`, `crates/kairo-ecs-bench/`, and the relevant closeout validators.
- Findings: no open Track 12 blocker remains after the closed `conductor/phase-closeout.yaml` entry for PR #12, the pushed `origin/main` evidence, and the fresh local gates listed in `Current local checks`.
- Accepted fixes: updated Track 12 from `In Review` to `Done` in the machine-readable and human-readable track indexes, and recorded this review closeout evidence.
- Deferred or blocked fixes: none for Track 12 closeout. Runtime chaos injection, nightly chaos scheduling, OSS-Fuzz registration, and the planned DES/ABM/hybrid/Arrow/FFI fixture families remain explicitly scoped as later beta-and-beyond work rather than blockers for this bootstrap conformance track.
- Validation commands: the `Current local checks` above passed on 2026-05-08.
- Cleanup state: working tree was clean before the review closeout edits.
- Commit SHA / pushed ref: Track 12 review-entry evidence remains the closed ledger commit `9f6dbf1970bf85304748ca68d21b54df87280de7` on `origin/main`; this review closeout updates local conductor status files.
- Next-phase decision: Track 12 is `Done`.

## Q5.1 coordinated conformance slice

Track03/12 own the test-only DES fixtures and optional reference tooling; Track13
coordinates the native-owner CI addition. Clean source `32c10ff8ce999f1f193d9f3821416f66a57da9d4` passed
214 core/DES debug tests, 175 DES release tests, strict scoped Clippy, formatting,
8 comparator tests and a fresh hash-locked SimPy4.1.2 comparison (5 cases/29 rows).
See `conductor/design/queue/q5.1-conformance-20261004.md` and the paired source
verification receipt. Native fixture aggregate checksum is `47cfb7dca4211252`;
full byte equality, expected strategy traces and interval accounting are the
oracles. Existing ready fixture IDs/goldens remain unchanged. This does not
close Q5 or reopen historical track completion; Q5.2–Q5.4 and existing holds remain.
Exact-head Linux/macOS native-owner qualification and parent integration are
controlled by the parent pin contract, not this antecedent source receipt.

## Q5.2–Q5.4 queue development handoff — 2026-10-05

The accepted Q5.2 benchmark and Q5.3 compatibility slices are source-bound to Kairos `eae890b0a2a3524a543ec4ee4aca61346e273b52` on `origin/codex/careops-q53-compatibility` (stacked/draft PR #218). The parent records Q5.2's full bounded 39-scenario/five-repeat matrix, all thirteen 100,000-request cases, and two passing isolated Ubuntu comparisons in `conductor/evidence/q5.2-completion-20261005/`; initial failed local observations remain retained and no general speedup is claimed. Q5.3 consumer/migration/backend evidence is in `conductor/evidence/q5.3-completion-20261005/`. The independent parent reader completed 11 checks with three zero exits, replayed both exact examples byte-equal to their expected oracles, and verified twelve aggregate hash lines matching Q5.1 checksum `47cfb7dca4211252`; parent receipt is slated for `conductor/evidence/q5.4-completion-20261005/manual-review.json`. Parent evidence integration and hosted acceptance remain pending. Existing fixture identity/goldens and historical Track 12 `Done` scope are unchanged; this note does not reopen or broaden Track 12, assert native fault-injection coverage, or waive release/security gates.


## C4.1 bounded fixture preparation — 2026-10-05

Accepted experimental fixture/test source `6a17578a16d4bb13969961dfd116b3afaf7c8c6c`
adds generic `conformance/c41` and one private calibration integration test.
42 fixtures, exact/pinned independent references, 8 native and 10 Python tests
qualify fixture preparation only. Missing/runtime-mock reports fail the opt-in
C4.2 gate. See [retained review and evidence](../../evidence/c4.1-preparation-20261005/README.md).
Track 21 owns semantics and Track 12 the references; this does not advance an
upstream phase or change historical registry/ledger status. C4.2 runtime,
C4.3 sidecars and C4.4/C-04 acceptance remain open; no release claim follows.

## C4.2 bounded private runtime — 5 October 2026

Runtime source `59d7dbb0c004654e3da90951d42e2b983ef23993`: 42 actual cases independently conform; 119 calibration tests pass on Rust1.99/1.88, strict current Clippy and formatting pass. Paired residuals, fixed groups/source windows, diagnostic retention, provenance binding and ordered reduction are implemented privately. [Retained evidence](../../evidence/c4.2-runtime-20261005/README.md). This supersedes the historical C4.1 runtime-not-implemented boundary only for this experimental leaf; public API, C4.3, C4.4/C-04 and clinical/release gates stay open. No upstream phase/registry/ledger status advances. Hosted successor checks and parent pin acceptance are separate.
