# Handoff

Updated 8 November 2026 (Australia/Brisbane). Status: Spec Approved; no implementation phase is accepted by this registration.

## Summary

Ecosystem amendment: accepted separation of simulation mechanisms, UOGTO
semantics, generic Rust games, domain models, HEOR valuation and VOI. Existing
game incubator is reused; no new repository or pin is created. Movement and
resource use are explicitly separated from physical domain policy and money.
The next bounded implementation tranche and its checker are added without a
second task-status ledger. Scientific obligations remain open through extraction.

Reduce custom maintenance through qualified reuse and make Kairos a standalone dependency consumed by CareOps through explicit public contracts.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 01, 02, 04, 25, 70, 108.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

A convenient framework can alter identities, order or build footprint. Preserve public contracts; consumer pin changes require separate actual downstream qualification.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.


## Ecosystem amendment verification

At baseline ab9834d, the final bounded catalogue has 59 tasks. Fourteen focused
checker tests pass, including duplicate/cycle/path/feature-gate negative cases;
the existing nine frontier-evidence tests and 27-entry capability integrity check
pass. Changed JSON/local Markdown links, website source-link check and whitespace
check pass. Independent review identified missing feature flags and consumer
candidate/lock ordering; all findings were corrected and the reviewer rechecked
those fixes. No Rust/compiler/native method execution was performed locally.

Tooling used isolated Python 3.12.14 because the selected 3.15.2 interpreter is
not installed in this workspace; these are standard-library planning/tool checks,
not product runtime qualification. Hosted CI uses the declared repository profile.
The initial advisory claim expired during cross-repository work; the writer was
paused, the original diff preserved, and exact changed files copied to a clean
isolated checkout under a fresh claim. No shared history or lease policy changed.
The reviewed PR and hosted check results are the publication record. No phase,
consumer pin, scientific method or release is accepted by these structural checks.

## Separate game-library refinement

At Kairos baseline `018ef4975f7f146c9fa730381ac3c2bab8b15b60`, source inspection
maps game algorithms into the existing incubator, ECS-facing representations
into an optional game-owned adapter, and ontology mapping into semantic tooling.
Receiving coordination is issue 100/T17; Innovate G01–G04 remains its existing
consumer programme. No alternate solver, package/repository, runtime feature or
provider pin is created here.

The extraction map records source semantic differences and packaging hazards.
The execution-port design separates diagnostic alternatives from actual causal
events and defines lawful observation, stale-action, solver/simulation RNG and
replay requirements. The catalogue has 60 bounded leaves; K01.port must bind
these constraints to actual public APIs and exact fixture commands. S04 checks
now enable graph-relations; game/ontology inventory checks include graph and
generated/codegen features. All implementation/qualification leaves remain open.

Core, static-game and dynamic-adapter release paths are independently scoped;
optional game profiles cannot block runtime-only consumers. S01–S04 still need
receiving numerical acceptance and source caller/API disposition. NHRA is a
prospective consumer with pending handoffs, not a qualified Rust provider.

Local structural validation: task checker accepts 60 leaves with S01–S17
retained; all 14 checker tests pass; the 27-entry capability check and all nine
frontier-evidence tests pass. The checker first rejected an invalid hyphenated
new task ID; it was corrected to K01.port and the same checks rerun. Isolated
Python 3.12.14 and Node 24.22.1 support these local tooling checks because the
declared CI runtimes are unavailable here. No local Rust/native execution,
package qualification or phase closure is claimed; exact-head hosted results
and the reviewed PR remain the integration record.

Independent read-only review found no blocking issue in the dependency paths,
source-semantic map, feature commands or obligation transfer. The 37 local links
in changed Markdown resolved and the whitespace check passed. This review does
not qualify the future adapter or a numerical method.
