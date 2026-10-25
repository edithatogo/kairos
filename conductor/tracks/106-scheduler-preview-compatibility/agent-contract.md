# Track 106 ownership contract

The coordinator owns only the paths listed in `spec.md`. The implementation is
bounded to one public projection, one scheduler observer, one shared fixture
registration and one focused consumer test. No change to scheduler ordering,
dispatch or timing is authorized.

The existing core-scheduler, conformance, and API Governance owners retain
review authority over changes corresponding to Tracks 01, 12 and 25. The user's
recorded role is CareOps consumer owner for the R10.3 downstream acceptance
gate; that does not replace Kairos code-owner review.

Use the isolated worktree and repository-wide advisory claim. Keep the candidate
base exact, retain test-first evidence and run focused Rust and fixture gates.
Do not edit the CareOps submodule pin, push, publish or claim owner approval.

## Required handoffs

- Core scheduler owner: review projection semantics and guarantee no dispatch,
  time, counter or live-pending mutation.
- Conformance owner: review fixture vocabulary and manifest registration.
- API Governance owner: review additive compatibility classification and ADR.
- CareOps consumer owner: verify exact R10.3 build/tests against the candidate.

If any reviewer is unavailable, record the exact pending gate in `handoff.md`;
do not infer acceptance from the local test result.
