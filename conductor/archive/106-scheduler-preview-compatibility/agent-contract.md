# Track 106 ownership contract

The coordinator owns only the paths listed in `spec.md`. The implementation is
bounded to one public projection, one scheduler observer, one shared fixture
registration and one focused consumer test. No change to scheduler ordering,
dispatch or timing is authorized.

The existing core-scheduler, conformance, and API Governance owners retain
review authority over changes corresponding to Tracks 01, 12 and 25. The user's
recorded role is CareOps consumer owner for the R10.3 downstream acceptance
gate; that is separate from reviewer-agent assessment of Kairos core, conformance,
and API compatibility requirements.

Use the isolated worktree and repository-wide advisory claim. Keep the candidate
base exact, retain test-first evidence and run focused Rust and fixture gates.
Do not edit the CareOps submodule pin, push, publish or claim owner approval.

## Required handoffs

- Core scheduler reviewer agent/subagent: assess projection semantics and verify no dispatch,
  time, counter or live-pending mutation.
- Conformance reviewer agent/subagent: assess fixture vocabulary and manifest registration.
- API compatibility reviewer agent/subagent: assess additive compatibility classification and ADR.
- CareOps consumer validation: run and record the exact R10.3 build/tests against
  the candidate; the single maintainer integrates the result without a separate
  consumer-owner review seat.

Record exact candidate SHA, reviewer role, findings, and coordinator disposition
for each lens in `handoff.md`; do not infer review acceptance from local tests.
