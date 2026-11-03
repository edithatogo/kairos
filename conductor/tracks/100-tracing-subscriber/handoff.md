# Handoff

Updated: 2026-10-22. Maturity: planned; adoption decision pending.

## Summary

Separate dependency track registered. No runtime change, installation or accepted implementation phase.

## Files changed

Planning folder and coordinated registry/policy surfaces only.

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

Prospective replacement boundaries only; no runtime API changed.

## Tests added

Independent future tests specified in test-matrix.md; planning validators only this pass.

## Known risks

Value and behavioural gates remain unresolved. Preserve published contracts and explicit owner boundaries; see the dependency-specific specification.

## Follow-up issues

Record exact existing consumer, baseline and adopt/no-go decision in Phase 0. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Pending: `$conductor-review`, accepted fixes, rejected findings, command/cwd/commit SHA/toolchain/seed/input hashes, exit codes/artifacts, pushed ref or blocker and next-phase decision. Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.

## Phase 0 entry decision — 1 November 2026

**Defer tracing-subscriber adoption.** The two entry conditions in the
[specification](spec.md) are accepted tracing instrumentation and an executable
owner with a real consumer. At Kairos main `085d132ac45203a2dd5aaf4dcff20f1b25e75c5c`,
neither is established. [Track 97 PR #354](https://github.com/edithatogo/kairos/pull/354)
merged an evaluation that deferred the instrumentation. The prototype identified
a missing GVT participant but spent at least 40 ms on the reducing thread with
a deliberately slow scoped subscriber, exceeding that experiment's 5 ms budget.
That measurement applies to the prototype callback; it is not a benchmark of
tracing-subscriber.

The Kairos CLI manifest and root Cargo.lock contain no tracing-subscriber
dependency. The CLI currently uses stdout for command output and stderr for
errors (crates/kairo-ecs-cli/src/main.rs). No executable owner has accepted
the proposed GVT emitter or supplied a queue capacity, drop policy and run-ID
contract. The test-only subscriber in the Track 97 patch does not establish
a production subscriber requirement. CareOps Sim's existing
100-tracing-subscriber handoff entry remains unresolved with impact review
pending. Its application bootstrap remains consumer-owned.

Registry readback identified 0.3.23 as the current stable, non-yanked release
(MIT, declared Rust 1.65.0). These are inventory facts. No crate was installed
or exercised for this decision; security, runtime cost and cross-platform
behavior have not been qualified for adoption. The plan permits a Phase 0
no-go without dependency installation. Implementation phases stay unchecked.

Reopen when a named executable owner accepts an emitter and defines bounded
dispatch under saturation. Then test filtering and private-field exclusion,
duplicate initialization, queue-full and shutdown behavior, byte-identical
machine stdout and simulation results, and enabled/disabled costs. Reuse an
existing host subscriber where sufficient. A library must not install a global
subscriber on behalf of a consumer.

Review cross-checked the merged Track 97 disposition against the CLI source,
manifest, root lock and downstream row. An earlier inference that merging a
qualification could satisfy the instrumentation prerequisite is explicitly
excluded. Rollback is confined to the track records.

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns the Track 100 directory
in /Users/doughnut/.codex/worktrees/track48-rollback/kairos on
codex/track100-subscriber-decision. Native scope: read-only source, registry
and PR readback. The integrator retains shared registry/phase ownership.

## Current readback — 3 November 2026

Chat `01a19bed-7060-7910-af22-38cb31f51446` owns this bounded handoff readback in `/Users/doughnut/.codex/worktrees/kairos-track98-review/kairos`, branch `codex/conditional-tracks-reconcile-20261103`, based on `d7bdcd3`. Shared registry and runtime changes remain with their owners.

The entry decision was integrated by [PR #355](https://github.com/edithatogo/kairos/pull/355), merged as `41e58e0c012e3f2253559cbefb239db71ee7486c`. The decision remains a deferral; no production subscriber or host consumer is accepted.
