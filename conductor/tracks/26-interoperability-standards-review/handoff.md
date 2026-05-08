# Handoff: Track 26 Interoperability Standards Review

Last updated: 2026-05-08

## Summary

Captured the interoperability mapping story so downstream tracks can rely on a named set of supported, partial, deferred, and unsupported translations. The current implementation adds explicit `standards-mapping` and `adr-recommendations` artifacts rather than leaving those gates implicit in the summary page.

Current Track 26 mapping status:

| Standard | Label | Evidence summary |
|---|---|---|
| DEVS | Partial | Conceptual event-ordering/replay alignment only. |
| FMI/FMU | Partial | Unpacked-layout validation, lifecycle wrapper, and unpacked export layout generation are present; full FMU execution remains unclaimed. |
| SBML | Deferred | Named future bridge target only. |
| CellML | Deferred | Named future bridge target only. |
| OpenTelemetry semantic conventions | Partial | Trace/log naming guidance only; no native exporter. |
| Arrow C Data Interface | Partial | Field-level Arrow type contract exists; no ArrowArray/ArrowSchema FFI fixture. |
| Arrow IPC | Deferred | Integration target only; current event-log roundtrip is smoke bytes. |
| Parquet | Deferred | Planned analytical output only. |

## Files changed

`conductor/interoperability-standards.md`, `conductor/tracks/26-interoperability-standards-review/plan.md`, `conductor/tracks/26-interoperability-standards-review/test-matrix.md`, `conductor/tracks/26-interoperability-standards-review/risk-register.md`, `conductor/tracks/26-interoperability-standards-review/handoff.md`, `conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`, `docs/interoperability/standards-review.md`, `docs/interoperability/standards-mapping.md`, `docs/interoperability/adr-recommendations.md`

## Contracts consumed

`conductor/interoperability-standards.md`, `conductor/compatibility-promise.md`, `conductor/testing-strategy.md`, `conductor/experiment-runner.md`, `conductor/trustworthy-simulation.md`, `docs/trustworthy-simulation/`

## Release gates affected

Interoperability review now names the release-impacting assertions that need review before an external-compatibility claim is made.

- Arrow claims must name Arrow C Data Interface, Arrow IPC, or Parquet exactly.
- Arrow C Data Interface is partial field-level alignment, not an exported C Data Interface boundary.
- Arrow IPC and Parquet are deferred until real serializers/readers and fixtures exist.
- OpenTelemetry semantic conventions are partial naming guidance, not native OTel or OTLP export support.
- FMI/FMU is partial scaffold support, not arbitrary third-party FMU execution.
- SBML and CellML are deferred.
- Ecosystem references are unsupported compatibility claims and may be used only for comparison or teaching.
- `standards-mapping` is satisfied by `docs/interoperability/standards-mapping.md`.
- `adr-recommendations` is satisfied by `docs/interoperability/adr-recommendations.md`.

## Risks and unresolved questions

The main risk is overstating runtime interoperability where the repo only supports terminology alignment or data-exchange alignment.

## Validation

Run these focused checks after Track 26 edits:

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`
- `node scripts/validation/validate-tracks21-27.mjs`
- `rg -n "DEVS|FMI/FMU|SBML|CellML|OpenTelemetry|Arrow C Data Interface|Arrow IPC|Parquet|standards-mapping|adr-recommendations|Supported|Partial|Deferred|Unsupported|ADR-026" docs/interoperability conductor/interoperability-standards.md conductor/tracks/26-interoperability-standards-review`

## Contracts changed

The interoperability contract now classifies named standards as partial, deferred, or unsupported instead of allowing broad external-compatibility claims.

## Tests added

`conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` validates the standards review, labels, exact claim-boundary language, standards-mapping rows, and ADR recommendation rows.

## Known risks

The main risk remains overstating runtime interoperability where only terminology, field-level, or scaffold alignment exists.

## Follow-up issues

Add conformance assertions before upgrading any partial or deferred standard to a stronger support label.

## Integration notes

Downstream docs should reuse the exact Track 26 labels for Arrow, FMI/FMU, OpenTelemetry, SBML, CellML, and ecosystem comparison claims.
## Phase closeout evidence

2026-05-08 implementation/review pass:

- `$conductor-review` findings: no blocking findings after the standards-mapping and ADR recommendation artifacts were added and validator coverage was updated.
- Accepted fixes: added `docs/interoperability/standards-mapping.md`, added `docs/interoperability/adr-recommendations.md`, linked both from the public and conductor-facing standards pages, and extended the Track 26 validator.
- Deferred or blocked fixes: commit/push closeout and strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` are blocked by pre-existing unrelated dirty worktree changes outside Track 26 ownership. Do not advance to `Done` until the cleaned slice is committed or the unrelated work is isolated.
- Validation commands: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` passed; `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` passed; `node scripts/validation/validate-tracks21-27.mjs` passed the Track 26 step but failed in Track 27's docs workflow because `website/scripts/check-links.js` scanned `bindings/typescript/node_modules`; `rg -n "DEVS|FMI/FMU|SBML|CellML|OpenTelemetry|Arrow C Data Interface|Arrow IPC|Parquet|standards-mapping|adr-recommendations|Supported|Partial|Deferred|Unsupported|ADR-026" docs/interoperability conductor/interoperability-standards.md conductor/tracks/26-interoperability-standards-review` passed.
- Git cleanup state: dirty before and after this slice due to unrelated worker edits; Track 26-owned diffs are limited to the files named above and status/ledger entries.
- Commit SHA: pending.
- Pushed ref: pending.
- Next-phase decision: Track 26 can move from `Spec Approved` to `In Review` after focused validation passes; keep `Done` blocked until strict git closeout evidence exists.
