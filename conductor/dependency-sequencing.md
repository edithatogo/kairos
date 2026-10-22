# Dependency sequencing — 22 October 2026

This refinement supersedes earlier “Polars first” wording only for its bounded integrity, temporary-file and testing prerequisites. No runtime acceptance or implementation status changes. Separate tracks remain mandatory for each direct dependency.

## Core order and ownership

1. Joint read-only source/format inventory with the Polars owners; distinguish removed codecs from retained digest/publication consumers.
2. Kairos 93 sha2, 94 tempfile and 96 proptest: independent bounded foundations, each with its own claim and receipt. Suggested delivery order 93 → 94 → 96; no false technical dependency between them.
3. Kairos 70 Polars migration and full output/replay acceptance. It depends on all three foundations and its existing prerequisites.
4. CareOps R10 upstream baseline. Parent sha2 R50–R52, tempfile R53–R55 and proptest R59–R61 can be delivered once their respective upstream foundation receipts exist; all must finish before Polars R11. Each owns real direct parent code/tests, not duplicate generic implementations.
5. CareOps Polars R11–R14 acceptance → Kairos 74 Serde → 75 serde_json → 76 TOML → 77 Clap → 95 thiserror. Parent thiserror R56–R58 requires both accepted Kairos 95 and CareOps Polars R14.
6. Kairos 71 rand → CareOps rand R20–R22 (also waits for parent thiserror R58) → Kairos 72 rand_distr → CareOps sampling R23–R25 → Kairos 78 Rayon and parent consumer verification → CareOps ECharts R26–R28 → Kairos 73 petgraph → CareOps routing R29–R31 → Three.js R32–R35.

Foundation closure never waits for future migration outputs. Polars independently proves integration, durability, real interoperability and deletion of obsolete formats. This avoids the cycle of making a prerequisite wait for the migration it enables. Existing RNG semantics remain unchanged during Polars. Property tests are extended within every owning implementation track; Track 96 is not a promise to test future work in advance.

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

Current local CareOps main diverges from origin/main; reconcile source/ownership/acceptance evidence before execution or publishing. This planning update does not resolve that divergence. Kairos main requires PR/checks; do not force-push or waive them.

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
