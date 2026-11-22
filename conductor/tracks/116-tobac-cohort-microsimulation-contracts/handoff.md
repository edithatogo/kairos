# Handoff

Updated 23 November 2026 (Australia/Brisbane). The provider contract and
standalone fixture are qualified.

## Provider and standalone fixture — 23 November 2026

## Summary

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the bounded provider change
in the managed `kairos-cohort-contracts` worktree on
`codex/cohort-native-export`.

## Files changed

The Arrow module, independent Cargo workspace and existing Arrow platform
workflow deliver and validate the exports.
[Standalone fixture](../../../tests/fixtures/cohort-consumer/README.md).

## Contracts consumed

The fixture uses Track 69 hazards, the ABM spatial index and the shared
trusted Arrow C Data importer.

## Contracts changed

The opt-in schema and replay contracts are accepted by
[ADR 0015](../../../docs/adr/0015-cohort-and-exposure-schemas.md) and documented
in the [API review](../../../docs/design/track116-cohort-export-api-review.md).

## Tests added

Rust 1.99.1 on macOS ARM64 ran five locked tests and the executable from a
Git-free extracted tree `04dff29ffa3390b049b32e322860eb29e083602d`. Both
commands exited zero. The executable returned 3,323 baseline, 3,968 paired
policy and two spatial rows. It checked the effective primitive buffer address
at a nonzero stream offset after releasing the frame and reader.

Independent review identified two gaps. The implementation now counts visits
per entity and origin state, and its paired trace proves the shared sampled exit
after one arm takes an extra intermediate transition. The convergence case
compares imported trajectories from 100,000 entities to independent analytic
incidence and survival probabilities at 2 and 6 seconds. Re-review accepted both
fixes. The extracted archive and fixture lock SHA-256 values are
`88dfa8c7bf3adb741d40b86d98453661b34ec2c5483709f777214e3e6a8933db` and
`cfe3cd69b39eaace6bb4fef031c9d1567356d3768444944ce81d75f0c9ea4072`.
Published implementation commit `51d718504c05d7c1198ef2048db89883f2cd94e0`
has that exact tree.

The `tobac` handoff is the two versioned fields, typed metadata and runnable
fixture. The receiving transport preserves its own provenance and input
metadata. Receiving adapter qualification and pin selection belong to the
application repositories. [tobac interchange](https://github.com/edithatogo/tobac/blob/cf0cff9260c0c168c99342a4f71a565ecea6c6d5/src/tobac/interchange.py).

## Prior registration

The 10 November 2026 registration coordinated Kairos with
`edithatogo/tobac` under `dependency_ecosystem_codesign_20261110`. Its
dependency records remain in Tracks 01, 03, 04, 69, 70 and 114 and
[ADR 0011](../../../docs/adr/0011-ecosystem-boundaries.md).

The [cohort API review](../../../docs/design/track116-cohort-export-api-review.md)
defines the contracts changed. The [test matrix](test-matrix.md) records
schema, simulation and installed fixture checks.

## Known risks

Uncoupled RNG substreams under variable event counts can inflate Monte Carlo variance between policy arms; domain terminology must not leak into neutral Kairos crates. Both risks are bound to explicit negative and convergence gates in `risk-register.md` and `test-matrix.md`.

## Follow-up issues

The receiving applications own their installed round-trips and summary
qualification.

## Integration notes

Use the versioned schema metadata and locked fixture for the receiving
adapter. Tobacco/vaping epidemiology and policy inputs belong to `tobac`.

## Phase closeout evidence

| Evidence | Result |
|---|---|
| `$conductor-review` | Independent review and re-review accepted the fixture. |
| accepted fixes | Source-state visit addressing and a convergence check on imported results. |
| commit SHA | [PR #420](https://github.com/edithatogo/kairos/pull/420) merged the reviewed provider as `6955d3f85c71b21889a6d3aec1ca006a436ed4bd`. |
| pushed ref | `origin/main`. |
| `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Required after pushing the closeout update. |
| next-phase decision | Accept provider Phases 0–2 and hand off installed adapter qualification. |

## Registration repair — 14 November 2026

At GitHub main `aeb52c937f1ffdc01e3dc5064cceae5134bf2790`, the existing
phase-gate validator reported six missing handoff evidence fields and three
missing test-matrix requirements. The same nine errors were reproduced locally
with PowerShell 7.7.0. This repair adds the pending evidence fields above and
names the existing phase/clean-source gates in `test-matrix.md`.

Only those two documentation files change. The validator, scientific tasks,
registry status, Rust-only execution policy, ownership boundaries and consumer
pins are unchanged. Candidate validation and exact-head hosted results belong
to the repair PR; they do not close an implementation phase.
