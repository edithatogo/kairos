# H0 scoped ownership handoff — 3 October 2026

This corrective handoff records the cross-track dependencies of PR #192. The Kairos owner requested single-maintainer harness and context engineering. Existing completed tracks remain completed; this additive internal-tooling change does not reopen their historical acceptance.

## Artifacts and bounded roles

- Track 27 / dx-agent: `scripts/agent_session.py`, `tests/agent_harness/test_agent_session.py`, and `conductor/harness/single-maintainer.md` (new developer tooling and documentation).
- Track 13 / ci-agent + security-agent: `.github/workflows/agent-harness.yml` and its inventory entries in `ci-policy.yml` and `workflow-security.yml`.
- Coordinator, under the current owner directive: root `AGENTS.md` protocol link, `CHANGELOG.md` internal-tooling note, and this handoff. These are cross-track foundation/governance documentation dependencies, not transfers of general path ownership.

Reviewed code head: `3fdcb50af36e35ad1fcb80613fbd71281e30a4cf`.
Tool SHA-256: `ab4cf0ad5d4ff124ef6dfba9bcebe74970b56b69d360be47977db5a020e3d3cf`.
The parent CareOps Sim implementation is merged in PR #8. Its currently reviewed engine development pin is preserved; upstream-main harness integration into that pin remains a separate coordinated dependency.

## Gates and contracts consumed

Consumes the existing Conductor ownership map, Track 27 agent contract, Track 13 handoff rules, and single-writer/determinism boundaries in root guidance. No public Rust API, C ABI, Arrow semantics, package, publication or compatibility contract changes.
Existing bootstrap-smoke and toolchain-docs gates are unchanged. Adds a focused 17-case developer-tool regression workflow on Ubuntu x86_64 and macOS arm64. This is H0 verification, not model qualification or evidence of completed H1–H4. Existing repository quality and security gates remain required.

## Executed verification and review

Coordinator working directory: `/private/tmp/kairos-harness-20261003`.
- `python3 -m unittest discover -s tests/agent_harness -v`: exit 0, 17 cases; local log `/tmp/kairos-harness-final-unittest-20261003.log`.
- `node scripts/validation/validate-track13-metadata.mjs`: exit 0, 62 tracks validated.
- `actionlint .github/workflows/agent-harness.yml`: exit 0.
- Actual claim/check/snapshot/release smoke at the reviewed head: exit 0; lease ID `c9dfce314b2c4d29b2cea39b402863e9`. No token committed.
- Hosted checks re-fetched by coordinator for the exact reviewed head: all successful or expected Codecov skips. Conductor checks run `37089568445` succeeded; session suite passed on both hosts. See [PR #192 checks](https://github.com/edithatogo/kairos/pull/192/checks).

Bounded agent panel reviews are code reviews, not independent human approvals or authenticated model qualification. Track 13 CI and Track 20 security reviewers accepted the exact reviewed surface with no correction. Track 27 reviewer found no code defect; final handoff disposition is recorded below after review of this document.

## Risks and follow-ups

H0 is a cooperative protocol, not a sandbox. Recovery asserts stopped ownership; it does not kill a process. Git-local receipts are not authenticated or externally immutable. Separate repositories require separate reservations. Windows runtime support and automated dispatch/model qualification are unverified and deferred to H1–H4 in the guide.

The Amazon Q subprocess comment is not a code defect: CLI `main()` catches `CalledProcessError`; context-manager unwinding closes the lock file and releases its lock. A failed check intentionally retains the reservation rather than silently handing ownership away.

## Contributor commands

Use `python3 scripts/agent_session.py --help` for the exact claim/check/heartbeat/release/recover/snapshot commands. Use the guide's bounded task, tracked input and byte-budget examples. Check actual changed paths before committing, then release immediately after the single commit. Do not infer authority from a generated snapshot.

## Final scoped disposition

3 October 2026: Track 27 dx-agent panel reviewer accepted the three bounded developer-tool paths at the reviewed code head after reading this handoff; no code correction requested. Track 13 CI and Track 20 security acceptance is recorded above. Coordinator verified hosted results independently. The ownership record gap is resolved; normal repository checks and merge policy still apply.
