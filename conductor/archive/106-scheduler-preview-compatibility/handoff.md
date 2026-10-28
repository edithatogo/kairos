# Track 106 handoff

PR #333 was squash-merged to `origin/main` as
`797e969b4a3cd3605a617a32d0134d0a093aa06a`. The closeout ledger identifies
that integrated commit; the review and test receipts below remain bound to
their original source commits, including `e3c7dc2`.

Last updated: 2026-10-26.

## Summary

The bounded scheduler preview implementation is on PR #333. It restores the
retained `ScheduledEventPreview` projection and `Scheduler::peek_next` observer
without changing dispatch behavior. The current PR head is
`e3c7dc2f9f66e3c980b9d3cfb631be31842368c5`, based
on `main` at `80063b7413a26013e265107ea462828d94a49849`.

## Current state

The PR is open and unmerged. The repository uses reviewer-agent/subagent
assessments for technical review; no separate human owner review is required.
Three reviewer agents assessed the integrated candidate at
`b9250f5e013a6f27d43f1e58e20c4be1ff0421fb` and passed from core scheduler,
conformance and API compatibility lenses. Earlier findings were fixed: tests
exercise `Some(EntityId)` and assert fixture ordering/requirement; the ADR names
the affected crate root. All 18 workflow runs on implementation head
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
  `9ad9d38b8fe43a10a90a02989c1c1adf83607a90d620a64242386398e1a671f3` and
  retained pin `44261eb5cf8275f132f63a3396903d116d701dbe`,
  `cargo check --locked --offline -p careops-ed-cli`, both R10.3 reference tests,
  and focused Rust 1.99.1 rustfmt passed.
- With a candidate-specific lock hash
  `24747479d6663d57aa4bacec437f60c652d78f07ec4c7a17883e98391ce2ffb8`,
  `cargo check --locked --offline -p careops-ed-cli`,
  `cargo test --locked --offline -p careops-ed --test r10_tabular_reference`,
  and Rust 1.99.1 rustfmt checks passed. The two R10.3 reference tests passed.
- The candidate lock hash
  `24747479d6663d57aa4bacec437f60c652d78f07ec4c7a17883e98391ce2ffb8`
  adds `libm 0.2.16`, `rand 0.10.3`, `rand_chacha 0.10.0`, `rand_core 0.10.1`,
  and `rand_distr 0.6.0`, and removes the stale direct `thiserror` edge from
  `careops-ed`; proptest continues using rand 0.9.5. Candidate source
  `e3c7dc2` compiled and both references passed with that lock. A reviewer-agent
  found the candidate pin also migrates Kairos RNG behavior from SplitMix64 to
  ChaCha8. CareOps arrivals and pathway sampling consume these streams, and the
  R10.3 references do not prove old-pin replay parity. Do not adopt this broader
  pin/lock delta in Track 106.
- The disposable CareOps checkout was restored to pin
  `44261eb5cf8275f132f63a3396903d116d701dbe` and the original lock. The parent
  CareOps pin and lock remain unchanged.

## Local review and checks

Local source review found and fixed two in-scope issues: the rustdoc now labels
the public API experimental, and tests prove live pending membership remains
cancellable after preview, including a cancelled-only queue. Rebased Kairos
checks passed: 51 `kairo-ecs-core` tests, the focused Track 106 tests, formatting,
rustdoc, shared fixture validation, Conductor phase gates, DAG, artifact and
track coverage validators. The current PR head's hosted required checks all pass.

## Track 106 closeout

Track 106 is archived in this PR branch after local review, three exact-source
reviewer-agent assessments, retained-pin CareOps consumer checks, and passing
hosted Actions at `e3c7dc2`. CareOps RNG migration and pin advancement remain
separate work; the parent pin and lock stay at their retained values.

## Parent repository boundary

The PR remains unmerged pending the archive-closeout commit's hosted checks. No
CareOps parent pin/lock update or release is included. The user is the single
maintainer and final integrator; reviewer-agent reports provide technical review.

## Follow-up issues

The CareOps candidate RNG migration needs separately scoped compatibility and
replay review before any parent pin/lock update. Track 106 does not authorize
that broader migration.

## Integration notes

The proposed API candidate is PR #333 at the head recorded above. CareOps
consumer validation used an isolated disposable checkout and candidate lock;
the original pin and lock were restored after the check.

## Phase closeout evidence

- `$conductor-review`: source review found no remaining in-scope defect. Three reviewer-agent assessments passed on source head `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb`; earlier findings were fixed. A separate consumer-lock review identified RNG migration risk, so the parent pin and lock remain unchanged.
- Reviewer-agent reports on exact source head `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb` passed for core scheduler, conformance and API compatibility. No separate human owner review is required under the single-maintainer workflow.
- Accepted fixes: experimental maturity rustdoc; live-pending/cancelled-only and
  `Some(EntityId)` preview assertions; parsing/asserting fixture ordering and
  requirement; ADR names `crates/kairo-ecs-core` as the affected API root.
- Source candidate commit SHA: `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb`; pushed ref: `refs/heads/codex/r10-3-scheduler-preview-compat-20261025`.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pending for the archive-closeout commit.
- Next-phase decision: merge Track 106 after the archive-closeout commit's required checks pass. Keep the CareOps pin unchanged; handle RNG/pin migration separately.
