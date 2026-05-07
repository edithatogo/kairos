# Test Matrix: Track 26 Interoperability Standards Review

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| Standards inventory names DEVS, FMI/FMU, SBML, CellML, OpenTelemetry, Arrow C Data Interface, Arrow IPC, and Parquet | yes | yes | yes |
| Mapping table distinguishes supported, partial, deferred, and unsupported mappings | yes | yes | yes |
| Markdown lint/link check | yes | yes | yes |
| Artifact existence check | yes | yes | yes |
| Docs build smoke test passes | yes | yes | yes |
| Release-impacting assertions are named for Arrow schema and semantic-convention drift | no | yes | yes |
| Known gaps are documented with explicit missing behavior | yes | yes | yes |
| Red-team objections about false interoperability claims are answered | yes | yes | yes |
| Local Track 26 standards validator passes | yes | yes | yes |
| Cross-track evidence-boundary guard keeps standards labels and deferred claims explicit | yes | yes | yes |

## Evidence checks

| Evidence target | Required result |
|---|---|
| `docs/interoperability/standards-review.md` | Includes one mapping row each for DEVS, FMI/FMU, SBML, CellML, OpenTelemetry semantic conventions, Arrow C Data Interface, Arrow IPC, and Parquet. |
| `conductor/interoperability-standards.md` | Mirrors the Track 26 status vocabulary and current labels. |
| `conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` | Fails if any target standard, status label, evidence citation, or release guard is missing. |
| Arrow evidence | Names `crates/kairo-ecs-arrow/src/lib.rs`, `crates/kairo-ecs-arrow/tests/schema_compatibility.rs`, and `schemas/arrow/event_log_v1.schema.json`. |
| FMI/FMU evidence | Names `docs/fmi-digital-twin/import-guide.md`, `docs/fmi-digital-twin/export-guide.md`, and the import/export crate surfaces. |

## Validation commands

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`
- `node scripts/validation/validate-tracks21-27.mjs`
- `just docs-build`
- `just check-docs`
- `rg -n "DEVS|FMI/FMU|SBML|CellML|OpenTelemetry|Arrow C Data Interface|Arrow IPC|Parquet|supported|partial|deferred|unsupported" docs/interoperability/standards-review.md conductor/tracks/26-interoperability-standards-review`

## Focused validation run

| Command | Result | Evidence |
|---|---|---|
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` | Pass | `Track 26 standards review validation passed: 8 standards, 4 labels, evidence, and release guards found.` |
| `rg -n "DEVS\|FMI/FMU\|SBML\|CellML\|OpenTelemetry\|Arrow C Data Interface\|Arrow IPC\|Parquet\|Supported\|Partial\|Deferred\|Unsupported" docs/interoperability/standards-review.md conductor/interoperability-standards.md conductor/tracks/26-interoperability-standards-review` | Pass | Output showed the required standard names and labels in the public review, conductor mirror, handoff, risk register, test matrix, plan, spec, and validator. |
| `just check-docs` | Fail | `just` is not recognized on PATH in this PowerShell session. |
| `node scripts/dx/validate-docs-workflow.mjs` | Pass | Link check passed with 29 required paths and 2 Markdown sources, `website/build/index.html` was built, and preview smoke passed at `http://127.0.0.1:41727/`. |
| `node scripts/validation/validate-tracks21-27.mjs` | Pass | Ran the Track 26 standards validator with the adjacent Track 21-27 local validators; all seven track checks passed. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, cleaned commit/push, and blocker recording.