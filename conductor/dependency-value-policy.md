# Robustness-first dependency acceptance policy

Approved planning direction: 22 October 2026. Primary goal is robustness and reduced maintained code in existing systems. A dependency is not a feature or a success criterion by itself. Registration does not install anything, accept runtime behaviour or authorise conditional adoption.

## Version selection — user clarification, 24 October 2026

Prefer the latest suitable qualified releases, including Python 3.15.2 as the
current repository default (29 October qualification update). A necessary alternate runtime or dependency version
is permitted for application, tooling, CI/helper or provider compatibility;
it does not require additional user permission. This supersedes the earlier
mandatory Python 3.15-only/no-alternate rule in current instructions and plans.

Record the exact consumer/profile, compatibility limitation, newest suitable
version selected, constraints/hash locks, actual version/prefix and meaningful
execution/security evidence. An installed binary, provider declaration or metadata
edit is not qualification. Keep profile-specific contracts explicit; do not lower
application metadata merely to satisfy a provider parser. Change only the affected
profile, preserve native bindings and in-flight environments, and retain the
working latest-version profile. An exception is not permission to weaken security,
functionality, ownership, reproducibility or required checks.

No alternate is adopted by this policy revision. Historical receipts retain the
requirements and results that applied when captured. Optional tooling still needs
a demonstrated benefit; compatibility permission alone does not justify adoption.

## Mandatory value gate

Before any adoption, record the exact existing production code to replace (or a required feature and avoided implementation), real consumers, observed failures/limitations and an independent behavioural baseline. Compare retain-current versus dependency-backed implementations. Count retained wrappers, migrations and operational burden in the result, not just deleted lines. Keep domain validation and independent tests; do not delete tests to inflate savings.

Require a demonstrated robustness or maintainability improvement, with no unacceptable performance regression. Performance improvements are measured on representative workloads; new features count only when tied to an accepted consumer. Freeze budgets and acceptance thresholds before implementation; a runtime speedup is not required for a parser whose benefit is correctness, but build/binary/memory/latency costs must be known and acceptable.

Check current maintenance/release activity, issue/security handling, documentation, tests/fuzzing, licence, MSRV, feature closure, targets and upstream compatibility policy at entry. Pin exact qualified versions; no blanket latest-version upgrade and no unsupported marketing claims. Rust-native dependencies are preferred. Native runtime/service exceptions (MPI, Valhalla, SUMO) require a benefit that outweighs build, operation and coupling costs.

## Stop or decline

If there is no production target/accepted consumer, no net benefit, unsupported contracts or excessive migration burden, record no-go/deferred with evidence. Keep Phase 1/2 unchecked, adoption_decision=deferred/rejected, and do not install a dependency merely to close a track. Close/archive a rejected evaluation only with an explicit outcome; never label it implemented. Automatic ready lists are preparation only and cannot override these gates.

## Ownership and validation

Kairos owns generic reusable infrastructure; CareOps owns healthcare policy, site/vehicle/crew data and dashboard/application services. One canonical dependency track lives with the implementation owner; downstream work uses that track's receipt and a separately claimed parent packet. Do not clone generic implementations into CareOps. A direct parent dependency needs a justified local consumer; upstream transitive use alone needs no duplicate direct dependency.

Retain DES scheduling/resource arbitration, domain clinical/hazard semantics, PDES causality and public identity/replay guarantees. Version intentional changes; preserve old outputs. Independent consumer tests, failure/recovery, precise command/cwd/commit/toolchain/versions/seed/input/output hashes and raw receipts are required. No production, clinical, hosted-CI or release claim follows from a document validator.

## Priority and conditionality

Foundation: Serde, serde_json, TOML and Clap, then accepted rand/rand_distr and Rayon. Polars inventory and migration-boundary decisions come first. Separate sha2/tempfile/proptest tracks gate only named migration tasks with demonstrated retained consumers or required test oracles, never the whole migration. Record the path, owner, prerequisite receipt and oracle before affected dispatch. If Polars removes every target, record a reviewed scope-elimination decision without rebuilding it or claiming adoption. The documented 1.76 floor is stale relative to the current Kairos Cargo.toml declaration of 1.99; resolve actual manifests/CI/pins in both repositories, not by blindly raising an already higher floor.

Existing-subsystem work: ZIP/XML only for an active FMI consumer; assess whole rust-fmi replacement first to avoid double migration. Distributed and numerical libraries require active requirements and independent capability proof. hecs/rstar are evaluations with retain-current as a valid outcome, not mandated migrations.

Geographic transport is later and consumer-gated. SUMO stays conditional on traffic interactions materially affecting the research/operational question; consider offline calibrated scenarios before live coupling. Rapier remains deferred, without executable adoption tasks. Road-network movement does not require rigid-body physics. DataFusion and WebGPU remain deferred.

## Integrity and qualification refinement

[Dependency sequencing](dependency-sequencing.md) is the current complete ordering. New canonical Kairos tracks: 93 sha2; 94 tempfile; 95 thiserror; 96 proptest; conditional 97 tracing, 98 Loom, 99 proptest-state-machine and 100 tracing-subscriber. Parent direct-use tracks cover sha2/tempfile/thiserror/proptest. No new dependency installed.

## Optimisation versus adoption

Prefer reusing qualified existing tooling (Criterion, Python jsonschema) over another framework. Schema libraries replace duplicated structural checks only; preserve domain validation. Capability placeholders and incorrectly named algorithms require truthful correction regardless of whether an alternative dependency is adopted.

