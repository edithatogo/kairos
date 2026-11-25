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

[Current sequencing review and validation](polars-sequencing-review-20261023.md). The [22 October review](dependency-sequencing-review.md) is historical and does not validate the revised graph.

## Schema validation, existing tooling and conditional capabilities

Refinement approved 22 October 2026; no runtime adoption.

| Work | Kind | Owner and order |
| --- | --- | --- |
| Criterion 101 | Consolidate existing tooling | Kairos benchmark owner; no core prerequisite, then CareOps R68–R70 for actual microbenchmark consumers |
| Rust jsonschema | New dependency qualification | CareOps R62–R64 after parent Polars R14 and upstream serde_json 75; shared schema contract before browser qualification |
| Ajv | New dependency qualification | CareOps R65–R67 after schema R64; gates ECharts implementation R27 and affected live input acceptance |
| ode_solvers 102 | Conditional scientific capability | Kairos after Rayon 78 and a demonstrated continuous-dynamics need in the engine |
| Tungstenite 103 | Conditional real transport | Kairos after Rayon 78 and accepted live consumer; parent schemas before affected input acceptance |
| tokio-tungstenite 104 | Conditional async adapter | Only after Tungstenite 103 and accepted Tokio 82; real async requirement, no automatic second transport |

Core sequence otherwise remains intact. Rust schema qualification can run once parent Polars and upstream JSON contracts are accepted, while TOML/Clap/error/RNG/sampling/Rayon work proceeds in its existing order. ECharts R26 can prepare; R27 waits for Ajv R67. The existing Python jsonschema validator remains an independent oracle. No new schema validator is added to Kairos without an actual generic consumer. Shared schemas cover structure; authoritative clinical and cross-record decisions remain native domain logic.

Benchmark consolidation is supporting work, not a blanket migration blocker. Each implementation track must still use an adequate baseline and its existing cost gates; do not replace specialist subprocess/GPU/MPI/RSS measurements with microbenchmarks.

