# Track 111 specification: Measured performance and parallel execution improvement

## Overview and approved outcome

Remove demonstrated overhead, establish a measured serial/parallel/GPU crossover and publish reproducible matched-workload results.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../performance-programme.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: crates/kairo-ecs-pdes/, crates/kairo-ecs-state/, benches/. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 18, 31, 47, 48, 78, 101 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- Semantic identity: Serial and parallel executions consume the same event tapes and satisfy exact/tolerance-based contracts.
- Overhead: Thread creation counts and profiler traces show whether the targeted overhead was actually removed.
- Performance: Raw repeated wall-clock/RSS samples from named hardware and equivalent work; useful events exclude rolled-back work.
- Regression: Dedicated comparable runners evaluate declared thresholds; noisy shared-runner smoke is not performance certification.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

Batching or work stealing can alter causality and stochastic order. Correctness gates precede speed gates, and serial remains available where it wins.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).

