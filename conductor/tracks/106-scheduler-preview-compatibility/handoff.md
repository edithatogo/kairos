# Track 106 handoff

Last updated: 2026-10-26.

## Summary

The bounded scheduler preview implementation is on PR #333 and is ready for
review. It restores the retained `ScheduledEventPreview` projection and
`Scheduler::peek_next` observer without changing dispatch behavior. The proposed
head at last verification is `29abb88718fb1e76b7dedf81766f7d49058dc5d2`, based
on `main` at `80063b7413a26013e265107ea462828d94a49849`.

## Current state

The PR is open and unmerged. The repository uses reviewer-agent/subagent
assessments for technical review; no separate human owner review is required.
Three reviewer agents assessed the candidate at
`89e713e23bf1784b9b4dd4b72d13a0c4e0c33749`. Their reports found no scheduler
correctness defect, confirmed the fixture should remain planned, and identified
three small fixes: exercise `Some(EntityId)`, assert the fixture's descriptive
ordering/requirement fields, and name the affected crate root in the ADR. These
fixes are applied; request a final review of the integrated commit before phase
acceptance. All 18 workflow runs on implementation head
`29abb88718fb1e76b7dedf81766f7d49058dc5d2` passed. Later docs-only commits
triggered fresh hosted checks; final PR-head status must be refreshed. Automated
Codex review on the implementation head reported no findings.

## Evidence

- The original CareOps R10.3 failure was observed at Kairos base
  `5692e6a9ee866131262d60103a84e2544dc51e4a`: `Scheduler::peek_next` was absent.
- The retained CareOps pin
  `44261eb5cf8275f132f63a3396903d116d701dbe` contains the expected preview
  shape and observer semantics.
- Current-head CareOps consumer source: `f2c4f7efb709a8ac184b111688f1656051a30b39`.
  R10.3 test hash:
  `1a453cd35ee3ae224f7318f73eb28f5c68740dc7d6ecd5f40725f352b5de6853`.
- With the original CareOps lockfile hash
  `9ad9d38b8fe43a10a90a02989c1c1adf83607a90d620a64242386398e1a671f3`,
  `cargo check --locked --offline -p careops-ed-cli` stopped before compilation
  because Cargo needed to update the lock.
- With a candidate-specific lock hash
  `24747479d6663d57aa4bacec437f60c652d78f07ec4c7a17883e98391ce2ffb8`,
  `cargo check --locked --offline -p careops-ed-cli`,
  `cargo test --locked --offline -p careops-ed --test r10_tabular_reference`,
  and Rust 1.99.1 rustfmt checks passed. The two R10.3 reference tests passed.
- The candidate lock adds/updates RNG dependencies (`rand`, `rand_chacha`,
  `rand_core`, `rand_distr`, `libm`) and removes `thiserror` from `careops-ed`;
  the lock contains parallel rand 0.9/0.10 versions. This delta needs owner
  assessment before any parent lock or pin change. The single maintainer remains
  final integrator; no separate owner reviewer is required.
- The disposable CareOps checkout was restored to pin
  `44261eb5cf8275f132f63a3396903d116d701dbe` and the original lock. The parent
  CareOps pin and lock remain unchanged.

## Local review and checks

Local source review found and fixed two in-scope issues: the rustdoc now labels
the public API experimental, and tests prove live pending membership remains
cancellable after preview, including a cancelled-only queue. Rebased Kairos
checks passed: 51 `kairo-ecs-core` tests, the focused Track 106 tests, formatting,
rustdoc, shared fixture validation, Conductor phase gates, DAG, artifact and
track coverage validators. Current PR check readback remains authoritative for
hosted status.

## Remaining gates

1. Record the three reviewer-agent reports and coordinator dispositions on the
   exact candidate commit.
2. Confirm all hosted CI jobs complete successfully on the final PR head.
3. Record final reviewer-agent assessments and the single maintainer's disposition of
   the CareOps candidate lock delta. Candidate-lock tests passing alone do not
   authorize a parent change.
4. Only after these gates, consider a separately reviewed CareOps pin and lock
   update, then run final consumer acceptance and closeout.

## Parent repository boundary

No Track 106 acceptance, merge, release, or CareOps parent pin/lock update is
claimed. The PR remains unmerged. The user is the single maintainer and final
integrator; reviewer-agent reports provide the technical review evidence.

## Follow-up issues

Final reviewer-agent reports, hosted checks, and the single maintainer's
disposition of the consumer dependency-lock delta remain open. Parent pin and
lock must remain unchanged until all are resolved.

## Integration notes

The proposed API candidate is PR #333 at the head recorded above. CareOps
consumer validation used an isolated disposable checkout and candidate lock;
the original pin and lock were restored after the check.

## Phase closeout evidence

- `$conductor-review`: local source review completed. Reviewer-agent reports on
  `89e713e` returned; the three reported improvements are applied, and final-head
  reviewer-agent confirmation remains pending.
- Accepted fixes: experimental maturity rustdoc; live-pending/cancelled-only and
  `Some(EntityId)` preview assertions; parsing/asserting fixture ordering and
  requirement; ADR names `crates/kairo-ecs-core` as the affected API root.
- Commit SHA: implementation candidate `29abb88718fb1e76b7dedf81766f7d49058dc5d2`; documentation reconciliation `fec06b928845b44352a2ac1edcf3c81712b3175a`.
- Pushed ref: `refs/heads/codex/r10-3-scheduler-preview-compat-20261025` contains both commits.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: passed after the documentation reconciliation commit.
- Next-phase decision: record the three reviewer-agent reports, complete hosted checks and lock-delta acceptance; do not advance CareOps parent pin.
