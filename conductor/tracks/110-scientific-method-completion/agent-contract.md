# Agent contract

## Owner

vv-uq-agent + game-theory-agent + ml-integration-agent. Root coordinator integrates the approved 7 November packet; original capability owners retain source authority outside each bounded handoff.

## Write scope

This track directory and the explicitly claimed implementation slice in crates/kairo-ecs-game-theory/, crates/kairo-ecs-ml/, crates/kairo-ecs-rng/. Read AGENTS.md, source contracts and the selected phase before claiming. Use isolated worktrees, scripts/agent_session.py claim/context/check, and release immediately after the single bounded commit. No shared-checkout parallel writers.

## Required evidence

- S01–S08: Use the named independent analytical/game/QMC/statistical oracles in scientific-completion.md, including degenerate and boundary cases.
- S09–S10: Nonidentity outputs and real state transitions originate in Rust; scalar/batched equivalence, reset/seed/termination and lifetime tests.
- S11–S17: Actual devices/transports/external FMUs/filesystems or explicit capability-unavailable result; no mock or fixture metadata accepted as native proof.
- Scientific claims: Every supported method has a measured residual/error criterion and held-out test; classify each unsupported method explicitly.

## Handoff and review

Return exact commit, changed paths, actual commands/exits, remaining risks and unsupported claims. A different reviewer checks semantics and negative cases. Metadata validation, simulated hardware or tests that only read expected values cannot certify implementation. Never advance a consumer pin or publish an artefact as a side effect of a local pass.

