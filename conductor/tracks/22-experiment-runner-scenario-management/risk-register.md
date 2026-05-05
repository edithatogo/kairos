# Risk Register: Track 22 Experiment Runner & Scenario Management

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Scenario manifest schema not versioned | 4 | 4 | 16 | Version the manifest schema (`v1`, `v2`) and keep backwards-compatible parsers; fail CI on unknown schema keys | experiment-agent | Unknown schema key detected without version bump |
| Resumability not tested under partial failure | 3 | 5 | 15 | Integration test that kills runner mid-sweep and verifies resume produces identical results | ci-agent | Resume test fails or produces divergent results |
| Parameter sweep output too large for CI | 3 | 3 | 9 | Cap sweep output per CI run; default to summary-only in CI, full output in manual/cloud runs | experiment-agent | CI storage overflow from sweep output |
| CLI interface not documented before first use | 4 | 4 | 16 | Publish `kairo-ecs-cli --help` output, man page, and quickstart before allowing usage to grow | experiment-agent | CLI ships without `--help` output matching implementation |
| Experiment runner conflates simulation control with analysis | 3 | 3 | 9 | Separate CLI into `run`, `collect`, and `analyze` subcommands; keep analysis in a distinct crate from execution | experiment-agent | Analysis code coupled to execution crate |
