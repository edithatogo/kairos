# Track 106 handoff

Last updated: 2026-10-25.

## Summary

Track registration and the bounded preview implementation are underway. The
candidate restores the retained API shape without changing dispatch behavior.

## Current state

Track registration is in progress at exact base
`5692e6a9ee866131262d60103a84e2544dc51e4a`. The local candidate is isolated in
worktree `/Volumes/PortableSSD/codex-worktrees/kairos-r10-3-compat-20261025`.

## Evidence

- Before implementation, CareOps R10.3 compilation was observed to fail because
  `Scheduler::peek_next` is absent at `crates/careops-ed/src/pathway.rs:269,274`.
- The retained CareOps Kairos pin `44261eb5cf8275f132f63a3396903d116d701dbe`
  contains the exact `ScheduledEventPreview` shape and observer semantics now
  scoped by this track.
- Track registration review, implementation, native checks, owner reviews and
  CareOps consumer checks are pending until their respective evidence is added.

## Local Conductor review

The source review found and fixed two in-scope gaps before closeout: rustdoc now
labels this public API as experimental, and a third test proves live pending
membership remains cancellable after preview, including a cancelled-only queue.
No ordering, timing, dispatch or API-shape issue remains in the local review.
This is not a substitute for the named core, conformance or API Governance
owner reviews; all three remain pending.

## Validation receipts

Working directory: `/Volumes/PortableSSD/codex-worktrees/kairos-r10-3-compat-20261025`.
Candidate base: `5692e6a9ee866131262d60103a84e2544dc51e4a`.
Toolchain: Rust `1.99.1`, Cargo `1.99.1`; repository-local Python `3.15.1`
was verified with `scripts/python_runtime.py` for the advisory harness.

- `cargo test --locked -p kairo-ecs-core --test track106_scheduler_preview`:
  passed, 3 tests. Before implementation, the same command failed only because
  `ScheduledEventPreview` and `peek_next` were absent.
- `cargo test --locked -p kairo-ecs-core`: passed, 47 tests.
- `cargo fmt --all --check`: passed.
- `cargo doc --locked -p kairo-ecs-core --no-deps`: passed; output at
  `target/doc/kairo_ecs_core/index.html`.
- `pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1`: passed.
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`: passed,
  0 errors and 0 warnings (informational findings remain in older tracks).
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: passed,
  0 errors and 0 warnings.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`:
  passed, 0 errors and 0 warnings.
- Source commit SHA and clean-tree closeout: pending.

## Files changed

`crates/kairo-ecs-core/src/lib.rs`,
`crates/kairo-ecs-core/tests/track106_scheduler_preview.rs`,
`conformance/fixtures/track106-scheduler-preview.json`,
`conformance/fixtures/manifest.json`, and the Track 106 Conductor artifacts.

## Contracts consumed

Track 01 core ordering/cancellation contract, Track 12 fixture contract, Track
25 versioning/compatibility rules, and the CareOps R10.3 exact consumer contract.

## Contracts changed

No shared Track 01/12/25 policy contract is changed. The additive preview API
contract is documented in the Track 106 ADR and fixture pending owner review.

## Tests added

Focused preview behavior tests passed (2/2); the full `kairo-ecs-core` package
passed 46 tests. Formatting, docs build and shared fixture validation passed;
exact receipts are recorded in `test-matrix.md`.

## Known risks

Core scheduler, conformance and API Governance owner reviews, plus downstream
CareOps R10.3 verification, remain outstanding.

## Follow-up issues

If remote push is required for owner review, obtain authorization before that
remote mutation. Keep the CareOps parent pin unchanged until every review and
consumer gate passes.

## Integration notes

Candidate base is `5692e6a9ee866131262d60103a84e2544dc51e4a`; retained consumer
pin remains `44261eb5cf8275f132f63a3396903d116d701dbe` until separate acceptance.

## Phase closeout evidence

- `$conductor-review`: local review completed for track registration and source.
- Accepted fixes: experimental maturity label and live-pending/cancelled-only
  assertions applied; no other in-scope fixes remain.
- Commit SHA: pending local commit.
- Pushed ref: pending; owner-review and downstream acceptance remain open.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pending.
- Next-phase decision: submit exact candidate for core/API/conformance review;
  then run the exact CareOps consumer gate. Parent pin remains unchanged.

## Parent repository boundary

CareOps pin and lockfile are unchanged. No upstream acceptance, push, release or
consumer acceptance is claimed by this draft. Pin work can start only after the
candidate commit passes the listed reviews and exact CareOps tests.
