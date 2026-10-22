# Robustness-first dependency acceptance policy

Approved planning direction: 22 October 2026. Primary goal is robustness and reduced maintained code in existing systems. A dependency is not a feature or a success criterion by itself. Registration does not install anything, accept runtime behaviour or authorise conditional adoption.

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

Foundation: Serde, serde_json, TOML and Clap, then accepted rand/rand_distr and Rayon. Bounded sha2/tempfile/proptest foundations precede Polars; if Polars already removes a target, reconcile and close the redundant scope without rebuilding it. The documented 1.76 floor is stale relative to the current Kairos Cargo.toml declaration of 1.99; resolve actual manifests/CI/pins in both repositories, not by blindly raising an already higher floor.

Existing-subsystem work: ZIP/XML only for an active FMI consumer; assess whole rust-fmi replacement first to avoid double migration. Distributed and numerical libraries require active requirements and independent capability proof. hecs/rstar are evaluations with retain-current as a valid outcome, not mandated migrations.

Geographic transport is later and consumer-gated. SUMO stays conditional on traffic interactions materially affecting the research/operational question; consider offline calibrated scenarios before live coupling. Rapier remains deferred, without executable adoption tasks. Road-network movement does not require rigid-body physics. DataFusion and WebGPU remain deferred.

## Integrity and qualification refinement

[Dependency sequencing](dependency-sequencing.md) is the current complete ordering. New canonical Kairos tracks: 93 sha2; 94 tempfile; 95 thiserror; 96 proptest; conditional 97 tracing, 98 Loom, 99 proptest-state-machine and 100 tracing-subscriber. Parent direct-use tracks cover sha2/tempfile/thiserror/proptest. No new dependency installed.

## Optimisation versus adoption

Prefer reusing qualified existing tooling (Criterion, Python jsonschema) over another framework. Schema libraries replace duplicated structural checks only; preserve domain validation. Capability placeholders and incorrectly named algorithms require truthful correction regardless of whether an alternative dependency is adopted.
