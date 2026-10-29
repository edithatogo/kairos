# Track 107 handoff — 2026-10-27

## Registration reconciliation — 29 October 2026

Only the eight planning and handoff records are imported from frozen PR #334
head `56ac7691946d8cfc08f4d601e91ba4ea5547e886`. Runtime code, tests, ready
fixtures, pins and acceptance are not imported by this registration packet.
The registry records Blocked and the metadata accepts no phases. Historical
candidate claims below apply to that source lane; its
[raw evidence](https://github.com/edithatogo/kairos/tree/56ac7691946d8cfc08f4d601e91ba4ea5547e886/conductor/tracks/107-careops-staged-flow-compatibility/evidence)
remains at the immutable source. PR #334 and the zstd audit must pass, followed
by separate CareOps acceptance, before promotion. This does not close a phase.

## Summary

Local candidate adds experimental `StagedResourceRuntime` for the narrow staged allocation contract. CareOps consumer migration and parent acceptance remain separate.

## Files changed

See `git status --short`; exact paths are declared in `spec.md` and the Track107 registry entry.

## Contracts consumed

Kairos scheduler ordering, DES/core/types APIs, conformance fixture contract and CareOps staged allocator test at parent commit `87439c3`.

## Contracts changed

Adds the protected experimental Rust API root `crates/kairo-ecs-des` to compatibility governance and defines the staged request/lease lifecycle. It does not change existing scheduler or RNG contracts.

## Tests added

Four consumer-shaped tests cover capacity-one FIFO contention, scheduled release promotion, invalid time/capacity, foreign runtime handles, and wide capacities. A versioned fixture is registered using the validator-supported bootstrap JSON form.

## Known risks

API remains experimental. The full removed `FlowRuntime` surface is not restored. CareOps must migrate the test and pass candidate locked acceptance before pin adoption.

## Follow-up issues

CareOps R10.3 consumer migration, parent cargo lock/readback, full ED compatibility, CLI/benchmark gates, and parent acceptance remain outstanding.

## Integration notes

Candidate base is `d3a6d831ddbe5886078541468e3fbf08b6f8c4c9`, a Track106/SplitMix descendant. Parent pin was not modified. Reviewer-agent API design review completed with accepted fixes applied and no remaining focused findings.

## Phase closeout evidence

- `$conductor-review`: reviewer-agent design/API review completed; implementation review and fixes applied.
- accepted fixes: duplicate-release guard, fixed revision, past-time validation, same-time ordering, namespace isolation/redaction, portable capacity comparison, and fixture consumer scope.
- commit SHA: pending; candidate remains uncommitted.
- pushed ref: pending; candidate remains local.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pending until commit/push and cleanup.
- next-phase decision: hold Track107 closeout until reviewed implementation gate, commit/push evidence, and separate CareOps R10.3 consumer acceptance.
