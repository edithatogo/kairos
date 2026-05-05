# Test Matrix — 00 Project Foundation, Governance & Naming

## Required tests

- Required Conductor setup files exist.
- All 32 track directories exist.
- Every track has `spec.md`, `plan.md`, `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.
- `conductor/tracks.yaml` exists as the machine-readable index.
- Initial implementation-readiness files exist.
- The setup validators succeed against the current tree.

## CI commands

```bash
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo
test -f conductor/status.md
test -f conductor/tracks.yaml
test -f conductor/implementation-readiness.md
test -f Cargo.toml
test -d crates
test -d conformance/fixtures
test -d website
```
