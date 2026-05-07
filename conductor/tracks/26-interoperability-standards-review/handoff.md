# Handoff: Track 26 Interoperability Standards Review

Last updated: 2026-05-07

## Summary

Captured the interoperability mapping story so downstream tracks can rely on a named set of supported, partial, deferred, and unsupported translations.

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

`conductor/interoperability-standards.md`, `conductor/tracks/26-interoperability-standards-review/plan.md`, `conductor/tracks/26-interoperability-standards-review/test-matrix.md`, `conductor/tracks/26-interoperability-standards-review/risk-register.md`, `conductor/tracks/26-interoperability-standards-review/handoff.md`, `conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`, `docs/interoperability/standards-review.md`

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

## Risks and unresolved questions

The main risk is overstating runtime interoperability where the repo only supports terminology alignment or data-exchange alignment.

## Validation

Run these focused checks after Track 26 edits:

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`
- `rg -n "DEVS|FMI/FMU|SBML|CellML|OpenTelemetry|Arrow C Data Interface|Arrow IPC|Parquet|Supported|Partial|Deferred|Unsupported" docs/interoperability/standards-review.md conductor/interoperability-standards.md conductor/tracks/26-interoperability-standards-review`

## Contracts changed

The interoperability contract now classifies named standards as partial, deferred, or unsupported instead of allowing broad external-compatibility claims.

## Tests added

`conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1` validates the standards review, labels, and exact claim-boundary language.

## Known risks

The main risk remains overstating runtime interoperability where only terminology, field-level, or scaffold alignment exists.

## Follow-up issues

Add conformance assertions before upgrading any partial or deferred standard to a stronger support label.

## Integration notes

Downstream docs should reuse the exact Track 26 labels for Arrow, FMI/FMU, OpenTelemetry, SBML, CellML, and ecosystem comparison claims.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.