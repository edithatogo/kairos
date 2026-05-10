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
| `standards-mapping` gate artifact exists and names exact claim surfaces, evidence, and missing behavior | yes | yes | yes |
| `adr-recommendations` gate artifact exists and names ADR thresholds, recommendation IDs, and claim-change triggers | yes | yes | yes |

## Evidence checks

| Evidence target | Required result |
|---|---|
| `docs/interoperability/standards-review.md` | Includes one mapping row each for DEVS, FMI/FMU, SBML, CellML, OpenTelemetry semantic conventions, Arrow C Data Interface, Arrow IPC, and Parquet. |
| `docs/interoperability/standards-mapping.md` | Satisfies `standards-mapping` with one primary row for each target standard plus release-language rewrites and unsupported ecosystem guards. |
| `docs/interoperability/adr-recommendations.md` | Satisfies `adr-recommendations` with ADR threshold rules, `ADR-026-001` through `ADR-026-009`, and follow-up priorities. |
| `conductor/interoperability-standards.md` | Mirrors the Track 26 status vocabulary and current labels. |
| `conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` | Fails if any target standard, status label, evidence citation, release guard, standards-mapping gate, or adr-recommendations gate is missing. |
| Arrow evidence | Names `crates/kairo-ecs-arrow/src/lib.rs`, `crates/kairo-ecs-arrow/tests/schema_compatibility.rs`, and `schemas/arrow/event_log_v1.schema.json`. |
| FMI/FMU evidence | Names `docs/fmi-digital-twin/import-guide.md`, `docs/fmi-digital-twin/export-guide.md`, and the import/export crate surfaces. |

## Validation commands

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`
- `node scripts/validation/validate-tracks21-27.mjs`
- `just docs-build`
- `just check-docs`
- `rg -n "DEVS|FMI/FMU|SBML|CellML|OpenTelemetry|Arrow C Data Interface|Arrow IPC|Parquet|standards-mapping|adr-recommendations|Supported|Partial|Deferred|Unsupported|ADR-026" docs/interoperability conductor/interoperability-standards.md conductor/tracks/26-interoperability-standards-review`

## Focused validation run

| Command | Result | Evidence |
|---|---|---|
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` | Pass | `Track 26 standards review validation passed: 8 standards, 4 labels, evidence, release guards, standards-mapping, and adr-recommendations found.` |
| `rg -n "DEVS\|FMI/FMU\|SBML\|CellML\|OpenTelemetry\|Arrow C Data Interface\|Arrow IPC\|Parquet\|standards-mapping\|adr-recommendations\|Supported\|Partial\|Deferred\|Unsupported\|ADR-026" docs/interoperability conductor/interoperability-standards.md conductor/tracks/26-interoperability-standards-review` | Pass | Output showed the required standard names, labels, standards-mapping gate text, ADR recommendation IDs, public review, conductor mirror, handoff, risk register, test matrix, plan, spec, and validator. |
| `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` | Pass | Cross-track evidence-boundary guard passed for Track 21-27, including compatibility and standards release boundaries. |
| `just check-docs` | Fail | `just` is not recognized on PATH in this PowerShell session. |
| `node scripts/dx/validate-docs-workflow.mjs` | Pass | Link check passed with 29 required paths and 2 Markdown sources, `website/build/index.html` was built, and preview smoke passed at `http://127.0.0.1:41727/`. |
| `node scripts/validation/validate-tracks21-27.mjs` | Pass | 2026-05-09 rerun passed Track 21 through Track 27, including Track 26 standards review, Track 27 docs workflow, and the final cross-track evidence-boundary guard. |
| `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Pass | 2026-05-09 rerun reported `0 error(s), 0 warning(s)` and `Conductor phase gate validation passed.` |
| `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Fail | Closeout gate correctly failed because the working tree has unrelated uncommitted tracked or untracked changes outside Track 26 ownership. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
