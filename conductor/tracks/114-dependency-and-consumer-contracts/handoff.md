# Handoff

Updated 25 November 2026 (Australia/Brisbane). Status **Done** for the bounded
consumer contract and reuse scope.

## Consumer contract closeout — 25 November 2026

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns this closeout in
`/Users/doughnut/.codex/worktrees/kairos-scientific-evidence-closeout/kairos` on `codex/consumer-contract-closeout-20261125`. It covers the consumer contracts, resolved Rust
feature guard, receiving evidence and Conductor records. The current operation
is publication and verification of this scope.

The standalone fixture at Kairos `6316b4d7a2158865e0f3533ff3abe1cd4856099b` ran with
`cargo run --locked --offline` from its Git-free archived Cargo workspace on
macOS ARM64, using Rust and Cargo 1.99.1. It dispatched a service completion
and admitted the queued job. The lock SHA-256 is
`85a5e4adcc83bbe4f6b2660c479e0f70ec0baf792e9609a3589bba44d04af8c1`.
The independent graph guard and its seeded negative passed on CPython 3.15.2.
The fixture compiled in 7.17 seconds with cached dependencies and debug
information disabled, and the resulting binary was 624,656 bytes.
[Public profile and commands](../../contracts/minimal-consumer.md#standalone-rust-fixture--25-november-2026)

The four solver retirement PRs removed a net 274 lines from
`crates/kairo-ecs-game-theory/src`, counting retained test-only code. Paignia
owns the receiving algorithms. Its static core and solver have eight resolved
package names on macOS ARM64. Its separate unpublished adapter ran 12 locked
cases against Kairos `235e6a3`. Linux ran the same adapter at Paignia
`e675da1a3a8bf9fbf88c43d25171643965aa05e7` on stable and Rust 1.99.0.
[API withdrawals](../../scientific-completion.md#s01s04-paignia-transfer) ·
[Native adapter handoff](../../contracts/game-port-acceptance.md#receiving-handoff--25-november-2026)

Use the accepted CareOps and Innovate receipts for replay, lifetime and
schema behavior. The selected SDK and movement probe exercise native
allocation, asynchronous cancellation and committed occupancy. The existing
worker comparison provides the runtime-cost boundary on its measured ring.
At the pinned revision, the normal provider closure increases from 21 unique
packages for the four game-port roots to 22 for DES and 50 for ABM.
These counts measure build exposure. The elapsed compile and binary size
above measure the standalone fixture. The source delta measures maintenance
in Kairos. Ecosystem-wide maintenance and build-time comparisons, hardware
scaling and broader dynamic models require separate workload evidence.
[Consumer receipts](../108-scientific-ecosystem-alignment/handoff.md#installed-consumer-qualification--23-november-2026) ·
[Measured ring](../../archive/111-measured-performance/handoff.md#matched-ring-and-worker-reuse) ·
[Comparable dependency trees](../../contracts/minimal-consumer.md)

The standalone execution log SHA-256 is
`97582f149a7a1ac48d7a7be91b57670a5aed2ee6f752d8f9f735203ebe8b98ca`.
The Paignia archive SHA-256 is
`daa515b1bf7f0fb3c87288e4ff8a98bd116f280233e764edb33ccfc08b5b4beb`
and the adapter test log SHA-256 is
`63ac95b54cabe0e9c9dabe0aead0201a0d74f96b60307f5c16c9c086db448f35`.

## Installed CareOps movement — 23 November 2026

Integration chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the movement
qualification and evidence files in the Kairos closeout worktree on
`codex/careops-installed-movement-20261123`. The CareOps receiving probe ran in
`/Volumes/PortableSSD/kairos-careops-movement.FDdotV/careops`.

The test used [CareOps main `9af99b5`](https://github.com/edithatogo/careops-sim/tree/9af99b5730d6aa5cef1c05bd48e90fd377b80cf2)
and the extracted Rust SDK from [native install run 45051171243](https://github.com/edithatogo/kairos/actions/runs/45051171243).
The SDK is from Kairos `e2875387ce45c34cd19716ea66f0102251a6aa6f`, SHA-256
`0060c934d7b5d9eb187387175fc4604184093823aedbfbb43fa52936b1a82240`.
The CareOps source archive contains the workspace manifest, lock and
`crates/careops-core`, SHA-256
`380c917c5791cb57cf714d064f9a67e3baebdeecdb0dd1948e5f538511a5509f`.

In the archived workspace, use the [installed CareOps recipe](../../../packaging/scripts/qualify_native_beta.py)
to isolate `careops-core` and point its Kairos dependencies at the extracted
SDK. Add SDK `kairo-ecs-abm` and `kairo-ecs-state` as dev dependencies, copy
the SDK root's immutable `hashbrown` patch into the consumer root and append
the [probe module](careops_movement.rs) to
`crates/careops-core/tests/allocation_kairos_conformance.rs`. The initial
resolution selected registry `hashbrown` outside the retained locks. Reusing
the SDK patch resolved all six Kairos crates from the extracted artifact and
kept external dependencies within the two frozen locks. The resolved lock
SHA-256 is `e9c5dd385bfa4f04c59cc509cf1576e83275a3d039896a2fcdb49b4c2e8e96ae`.

`rustup run 1.99.1 cargo test --locked --offline -p careops-core --test allocation_kairos_conformance`
passed all three tests on macOS ARM64, exit 0. The probe reuses CareOps's
`StagedAllocator` grant and release acknowledgments. The ordinary route
uses the stairs. A fictional patient requiring step-free access takes the
eight-tick lift route with a porter admitted at tick 2. Arrival at tick 10
changes location once, completes the CareOps handover and releases eight
porter-ticks of occupancy. Cancellation at tick 5 keeps the origin and
releases after three porter-ticks. Both cases match the occupancy interval
to the resource, actor, request and lease revision and clear the pending
movement. The existing capacity-one contention test also passed. There are
no stochastic draws in this fixture. The probe SHA-256 is
`b18681d9b122e1eeee2f6e5ba8c5803a3c462f7bc6a079c0d775a1d7c3f3c45c`
and the test log SHA-256 is
`0e921ca53ecb0cefe1bc75f58368534649af417905ad6f37a7e1d35dc6dad757`.

Independent review matched all 67 active SDK files to the candidate's
source. The review added the occupancy identity and pending-movement
checks and verified the rebuilt three-test pass.

CareOps owns the application adapter and provider pin under
[R76.2 and R77](https://github.com/edithatogo/careops-sim/blob/9af99b5730d6aa5cef1c05bd48e90fd377b80cf2/conductor/tracks/kairos_domain_contracts_20261107/plan.md).

## Reused installed consumer evidence

The [Track 108 receipts](../108-scientific-ecosystem-alignment/handoff.md#installed-consumer-qualification--23-november-2026)
cover replay, schema and lifetime. The 47 passing tests on the installed
Innovate receiving wheel included native DES dispatch, seeded replay and engine
error, ABM agent state under forced adoption, and checkpoints resumed at
ticks 0, 4 and 6 across DES, ABM and network paths. The Arrow/Polars cases
compared event rows and exercised retained buffers after producer or reader
drop. Innovate owns production qualification and pin selection.

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

Independent `$conductor-review` accepted Phases 0–3. The accepted fixes
correct the static graph count, include generated-code and HTTP/binding/inference
package families, test transitive bindgen resolution and distinguish measured
footprint from an ecosystem cost comparison. The runtime evidence baseline
commit SHA is `6316b4d7a2158865e0f3533ff3abe1cd4856099b` on the pushed
ref `origin/main`. The closeout PR carries the new checked graph guard and
preserves the exact receiving source IDs in the handoff above.
`scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
verifies published source and retained ancestry. The next-phase decision closes
the bounded provider contract. Receiving applications own their release and pin
acceptance, including the signed Tobac inventory in Track 116.


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
