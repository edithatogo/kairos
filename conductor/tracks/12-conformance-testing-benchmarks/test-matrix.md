# Test Matrix — 12 Conformance, Testing & Benchmarks

## Required tests

- `cargo test --workspace`
- `Test-Path .github/workflows/conformance.yml`
- Manifest validation for `conformance/fixtures/manifest.json`
- JSON syntax check for each ready fixture under `conformance/fixtures`
- Ready fixture IDs: `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, and `vvuq_scenario_replay_v1`
- Benchmark scenario names: `schedule_1m_events`, `pop_1m_events`, `schedule_cancel_1m_mixed`, `create_1m_entities`, `component_insert_1m`, and `hybrid_des_abm_smoke_100k`
- `node tests/conformance/conformance-check.mjs`
- `node tests/conformance/runner.mjs`
- `node tests/conformance/runner.mjs --list`
- `node tests/conformance/runner-self-test.mjs`
- `node tests/conformance/track07_13_hardening_check.mjs`
- `node tests/conformance/track12_20_evidence_check.mjs`
- `python benches/benchmark_smoke.py`
- `cargo check -p kairo-ecs-bench`

## CI commands

```bash
cargo test --workspace
test -f .github/workflows/conformance.yml
node tests/conformance/conformance-check.mjs
node tests/conformance/runner.mjs
node tests/conformance/runner.mjs --list
node tests/conformance/runner-self-test.mjs
node tests/conformance/track07_13_hardening_check.mjs
node tests/conformance/track12_20_evidence_check.mjs
node -e "const fs=require('fs'); const manifest=JSON.parse(fs.readFileSync('conformance/fixtures/manifest.json','utf8')); const required=['scheduler_ordering_v1','scheduler_cancellation_v1','rng_reproducibility_v1','vvuq_scenario_replay_v1']; for (const id of required) { const f=manifest.fixtures.find(x => x.id === id); if (!f || f.status !== 'ready') throw new Error('Missing ready fixture: ' + id); if (!fs.existsSync('conformance/fixtures/' + f.source)) throw new Error('Missing ready fixture file: ' + f.source); }"
node -e "const fs=require('fs'); const manifest=JSON.parse(fs.readFileSync('conformance/fixtures/manifest.json','utf8')); const scenarios=['schedule_1m_events','pop_1m_events','schedule_cancel_1m_mixed','create_1m_entities','component_insert_1m','hybrid_des_abm_smoke_100k']; for (const scenario of scenarios) { const b=manifest.benchmarks.find(x => x.id === scenario); if (!b || b.status !== 'canonical') throw new Error('Missing canonical benchmark: ' + scenario); }"
node -e "const fs=require('fs'); const plan=fs.readFileSync('benches/benchmark-plan.md','utf8'); for (const scenario of ['schedule_1m_events','pop_1m_events','schedule_cancel_1m_mixed','create_1m_entities','component_insert_1m','hybrid_des_abm_smoke_100k']) { if (!plan.includes(scenario)) throw new Error('Missing benchmark scenario: ' + scenario); }"
python benches/benchmark_smoke.py
cargo check -p kairo-ecs-bench
test -f benches/benchmark-plan.md
test -f conformance/fixtures/README.md
```

## Test matrix

| Test | Status | Rust (Track 01) | FFI (Track 02) | Bindings (Track 06-11) |
|---|---|---|---|---|
| Ready conformance fixtures pass in the local runner | required for this slice | target | target | target |
| Chaos experiment manifest validates required fault types | required for this slice | target | target | target |
| Canonical benchmark metadata matches the smoke file | required for this slice | target | target | target |
| Planned fixture families remain documented but not ready | required for this slice | target | target | target |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

## C4.1 bounded fixture preparation

- `cargo +1.99.0 test --locked -p kairo-ecs-calibration --test metrics_c41`:
  eight comparator/fixture controls pass; one named future report test ignored.
- Repeat on Rust 1.88 floor; explicit actual-output opt-in requires
  `C41_CANDIDATE_REPORT` and rejects comparator mocks.
- Pinned optional reference environment runs
  `python -m unittest discover -s conformance/c41 -p 'test_*.py' -v` and
  `python conformance/c41/generate_reference.py --check`.
- [Actual commands/versions/hashes and red controls](../../evidence/c4.1-preparation-20261005/README.md)
  qualify test preparation only. Production metric/sidecar/C-04 gates remain open.

## C4.2 actual runtime qualification

- `rustup run 1.99.0 cargo test --locked -p kairo-ecs-calibration`:119pass/4ignored; repeat on1.88floor passes.
- `rustup run 1.99.0 cargo clippy --locked -p kairo-ecs-calibration --all-targets -- -D warnings` and workspace Rustfmt pass.
- Actual `metrics_c42` report is source/commit/toolchain-bound; explicitly execute existing ignored `metrics_c41::c42_candidate_report` on both toolchains and independent exact Python comparator:42cases pass.
- False commit/toolchain and mutated metric controls fail as required; explicit100000points/side debug timing passes. [Exact receipts](../../evidence/c4.2-runtime-20261005/README.md) preserve source and output hashes. Public API, Arrow sidecars and C-04 remain open.
