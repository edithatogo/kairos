# Handoff — 16 Release Governance & Maintenance

Last updated: 2026-05-09

## Summary

Implemented the minimal R2 release-governance slice: changelog policy/check,
compatibility and deprecation release rules, release evidence requirements, and
maintenance handoff. The slice stays aligned to Track 15's dry-run packaging
posture and Track 25's compatibility boundary.

## Files changed

`conductor/tracks/16-release-governance-maintenance/plan.md`
`conductor/tracks/16-release-governance-maintenance/test-matrix.md`
`conductor/tracks/16-release-governance-maintenance/handoff.md`
`conductor/maintenance-governance.md`
`CHANGELOG.md`
`.github/workflows/changelog-policy.yml`
`docs/release/release-governance.md`
`docs/release/changelog-policy.md`
`docs/release/compatibility.md`
`docs/release/maintenance-handoff.md`
`docs/release/maintainer-rotation.md`
`docs/release/release-checklist.md`
`docs/release/release-notes.md`

## Contracts consumed

`conductor/workflow.md`
`conductor/release-engineering.md`
`conductor/delivery-readiness-checklist.md`
`conductor/quality-gates.md`
`conductor/contracts/versioning-compatibility.md`
`conductor/tracks/15-packaging-publishing-delivery/handoff.md`

## Contracts changed

None.

## Tests added

The track now uses explicit file-existence checks, required-text checks for the
R2 governance docs, the changelog policy static check definition, and the
conductor setup validator as its baseline gate.

