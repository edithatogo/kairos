# Agent contract

## Owner

release-agent + api-governance-agent + security-agent. Root coordinator integrates the approved 7 November packet; original capability owners retain source authority outside each bounded handoff.

## Write scope

This track directory and the explicitly claimed implementation slice in packaging/, docs/release/, conductor/release-gates.md. Read AGENTS.md, source contracts and the selected phase before claiming. Use isolated worktrees, scripts/agent_session.py claim/context/check, and release immediately after the single bounded commit. No shared-checkout parallel writers.

## Required evidence

- Beta: Every enumerated required gate has candidate-specific evidence or the capability is excluded.
- Installed packages: Tests execute distributed artefacts in clean advertised environments, not only editable source.
- Stable: Independent reproduction, upgrade/replay fixtures, supported limits and no unresolved critical/high defects in supported scope.
- Publication: Actual tag/registry/provenance readback is distinct from CI or PR success.

## Handoff and review

Return exact commit, changed paths, actual commands/exits, remaining risks and unsupported claims. A different reviewer checks semantics and negative cases. Metadata validation, simulated hardware or tests that only read expected values cannot certify implementation. Never advance a consumer pin or publish an artefact as a side effect of a local pass.

