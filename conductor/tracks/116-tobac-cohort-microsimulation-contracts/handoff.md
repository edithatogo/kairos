# Handoff

Updated 25 November 2026 (Australia/Brisbane). The provider contract and
standalone fixture are qualified. The installed Tobac candidate passed locally.

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

## Standalone Arrow IPC output — 23 November 2026

The [standalone fixture](../../../tests/fixtures/cohort-consumer/README.md)
emits individual Arrow IPC streams for the baseline cohort, paired policy cohort
and spatial exposures. It projects the declared C Data struct fields directly
into flat IPC columns. The emitted schema retains the physical types, column
requiredness, tick units and cohort RNG metadata.

The independent reviewer checked the projection and accepted the explicit
stdout flush after the Arrow end marker. The locked fixture suite passed seven
tests with Rust 1.99.1 on macOS ARM64. PyArrow 26.0.1 read 3,323 baseline
transitions, 3,968 policy transitions and two exposure rows. The two arms
matched on every field in the 1,451 pre-intervention transitions. The
independent readback receipt SHA-256 is
`202ff0b94228e6c782cbdda887af9eb65fee1497f40d6b03ac805ebd08cf5ab5`.

The installed `tobac` 0.2.0 importer at
`fe06ef5958ba0f218317a738fe8de95973300a12` read its own positive Evidence
control and rejected each neutral stream with its exact-schema error.

## Installed Tobac candidate — 24 November 2026

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the isolated receiving
checkout `/private/tmp/kairos-track116-tobac.sr6RjG/tobac` on
`codex/kairos-cohort-admitted-20261124`. The source lane covers
`src/tobac_supply/kairos_cohort.py`, its regression tests and their exact
publication inventory entries. The existing Tobac integrator owns protected
main and signed native acceptance. [Receiving PR #202](https://github.com/edithatogo/tobac/pull/202).

A fresh macOS ARM64 Python 3.15.2 environment installed Tobac 0.2.0 from
`ce9b621fbd5c2123a695a83935f1462116d7401c`. Wheel SHA-256
`663e701587a7f8d0ba60cb02ea1b656256345f2884517289ab8e5bdcac7b3816`.
The installed reader consumed the three fictional streams from Kairos
`1e267e6c352277d8a3b2b13f019ee82b5aca8a69`. Both cohort summaries
accounted for 4,096 members, including those with no recorded transition.
The arms matched all 1,451 pre-intervention rows and all 3,323 shared RNG
addresses. Polars retained the Arrow UInt64 RNG buffer after the input
reader and table were released. The independent review covered full-width
ticks, fixed strata and an inclusive cutoff.

CodeQL and public boundary checks passed on the receiving PR. Its three
native jobs stopped at signed inventory verification with
`Scientific inventory differs from native acceptance`. The receiving
owner needs to renew the inventory for the added module before rerunning
the checks and recording receiving-main acceptance.
[Receiving Actions run](https://github.com/edithatogo/tobac/actions/runs/45377109838).

## Signed inventory renewal — 25 November 2026

The existing verifier's `inventory_identity` reproduced both hashes from
Git-free archives of Tobac main `fe06ef5958ba0f218317a738fe8de95973300a12`
and candidate `ce9b621fbd5c2123a695a83935f1462116d7401c` using CPython
3.15.2. The signed inventory is
`38102d5614683c0a65e260538b6823f58b4ed6365c8fe651e737c69e40d37dd7`.
The candidate inventory is
`5b8432a3cfd9632b0d7bc3a13a50b644acfe52f28906a1214d60c52f2e6500f9`.
The source addition is `src/tobac_supply/kairos_cohort.py`, SHA-256
`50965ab64af7eb8dd995c2e2c7ec5f63b24699fccd61e895eec282d53a57fe0d`.
Every existing signed input pin still matches. The retained local installed
receipt SHA-256 is
`831b14570b3c78d62266fc9acfc0becf60063eeaedff1f5600b475714c59f14b`.

PR #202 awaits inventory renewal by the signing and main-integration owner
recorded in Tobac's workstream. The next receiving operation binds the
candidate membership to the accepted signer, verifies the signed capsule on
the receiving head, then runs all of its required hosted lanes. The local
wheel result remains the prior candidate receipt. After publication, verify
the receiving-main revision and evidence before closing the receiving gate.
[Tobac #202](https://github.com/edithatogo/tobac/pull/202) ·
[Tobac signing ownership](https://github.com/edithatogo/tobac/blob/fe06ef5958ba0f218317a738fe8de95973300a12/docs/WORKSTREAM_SYNC.md#conductor-and-ci-simplification--22-november-2026)

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
| next-phase decision | Accept provider Phases 0–2 and record the locally passing Tobac reader. Keep receiving publication open until signed inventory verification and hosted qualification pass. |

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
