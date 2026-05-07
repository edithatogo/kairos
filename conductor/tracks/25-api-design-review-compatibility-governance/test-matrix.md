# Test Matrix: Track 25 API Design Review & Compatibility Governance

| Check | Alpha | Beta | RC | 1.0 |
|---|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| Machine-readable inventory exists at `docs/design/protected-surface-inventory.json` | yes | yes | yes | yes |
| Policy-pack validation passes with `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` | yes | yes | yes | yes |
| Release-gate validation passes with `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate` | no | yes | yes | yes |
| Surface inventory exists for `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, and `bindings/go` | yes | yes | yes | yes |
| Surface inventory covers `include/kairo_ecs.h`, `schemas/arrow/event_log_v1.schema.json`, and `conformance/fixtures` | yes | yes | yes | yes |
| Compatibility policy names the live crate and package roots | yes | yes | yes | yes |
| `conductor/delivery-readiness-checklist.md` includes the compatibility gate rows | yes | yes | yes | yes |
| `conductor/quality-gates.md` includes the compatibility gate section | yes | yes | yes | yes |
| Breaking-change definition is explicit | no | yes | yes | yes |
| ADR requirement is explicit | no | yes | yes | yes |
| Migration note requirement is explicit | no | yes | yes | yes |
| Release-stage decision rules are explicit | no | yes | yes | yes |
| Release hold path is documented | no | yes | yes | yes |
| Package catalog and package matrix are aligned to the live binding/package roots | no | yes | yes | yes |
| Compatibility notes name the exact affected crate or package root | no | yes | yes | yes |
| Rename, split, or removal of a published root is treated as breaking | no | yes | yes | yes |
| Any public API, ABI, or schema change without an ADR is rejected | no | yes | yes | yes |
| Any breaking change without a migration note is rejected at beta and beyond | no | yes | yes | yes |
| Any root mismatch between policy and release docs is a release hold | no | yes | yes | yes |
| Cross-track evidence-boundary guard keeps release-boundary wording present | yes | yes | yes | yes |

## Evidence commands

```powershell
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate
node scripts/validation/validate-track21-27-evidence-boundaries.mjs
node scripts/validation/validate-tracks21-27.mjs
rg -n "validate-compatibility-pack|protected-surface-inventory|Breaking-change rules|Release hold criteria" conductor/contracts/versioning-compatibility.md conductor/quality-gates.md conductor/delivery-readiness-checklist.md docs/design
```

## Current validation evidence

| Command | Result | Evidence |
|---|---|---|
| `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` | pass | `compatibility pack validation passed: 13 protected surfaces` |
| `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate` | pass | `compatibility release-gate validation passed: 13 protected surfaces` |
| `rg -n "validate-compatibility-pack|protected-surface-inventory|Breaking-change rules|Release hold criteria" conductor/contracts/versioning-compatibility.md conductor/quality-gates.md conductor/delivery-readiness-checklist.md docs/design` | pass | Found policy, readiness, quality-gate, design-index, and validator references |
| `cargo fmt --all --check` | pass | Rust formatting gate passed; no formatting changes needed |
| `node scripts/validation/validate-tracks21-27.mjs` | pass | Ran the non-release compatibility policy pack check with adjacent Track 21-27 local validators; all seven track checks passed. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, cleaned commit/push, and blocker recording.