# Dependency sequencing — 23 October 2026

Inventory-first Polars migration supersedes the 22 October blanket foundation gates. No runtime acceptance or implementation status changes. Separate tracks remain mandatory for each direct dependency.

## Core order and ownership

1. Start Kairos Track 70 Phase 0 inventory and migration-boundary work under its existing 04/62/63 owner handoffs. Inventory is not evidence that those contracts are accepted.
2. Classify each sha2 (93), tempfile (94) and proptest (96) target as removed, retained and required for a named migration task, or independent. Keep separate tracks; no universal 93 → 94 → 96 → 70 ordering.
3. Dispatch only the bounded prerequisite work needed by the next affected migration task; qualify its receipt before that task. Independent retained consumers can proceed separately. Track 70 owns real-format replacement, obsolete-code deletion and full output/replay acceptance.
4. CareOps keeps reconciliation R3 and accepted Kairos Track 70 Phase 4 receipts before R10/pin adoption. R10.2 classifies parent sha2 R50–R52, tempfile R53–R55 and proptest R59–R61 targets. Remove the blanket R11 gates; add exact task gates only where a retained consumer or required oracle demonstrates need. Existing upstream receipt requirements still apply when adopting upstream changes.
5. CareOps Polars R11–R14 acceptance → Kairos 74 Serde → 75 serde_json → 76 TOML → 77 Clap → 95 thiserror. Parent thiserror R56–R58 requires both accepted Kairos 95 and CareOps Polars R14.
6. Kairos 71 rand → CareOps rand R20–R22 (also waits for parent thiserror R58) → Kairos 72 rand_distr → CareOps sampling R23–R25 → Kairos 78 Rayon and parent consumer verification → CareOps ECharts R26–R28 → Kairos 73 petgraph → CareOps routing R29–R31 → Three.js R32–R35.

Track 70 starts with Phase 0 inventory and migration-boundary decisions; Tracks 93/94/96 are not blanket entry or runtime-migration prerequisites. For each digest, staging or property-test target, record its path, owner and disposition: removed by Polars, retained and required for a named migration task, or independent. Before dispatching an affected task, record the exact prerequisite task/receipt and acceptance oracle in the owning plan and dispatch packet (and the CareOps task graph where representable). Unresolved required work blocks that task only; unrelated migration work may proceed under existing ownership and contract gates. Do not retrofit code scheduled for deletion. If no targets remain, record a reviewed scope-elimination decision without claiming adoption. Required integrity, durability, interoperability, replay and independent test evidence remain mandatory in Polars. Separate dependency tracks and current owner handoffs remain in force.

Existing RNG semantics remain unchanged during Polars. Property tests belong before the affected replacement, not necessarily before all migration work. No dependency track waits for completion of the migration task it enables.

## Optional branches

| Dependency | Placement | Adoption condition |
| --- | --- | --- |
| tracing (97) | After Rayon 78 | Named diagnostic gap and bounded overhead; not authoritative event logging |
| tracing-subscriber (100) | After tracing 97 | Real executable configuration need; never global initialization in a library |
| loom (98) | After Rayon 78, before accepting any later change relying on its selected primitive | Bounded instrumented model, mutation proof, explicit unmodelled operations |
| proptest-state-machine (99) | After proptest 96 | Plain proptest leaves material transition/shrinking machinery to replace |

No optional branch is a prerequisite for Polars, RNG, charts or transport. When optional test adoption is declined, native/conformance/property obligations remain. CareOps optional use requires an owner-qualified consumer packet; no automatic mirrored dependency.

Existing secondary sequence remains: assess rust-fmi 91 before choosing ZIP 79 / quick-xml 80 replacements; actual RPC requirement → Prost 81 → Tokio 82 → Tonic 83; consumer-qualified MPI 88, Tract 89, Clarabel 85, Argmin 90 and Sobol 92. hecs 84 and rstar 86 remain evaluations. geo 87 follows petgraph; Valhalla follows parent routing and geo; MapLibre follows Valhalla and Three.js; SUMO follows qualified roads and demonstrated traffic need. Rapier stays deferred without executable work.

## Evidence and dispatch

Machine registries express whole-track/milestone dependencies; cross-repository receipts are manual and unresolved. A local DAG pass does not certify upstream acceptance. Read-only inventory is not mutation authority. Runtime files remain with their existing owners and require bounded claims. All candidates retain the value gate, exact version/security/licence/target qualification and rejection/defer option.

CareOps planning publication is reconciled; runtime candidate qualification remains governed by reconciliation R2/R3. Preserve the accepted pin until those gates pass. Kairos main requires PR/checks; do not force-push or waive them.

## Removed-target disposition

If reconciliation removes every candidate target, do not install a dependency to satisfy a graph edge. Record an accepted scope-elimination decision with evidence and update the affected prerequisite registry/receipt under coordinator review before dispatch; keep adoption tasks unimplemented. A no-go decision alone does not silently satisfy an existing gate.

[Planning review and validation](dependency-sequencing-review.md)

## Schema validation, existing tooling and conditional capabilities

Refinement approved 22 October 2026; no runtime adoption.

| Work | Kind | Owner and order |
| --- | --- | --- |
| Criterion 101 | Consolidate existing tooling | Kairos benchmark owner; no core prerequisite, then CareOps R68–R70 for actual microbenchmark consumers |
| Rust jsonschema | New dependency qualification | CareOps R62–R64 after parent Polars R14 and upstream serde_json 75; shared schema contract before browser qualification |
| Ajv | New dependency qualification | CareOps R65–R67 after schema R64; gates ECharts implementation R27 and affected live input acceptance |
| ode_solvers 102 | Conditional scientific capability | Kairos after Rayon 78 and demonstrated continuous-dynamics need; independent of correcting existing RK4 claim |
| Tungstenite 103 | Conditional real transport | Kairos after Rayon 78 and accepted live consumer; parent schemas before affected input acceptance |
| tokio-tungstenite 104 | Conditional async adapter | Only after Tungstenite 103 and accepted Tokio 82; real async requirement, no automatic second transport |

Core sequence otherwise remains intact. Rust schema qualification can run once parent Polars and upstream JSON contracts are accepted, while TOML/Clap/error/RNG/sampling/Rayon work proceeds in its existing order. ECharts R26 can prepare; R27 waits for Ajv R67. The existing Python jsonschema validator remains an independent oracle. No new schema validator is added to Kairos without an actual generic consumer. Shared schemas cover structure; authoritative clinical and cross-record decisions remain native domain logic.

Benchmark consolidation is supporting work, not a blanket migration blocker. Each implementation track must still use an adequate baseline and its existing cost gates; do not replace specialist subprocess/GPU/MPI/RSS measurements with microbenchmarks.

Track 65 owns the open Euler/RK4 mismatch independently of optional ODE selection. Do not revive stale numerical capability claims from names or self-roundtrip tests. Track 103 replaces an in-memory placeholder only when real transport is required; application ordering/resync/idempotency/backpressure and cancellation remain project-owned. Any live parent packet must verify accepted ABI schemas and actual browser/native transport, not simply the ECharts completion status.

No ajv-cli, additional format plugin, transport framework or numerical backend is implicitly approved. Separate direct-dependency tracks are required if qualification demonstrates a need. SUMO remains conditional; Rapier remains deferred.

[Validation/tooling planning review](validation-tooling-review.md)