Focused offline validator:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
```

## Known risks

The changelog policy is now enforced by `.github/workflows/changelog-policy.yml`,
but the PR diff matcher should be expanded if new public release surfaces are
added.
Release process changes still depend on the GitHub workflow files and registry
conventions staying consistent with the release-engineering notes.
Package publication remains blocked until Track 15 clears naming, registry, and
dry-run evidence.

## Integration notes

Track 15 should continue to own package paths and production publish enablement.
Track 16 owns the release-governance evidence and should be consulted before a
publish job is enabled. Tracks 13, 20, 25, and 28 should treat this track as the
source of release-governance expectations.

## Review-hardening update

Added a track-local release-governance validator that checks the changelog,
compatibility, deprecation, release-note, maintenance handoff, maintainer
rotation, `compatibility-policy`, and `changelog-check` claims against
checked-in docs and the central gate registry.

## Follow-up issues

Expand the changelog-policy workflow matcher when new public release surfaces are added, and keep release workflow dry-run posture aligned with Track 15 packaging gates.
## Phase closeout evidence

Review pass on 2026-05-09:

- `$conductor-review` findings: no blocking Track 16 implementation findings in the owned release-governance docs, changelog policy, validator, or maintenance governance surface.
- Accepted fixes: updated Track 16 evidence to record the current shared-worktree changelog-policy blocker.
- Deferred or blocked fixes: the local changelog-policy diff check fails because `bindings/julia/src/KairoECS.jl` and `bindings/julia/test/runtests.jl` are modified without a matching `CHANGELOG.md` diff. Those files are outside Track 16 ownership, so the fix is deferred to the Julia-binding owner or coordinating closeout.
- Validation commands:
  - `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\16-release-governance-maintenance\validate-release-governance.ps1` (passed)
  - `node tests\conformance\track12_20_evidence_check.mjs` (passed)
  - `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` (passed)
  - local changelog-policy diff check from `docs/release/changelog-policy.md` (failed on the Julia binding public-surface diff)
- Done eligibility: Track 16 is review-ready on its owned surfaces, but strict shared-worktree release-governance closeout is blocked until the cross-track Julia binding changelog-policy failure is resolved or waived.

Implementation refresh on 2026-05-09:

- Track status advanced from `In Progress` to `In Review` after the previously recorded cross-track phase-gate blocker cleared.
- Accepted fixes: registry/status reconciliation only; the existing release-governance implementation, maintainer-rotation output, and validator hardening remain unchanged.
- Deferred or blocked fixes: no in-scope Track 16 implementation blocker remains. Commit and push evidence are not recorded by this worker because parallel agents are active in the shared worktree.
- Validation commands:
  - `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\16-release-governance-maintenance\validate-release-governance.ps1`
  - `node tests\conformance\track12_20_evidence_check.mjs`
  - `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1`
- Commit SHA: pending; no Track 16 commit created in this shared-worker pass.
- Pushed ref: pending; no push performed.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: not run because this implementation pass intentionally leaves Track 16 registry/handoff updates for the coordinating closeout.
- Next-phase decision: Track 16 is ready for review.

Implementation/review pass on 2026-05-08:

- `$conductor-review` findings: no blocking Track 16 findings after the maintainer-rotation output and named gate assertions were added.
- Accepted fixes: added `docs/release/maintainer-rotation.md` with a preview maturity label, wired it into release governance/checklist/handoff docs, and hardened `validate-release-governance.ps1` to prove `compatibility-policy` and `changelog-check` are present in Track 16's `conductor/tracks.yaml` gate block and `conductor/quality-gates.md`.
- Deferred or blocked fixes: no Track 16 blocker remains for `compatibility-policy` or `changelog-check`. The shared phase-closeout validator is blocked by Track 19 handoff evidence outside Track 16 ownership.
- Validation commands:
  - `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\16-release-governance-maintenance\validate-release-governance.ps1`
  - `node tests\conformance\track12_20_evidence_check.mjs`
  - `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` (blocked by Track 19 handoff evidence, not Track 16)
- Commit SHA: blocked; no commit created in the shared worktree.
- Pushed ref: blocked; no push performed.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: not run because shared phase-closeout is already blocked and the worktree is not clean.
- Next-phase decision: keep Track 16 `In Progress` until the unrelated Track 19 phase-closeout evidence blocker is resolved or waived; Track 16's compatibility and changelog gates are locally satisfied.

## Cross-track handoff — PR #170 (2026-09-29)

- Track owner: `release-agent`; requested review: validate the changelog entry for the bootstrap runtime minimum and its release-facing wording.
- Artifact: `CHANGELOG.md` records that the pinned bootstrap npm CLI now requires Node.js >=22.9.0. No release workflow, compatibility policy, package publication, or version claim changed.
- Contracts consumed: Track 27's bootstrap runtime declaration and the repository Node toolchain matrix.
- Risks and follow-up: Dependabot alerts #64–66 remain unresolved upstream in npm's bundled dependencies; this PR does not represent them as fixed. Any later npm CLI update that addresses them should be separately recorded and assessed against bootstrap support policy.
- Validation: on 2026-09-29, in `/private/tmp/kairos-bootstrap-node-security` at base `0c77e8909fc7cbf7c628cdda6350177cb9e88e8a` plus the PR worktree, `pwsh -NoProfile -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1` passed (exit 0; changelog and compatibility gates reported ok). The Track 16 changelog-policy rule is not triggered by this set of changed paths; the hosted changelog-policy check will provide the fresh PR-diff result. The bootstrap `npm audit` command exits 1 for unresolved #64–66; no security fix is claimed.
- Owner acceptance: accepted by the Track 16 reviewer on 2026-09-29 at PR head `4d113b5fc7e6efaede703273075b00573d5b99f1`. Scope is the PR #170 `CHANGELOG.md` entry and this Track 16 handoff only. Evidence: reviewer acceptance recorded for these two artifacts; no GitHub review ID is available in PR metadata. This does not resolve the separate #64–66 security advisories.

## Cross-track review — PR #180 (2026-09-30)

- Track 16 release-agent disposition: retain a concise `Changed` entry because PR #180 changes the Track 06 Python binding surface, which the changelog policy names as requiring an entry. The wording records the observable import-time annotation behavior and explicitly states that public package behavior and the supported Python range are unchanged. This is a maintenance note, not a claim of a public API or support change.
- Artifact and scope: PR #180, `https://github.com/edithatogo/kairos/pull/180`, reviewed source head `68c1c7ca96bf612360fb7868418ee730ab6e3a17` on base `384e8546d69f9cbf2746fcb2ab646263256e6dec`. Track 06 changes are limited to `bindings/python/kairo_ecs/_ffi.py` and `bindings/python/kairo_ecs/_scheduler.py`; Track 16 changes are limited to the corresponding `CHANGELOG.md` note and this handoff record.
- Contracts and dependencies: Track 06 owns `bindings/python`; `conductor/subagents.yaml` assigns `CHANGELOG.md` to the release-agent. This records the cross-track release review and does not advance Track 06 status, waive its gates, or assert human owner approval.
- Hosted validation at source head `68c1c7ca96bf612360fb7868418ee730ab6e3a17`: the PR checks reported success for `Enforce changelog policy` (run `36601160880`), Python 3.10–3.14 (run `36601160872`), Python package dry-run (run `36601160819`), `docs-quality` (run `36601160855`), Rust stable/MSRV/Wasm verification and Rust core quality (run `36601160936`), CodeQL (runs `36601160844` and `109518953284`), fixture validation (`36601161014`), dependency review (`36601160829`), secret scan (`36601160782`), repository health (`36601160783`), and CI skip guard (`36601160825`). Codecov OIDC upload was skipped; no coverage result is claimed from that job.
- Validation on 2026-09-30 in `/private/tmp/kairos-python-import-cleanup`: `pwsh -NoProfile -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1` passed (`track16_status=ok`, `release_governance=offline-doc-gate`, `compatibility_policy=ok`, `changelog_check=ok`); the Track 16 PR-diff check implemented from `docs/release/changelog-policy.md` passed (`changelog_diff_check=ok`, public changes were the two Python modules, and `CHANGELOG.md` was present); and `git diff --check` passed. These checks validate Track 16 policy structure and changed-path coverage, not human release approval.
- Owner acceptance: this is the release-agent's scoped disposition for the changelog wording and Track 16 handoff only. No human approval is represented or inferred from automated checks or Codex review.
