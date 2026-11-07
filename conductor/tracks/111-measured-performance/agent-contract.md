# Agent contract

## Owner

performance-agent + pdes-agent. Root coordinator integrates the approved 7 November packet; original capability owners retain source authority outside each bounded handoff.

## Write scope

This track directory and the explicitly claimed implementation slice in crates/kairo-ecs-pdes/, crates/kairo-ecs-state/, benches/. Read AGENTS.md, source contracts and the selected phase before claiming. Use isolated worktrees, scripts/agent_session.py claim/context/check, and release immediately after the single bounded commit. No shared-checkout parallel writers.

## Required evidence

- Semantic identity: Serial and parallel executions consume the same event tapes and satisfy exact/tolerance-based contracts.
- Overhead: Thread creation counts and profiler traces show whether the targeted overhead was actually removed.
- Performance: Raw repeated wall-clock/RSS samples from named hardware and equivalent work; useful events exclude rolled-back work.
- Regression: Dedicated comparable runners evaluate declared thresholds; noisy shared-runner smoke is not performance certification.

## Handoff and review

Return exact commit, changed paths, actual commands/exits, remaining risks and unsupported claims. A different reviewer checks semantics and negative cases. Metadata validation, simulated hardware or tests that only read expected values cannot certify implementation. Never advance a consumer pin or publish an artefact as a side effect of a local pass.