Kairos retired the legacy RK4-named export in Track 65. Paignia owns the
qualified fixed-step replicator methods.
[Replicator migration](../docs/game-theory/equilibrium-and-cfr-solvers.md#replicator-migration)

Track 103 replaces the in-memory alias when a consumer needs real transport.
Application ordering, resync, idempotency, backpressure and cancellation remain
with the application. A live parent packet verifies the accepted ABI schemas
and browser/native transport.

No ajv-cli, additional format plugin, transport framework or numerical backend is implicitly approved. Separate direct-dependency tracks are required if qualification demonstrates a need. SUMO remains conditional; Rapier remains deferred.

## Conditional evaluation closeout — 25 November 2026

Tracks 90, 97, 98, 100, 102, 103 and 104 close Phase 0 with adoption **Deferred**.
Each decision applies to Kairos workspace use. Adoption phases and their
behavioural gates apply if a demonstrated consumer reopens the track. Track 100
requires accepted Track 97 instrumentation. Track 104 requires an accepted
Track 103 transport as well as the existing Track 82 Tokio runtime.

| Track | Current evidence and disposition | Reopen when |
| --- | --- | --- |
| [90 Argmin](tracks/90-argmin-robustness/handoff.md#phase-0-closeout--25-november-2026) | The CLI crate's Rust ensemble API scores caller-supplied inputs and implements sequential ABC-SMC with weighted populations. The reviewed callers identify no generic objective-minimisation loop for Argmin to replace. | A consuming calibration workflow needs an iterative optimiser with an objective, bounds, termination, seed and restart contracts. |
| [97 tracing](tracks/97-tracing-diagnostics/handoff.md#phase-0-closeout--25-november-2026) | The missing-GVT-peer probe reported useful fields. Its deliberate 40 ms subscriber blocked the reducing thread beyond the 5 ms budget. The current Tonic transport keeps reported minima diagnostic and conservatively prevents raw contribution rounds from advancing reclamation. | An application owner supplies the diagnostic consumer, numeric context and a bounded dispatch policy that passes the parity and overhead oracles. |
| [98 Loom](tracks/98-loom-concurrency/handoff.md#phase-0-closeout--25-november-2026) | Retain the standalone allocator model. Its locked Loom 0.7.3 run catches the seeded duplicate-ID mutant and passes the production CAS model. The reviewed paths yield no further workspace model requirement. | A second concrete synchronization contract or a production regression warrants a maintained model with a measured budget. |
| [100 tracing-subscriber](tracks/100-tracing-subscriber/handoff.md#phase-0-closeout--25-november-2026) | The CLI uses stdout for results and stderr for errors. The deferred GVT emitter supplies no executable subscriber integration. | A host needs configuration for an accepted emitter and can verify filtering, redaction, bounded queues and machine stdout. |
| [102 ode_solvers](tracks/102-ode-solvers-qualification/handoff.md#phase-0-closeout--25-november-2026) | Innovate executes adaptive Dopri5 with its vendored solver and owns the model and tolerances. Paignia provides versioned fixed-step replicator solvers. An accepted reusable Kairos ODE contract is absent. | A named Kairos consumer needs shared continuous integration and supplies numerical tolerances and reference solutions, with event and restart semantics where applicable. |
| [103 Tungstenite](tracks/103-tungstenite-transport/handoff.md#phase-0-closeout--25-november-2026) | The evaluated Kairos source exposed a process-local stream alias, [since withdrawn](../docs/adr/0027-retire-streaming-aliases.md). The inspected CareOps dashboard encodes commands and handles supplied messages. A live Kairos endpoint remains unqualified. | A live client and endpoint have an accepted wire contract with authentication, bounded messages, resync and idempotency. |
| [104 Tokio-Tungstenite](tracks/104-tokio-tungstenite-adapter/handoff.md#phase-0-closeout--25-november-2026) | The Tokio transport oracles cover framed TCP. There is no accepted WebSocket transport or concurrent socket workload to qualify this adapter against. | Track 103 has an accepted transport and a measured simultaneous-connection workload justifies the async adapter. |

The locked workspace inventory found zero direct edges to the seven libraries.
The root lock retains transitive `tracing`, and the project template declares
it separately. The standalone Loom lock also resolves `tracing` and
`tracing-subscriber`. CareOps's seven impact-review rows remain pending in
its owned register. Its current Kairos gitlink is `07410266`.
[CareOps dashboard](https://github.com/edithatogo/careops-sim/blob/9af99b5730d6aa5cef1c05bd48e90fd377b80cf2/apps/dashboard/src/live/websocket_client.ts) ·
[CareOps register](https://github.com/edithatogo/careops-sim/blob/9af99b5730d6aa5cef1c05bd48e90fd377b80cf2/conductor/dependency-handoffs.json) ·
[Innovate ODE and calibration code](https://github.com/edithatogo/innovate/blob/690e0496078fe2f4cbd22c8e4ab13242b0106b96/rust/crates/innovate-core/src/numerics.rs) ·
[Paignia solver](https://github.com/edithatogo/paignia/blob/e675da1a3a8bf9fbf88c43d25171643965aa05e7/crates/paignia-solve/src/replicator.rs) ·
[Tobac source inspected](https://github.com/edithatogo/tobac/tree/fe06ef5958ba0f218317a738fe8de95973300a12)

### Reproduction

Run from the Kairos root at `ca00777512db29e846e458696244eb09686dda8e`.
The 25 November macOS ARM64 readback used Rust/Cargo 1.100.0 and isolated
target directories with `CARGO_PROFILE_TEST_DEBUG=0`. All four commands exited
zero. The first test invocation ran 4 ABC-SMC, 8 streaming, 7 Tonic and 7
Tokio cases. The allocator ran 11 native cases. The standalone Loom suite ran
3 tests, including the expected mutant panic and 7 modeled CAS executions.

```sh
cargo metadata --no-deps --locked --offline --format-version 1
cargo test --offline --locked -p kairo-ecs-cli -p kairo-ecs-streaming -p kairo-ecs-grpc \
  --features kairo-ecs-streaming/websocket --test maturity_abc_sequential \
  --test feature_matrix --test tonic_transport_oracles --test tokio_transport_oracles
cargo test --offline --locked -p kairo-ecs-pdes --features time-warp --lib generation_bitset::tests
cargo test --offline --locked --manifest-path conductor/tracks/98-loom-concurrency/model/Cargo.toml -- --nocapture
```

| Retained file | SHA-256 |
| --- | --- |
| Workspace `Cargo.lock` | `df26476d07b6323f89bb02819aaff00a12f00c15bdd0114c91d082cd088f0637` |
| Standalone Loom `Cargo.lock` | `b829ffcb90a2e5230c9b2b8c88e48c24b7e4bbac764491e6501f80860dd76d98` |
| `workspace-metadata.json` | `9f6692e7a13fd6d477678010a63df915ab57b10a6bd533871381c0d6aaaaf94f` |
| `runtime-oracles.log` | `c279744f1245ec6986865fa0e456134f9cdc73bf8dca742453ae5dd847f85167` |
| `native-allocator.log` | `2a5ea316e72c962b52fbf8f501249083c867302b60820192071a091164c24e23` |
| `loom-model.log` | `9f5655eb746f18b69bbe50c5ac2fabd6be8868044bc01a26cd64051b135e1fd9` |

Local receipts are under `/Volumes/PortableSSD/kairos-dependency-decisions.mwMe0q`.
The handoffs preserve the prior measured probes and their source-specific limits.
The independent Conductor reviewer reconciled the caller and package inventory
and confirmed the test counts at the source revision above. Review corrected
the retired RK4 description and clarified the Rust ensemble API's scope.

[Validation/tooling planning review](validation-tooling-review.md)
