# Test Matrix — 12 Conformance, Testing & Benchmarks

## Required tests

- `cargo test --workspace`
- `Test-Path .github/workflows/conformance.yml`
- `pwsh -NoProfile -File scripts\validate_conformance_fixtures.ps1`
- Manifest validation for `conformance/fixtures/manifest.json`
- JSON syntax check for each ready fixture under `conformance/fixtures`
- Benchmark plan presence and scenario-name check under `benches`
- `node tests/conformance/conformance-check.mjs`
- `python benches/benchmark_smoke.py`
- `cargo check -p kairo-ecs-bench`

## CI commands

```bash
cargo test --workspace
test -f .github/workflows/conformance.yml
pwsh -NoProfile -File scripts\validate_conformance_fixtures.ps1
node -e "const fs=require('fs'); const manifest=JSON.parse(fs.readFileSync('conformance/fixtures/manifest.json','utf8')); const required=['scheduler_ordering_v1','scheduler_cancellation_v1','rng_reproducibility_v1']; for (const id of required) { const f=manifest.fixtures.find(x => x.id === id); if (!f || f.status !== 'ready') throw new Error('Missing ready fixture: ' + id); if (!fs.existsSync('conformance/fixtures/' + f.source)) throw new Error('Missing ready fixture file: ' + f.source); }"
node -e "const fs=require('fs'); const manifest=JSON.parse(fs.readFileSync('conformance/fixtures/manifest.json','utf8')); const scenarios=['schedule_1m_events','pop_1m_events','schedule_cancel_1m_mixed','create_1m_entities','component_insert_1m','hybrid_des_abm_smoke_100k']; for (const scenario of scenarios) { const b=manifest.benchmarks.find(x => x.id === scenario); if (!b || b.status !== 'canonical') throw new Error('Missing canonical benchmark: ' + scenario); }"
node -e "const fs=require('fs'); const plan=fs.readFileSync('benches/benchmark-plan.md','utf8'); for (const scenario of ['schedule_1m_events','pop_1m_events','schedule_cancel_1m_mixed','create_1m_entities','component_insert_1m','hybrid_des_abm_smoke_100k']) { if (!plan.includes(scenario)) throw new Error('Missing benchmark scenario: ' + scenario); }"
just validate-conformance
node tests/conformance/conformance-check.mjs
python benches/benchmark_smoke.py
cargo check -p kairo-ecs-bench
test -f benches/benchmark-plan.md
test -f conformance/fixtures/README.md
```

## Test matrix

| Test | Status | Rust (Track 01) | FFI (Track 02) | Bindings (Track 06-11) |
|---|---|---|---|---|
| Chaos experiments pass (no panic, correct error codes) | pending | yes | yes | yes |
| OSS-Fuzz integration active with ≥2 fuzz targets | no | yes | yes | yes |
```
