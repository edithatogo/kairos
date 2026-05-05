# Risk Register: Track 22 Experiment Runner & Scenario Management

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Scenario manifest schema not versioned | 4 | 4 | 16 | Version the manifest schema (`v1`, `v2`) and keep backwards-compatible parsers; fail CI on unknown schema keys | experiment-agent | Unknown schema key detected without version bump |
| Resumability not tested under partial failure | 3 | 5 | 15 | Integration test that kills runner mid-sweep and verifies resume produces identical results | ci-agent | Resume test fails or produces divergent results |
| Parameter sweep output too large for CI | 3 | 3 | 9 | Cap sweep output per CI run; default to summary-only in CI, full output in manual/cloud runs | experiment-agent | CI storage overflow from sweep output |
| CLI interface not documented before first use | 4 | 4 | 16 | Publish `kairo-ecs-cli --help` output, man page, and quickstart before allowing usage to grow | experiment-agent | CLI ships without `--help` output matching implementation |
| Experiment runner conflates simulation control with analysis | 3 | 3 | 9 | Separate CLI into `run`, `collect`, and `analyze` subcommands; keep analysis in a distinct crate from execution | experiment-agent | Analysis code coupled to execution crate |
| Scenario notes drift from committed fixture paths | 2 | 4 | 8 | `scripts/scenarios/validate-track22-smoke.ps1` checks `scenarios/manifest-index.json` against `examples/experiments/*`, `conformance/fixtures/manifest.json`, `vvuq_scenario_replay_v1`, and `scheduler_ordering_v1` | experiment-agent | Validator fails or a documented scenario path is moved without index update |
| Local CLI replay smoke blocked by wrong Windows linker | 3 | 4 | 12 | Keep the read-only scenario validator as the local gate; require MSVC linker/Windows SDK PATH correction before accepting CLI replay evidence | experiment-agent | `cargo run -p kairo-ecs-cli -- validate-scenario ...` fails at link on CI or maintainer machines |
