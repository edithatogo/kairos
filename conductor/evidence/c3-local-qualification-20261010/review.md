# Review Report: C3 trace-driven shadow runner

## Summary

The experimental private implementation passes local synthetic C-03 checks; final hosted qualification and parent acceptance remain pending.

## Verification Checks

- [x] **Plan Compliance**: Yes for C3 runtime and fixture scope; C3.4 remains pending hosted closeout.
- [x] **Style Compliance**: Pass, fixed ticks and private typed contracts, formatting and strict Clippy.
- [x] **New Tests**: Yes, protocol plus actual Flow/C2 and fresh-process recovery.
- [x] **Test Coverage**: Native fixtures and protocol adversarial cases jointly cover C-03; no clinical or release claim.
- [x] **Test Results**: Passed the current gates in result.json; earlier failed attempts remain archived.

## Review fixes

Independent Luna reviews and coordinator integration corrected source-prefix visibility,
actual consumed dispatch accounting, terminal/pending inventory validation,
Submitted-target restoration, no-holder nonzero anchors with future interruption,
and metadata preflight before snapshot/image cloning. Exact-cap and cap-minus-one
controls include zero native callbacks on rejected restoration. Native output now
round-trips both residual and metric records through actual IPC and Parquet, with
exact batch count and record equality.

Reviewers: c3_semantics_preparation (native oracles and manual output readback),
c3_fixture_preparation (metadata bounds and workflow), c3_recovery_preparation
(native recovery and evidence gaps), plus coordinator source/integration review.
These are internal engineering role reviews, not external certification.

## Manual readback

The source ledger consumes anchor tick 0 and observed target tick 1: frontier 2.
An isolated native route takes 3 ticks and work 7, producing completion tick 10,
residual +9 and four predictive dispatches. Neither native time nor the late
prediction rewrites the observed source. A slower route moves prediction from
10 to 13 with the same anchor and residual +3 against observation 10.

The work checkpoint is cut after three dispatches at tick 3. Restoring that image
reports actual arrival 3 and incomplete work. Separate-process restoration forbids
model start, recaptures byte-identical checkpoint bytes before dispatch, then
matches baseline IDs, frontier, digest, event count and completion exactly.
Transit and overdue-work cuts both pass. Independent repeated debug/release
process outputs and checkpoint hashes match exactly (repeat-readback.json).

Advancing only p0 leaves the complete saved p1 spec/snapshot/native image unchanged.
The result list contains only p0 until p1 is advanced. Resource occupancy is
materialized as real native holder work in separate Flow worlds.

Four real route/work runs (1+4, 2+3, 3+2, 4+1) finish at 5. A separate observed
walk endpoint at 2, read through the ledger, selects 2+3. Equal total objective
values alone do not identify the primitive durations.

## Remaining boundaries

C3-specific hosted Ubuntu/macOS jobs and final source-bound publication are pending.
Public API/schema, candidate search, empirical/clinical validation and release
remain separate. The model adapter is trusted code, not a sandbox. Per-inventory
metadata limits are conservative estimates and do not cap total process memory.
