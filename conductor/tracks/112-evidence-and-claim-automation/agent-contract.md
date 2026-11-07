# Agent contract

## Owner

conformance-agent + docs-agent + ci-agent. Root coordinator integrates the approved 7 November packet; original capability owners retain source authority outside each bounded handoff.

## Write scope

This track directory and the explicitly claimed implementation slice in conductor/capabilities.json, conductor/capabilities.md, scripts/validation/, tests/conformance/. Read AGENTS.md, source contracts and the selected phase before claiming. Use isolated worktrees, scripts/agent_session.py claim/context/check, and release immediately after the single bounded commit. No shared-checkout parallel writers.

## Required evidence

- Integrity: Changing a local result byte makes its digest check fail.
- False claims: Synthetic/local-placeholder data cannot satisfy a measured speedup claim; source-only evidence cannot qualify native execution.
- Drift: Generated capability table must match its JSON; dangling evidence and track IDs fail.
- Scope: Validator explicitly states it validates evidence structure, not the truth of external experiments or new scientific results.

## Handoff and review

Return exact commit, changed paths, actual commands/exits, remaining risks and unsupported claims. A different reviewer checks semantics and negative cases. Metadata validation, simulated hardware or tests that only read expected values cannot certify implementation. Never advance a consumer pin or publish an artefact as a side effect of a local pass.