## CareOps-Sim impact and opportunity review

At dependency closeout (including defer/reject), the Kairos owner supplies the accepted immutable commit, decision, exact versions/features and relevant behavioural/compatibility receipts. The CareOps-Sim coordinator and actual consumer owner perform a small downstream review in `conductor/dependency-handoffs.json`, keyed by `owner_track`. Review already accepted tracks as follow-through; do not reopen historical implementation phases or rewrite their receipts.

Check actual consumers and implications for APIs/bindings, schemas/output formats, configuration, deterministic streams/checkpoints, builds/deployment and compatibility. Look for worthwhile opportunities to remove bespoke code, reuse capabilities or improve robustness/performance; record evidence and limits rather than assuming benefit from library adoption.

Record one outcome: `no_action`, `pin_update_and_verification`, `bounded_adaptation`, or `implementation_track`. “Reviewed; no implications or worthwhile opportunities” is a valid `no_action` result with a reason and evidence, including no consumer or an upstream no-go. A pin update still requires consumer verification; adaptation names bounded paths, owner and checks. Reuse an existing implementation track first. Create a separate track only for demonstrated substantive work not already covered; register its link once, without mirroring the upstream implementation.

Each review records status, accepted upstream commit, upstream decision, reviewer/consumer owner, review date, consumer context, implications, opportunities, outcome, rationale, evidence and follow-up reference. Pending fields remain null/empty until reviewed. Completion requires the commit, owner, date, context, rationale and evidence, plus an owned follow-up for actionable outcomes. Review completion is distinct from follow-up implementation acceptance. Re-review when relevant upstream contracts/behaviour or consumer context changes; unrelated commits alone do not invalidate a review.

Sequence: accepted Kairos receipt -> downstream review -> separately claimed pin/adaptation or existing-track task -> consumer acceptance. This adds no blanket prerequisite to unrelated work and changes no implementation order. Direct dependencies require a demonstrated local consumer; the register’s dependency disposition describes local consumer needs, not automatic installation authority. Keep DES/domain semantics and existing conditional/deferred decisions intact. No new runtime dependency, automation or mirrored implementation track is required.

## Delivery and maintenance refinements — 24 October 2026

These requirements refine existing acceptance tasks; they create no adoption tracks or blanket dispatch gates.

- **Durable receipts:** distinguish the tested implementation/head SHA from the accepted integration/merge SHA and record the PR, versions/features, commands and retained evidence. After integration, verify the accepted commit and evidence from a fresh checkout of published history. Squash merges can remove implementation commits from main ancestry: retain their provenance separately, verify source parity and tests, and reconcile the canonical published receipt/ref through its owner. A deleted topic branch is not a durable receipt; never invent a replacement SHA, silently relabel tested evidence or weaken ancestry checks. The current closeout owner controls ledger changes.
- **Real prerequisites versus preferred order:** the documented serial order remains the delivery preference. A blocking edge must name the contract/consumer, exact task/receipt and oracle it enables. Record preferred ordering as scheduling guidance, not an artificial technical gate. Reconcile existing graph edges through their owners before dispatch; this policy does not silently bypass them.
- **Shared consumer fixtures:** reuse the existing conformance suite for a small set of representative Kairos-to-CareOps-Sim cases: full-range identity/ticks, units, empty/missing values, actual output interchange and replay/checkpoint behaviour where affected. Add independent expected results and negative cases at the changed boundary; avoid a new test framework or a duplicate domain model. No fixture is a substitute for representative runtime/cost checks.
- **Net maintenance result:** closeout compares bespoke code removed, wrappers retained and their purpose, duplicate paths eliminated, enabled Cargo features/transitive dependencies, and material build/binary/memory/runtime costs against the entry baseline. Use existing representative benchmarks; add measurement only where the affected boundary warrants it. State uncertainty and accepted tradeoffs rather than treating line counts or dependency presence as benefit.
- **Removal is completion:** after parity and recovery are demonstrated, delete superseded implementations, unused dependencies/features and stale setup instructions in the owning task. Keep historical outputs/provenance as evidence, not a permanent second implementation. Any temporary compatibility path needs a named consumer, owner, exit condition and removal task in the existing track.
- **Proportionate upgrades:** reuse existing dependency automation and pinned qualification. For every update assess the affected API/ABI, feature closure, schemas, deterministic sampling, checkpoints and target/security implications. Run focused contract/consumer checks for compatible changes; broaden to integration, clean-package, performance and reproducibility checks when those contracts or costs can change. Version sampling/format changes explicitly. A patch label alone does not prove compatibility; unchanged valid evidence can be reused. Do not add another upgrade service or recurring process.

The CareOps-Sim handoff register is checked by its existing context validator. It rejects duplicate owner-track keys, invalid outcomes, incomplete completed reviews and unresolved local follow-up references. `direct_dependency_disposition` is one of `undetermined`, `unnecessary`, `existing`, or `proposed`; it describes the actual local consumer decision, not permission to install. Legacy boolean decisions remain migration evidence and do not establish presence.

Review freshness is tied to the relevant upstream change and consumer context (paths/contracts plus versions/features or content fingerprints), not every unrelated Kairos commit. Retain the original accepted receipt; record the later comparison and its evidence before reusing a review. Mark affected reviews `stale` until reassessed; never autoaccept a changed consumer. Review completion does not mark follow-up implementation accepted. SUMO stays conditional; Rapier stays deferred.
