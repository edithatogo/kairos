# Agent contract

## Owner

ffi-agent + api-governance-agent. Root coordinator integrates the approved 7 November packet; original capability owners retain source authority outside each bounded handoff.

## Write scope

This track directory and the explicitly claimed implementation slice in bindings/, packaging/, python/kairo_gym/. Read AGENTS.md, source contracts and the selected phase before claiming. Use isolated worktrees, scripts/agent_session.py claim/context/check, and release immediately after the single bounded commit. No shared-checkout parallel writers.

## Required evidence

- Retirement: Every old host-engine constructor fails before state/time mutation, including with a fallback environment variable set.
- Native execution: Build the Rust library/Wasm artefact; installed test schedules/cancels/runs and verifies native event counts and error paths; missing artefact fails the native lane.
- Lifetime: Double-close, use-after-close, invalid handles, ABI mismatch, buffer ownership and overflow cases reject.
- Replay: Actual native stream generation and event traces match versioned fixtures; reading expected-vector length is insufficient.

## Handoff and review

Return exact commit, changed paths, actual commands/exits, remaining risks and unsupported claims. A different reviewer checks semantics and negative cases. Metadata validation, simulated hardware or tests that only read expected values cannot certify implementation. Never advance a consumer pin or publish an artefact as a side effect of a local pass.

