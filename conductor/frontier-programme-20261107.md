# Kairos: Rust-only maturity and frontier programme

Approved scope: maintainer request of 7 November 2026. Baseline: d56863505d938feb069af88a737eb7668113d14b. This programme implements immediate native-only support and truthful documentation, and registers the remaining engineering and research work. It does not claim beta, 1.0, universal speed leadership or newly validated scientific methods.

## Outcome and architecture

Build an independent Rust simulation dependency with explicit model semantics, native language adapters, reproducible execution and measured performance. CareOps Sim consumes Kairos; clinical/domain models and local datasets stay in the consumer. Host languages may configure, call and visualize Rust execution. Non-Rust simulation cores stop being supported immediately.

Current strengths include deterministic Rust scheduling/state/RNG, resource and agent APIs, and bounded native PDES/MPI/gRPC acceptance. The reviewed baseline hosted job passed 994 tests with 93.70% core coverage; those historical numbers are not a claim about this candidate. The published release is v0.4.0-alpha.1. Some current methods and interfaces are narrower than their names; the completion register makes those gaps actionable.

## Coordinated tracks

| Track | Deliverable | Entry/exit |
|---|---|---|
| [109](tracks/109-rust-only-runtime/index.md) | Immediate retirement of host cores; actual Rust adapters | Missing native support fails; supported installed packages execute Rust and prove lifetime/replay contracts. |
| [110](tracks/110-scientific-method-completion/index.md) | Correct and complete every advertised scientific/advanced method | Independent oracle for every supported method; no identity inference, mislabeled RK4 or unqualified QMC. |
| [111](tracks/111-measured-performance/index.md) | Profiling, persistent workers, batching and measured crossover | Equivalent work, raw measurements, useful speed regimes and regression limits. |
| [112](tracks/112-evidence-and-claim-automation/index.md) | Capability registry, generated status and false-claim rejection | Placeholder/historical/real evidence stays distinct; public summaries match qualified records. |
| [113](tracks/113-beta-and-stable-release/index.md) | Beta, RC and stable gates | Exact published scope, clean installation, consumer acceptance, compatibility and supply-chain evidence. |
| [114](tracks/114-dependency-and-consumer-contracts/index.md) | Maintained dependency reuse and consumer abstraction | Kairos builds independently; CareOps uses public immutable dependency contracts. |
| [115](tracks/115-frontier-experiments/index.md) | Falsifiable research beyond established capabilities | Held-out evaluation against named baselines; no claim of uniqueness without a literature/comparator review. |

The original capability/dependency tracks retain their ownership and historical receipts. These follow-up tracks coordinate unresolved current acceptance; they do not silently mark previously unrun work complete. Shared registry changes are integrated by one coordinator with disjoint writer leases and independent review.

## Immediate delivery

- Retire executable host schedulers and synthetic host environments, preserving explicit errors and actual native/Wasm interfaces.
- Correct README/homepages, quickstarts and paper claims; distinguish real alpha, source implementation and supported installed artefacts.
- Retract synthetic scaling percentages as measured evidence while preserving the genuine small-workload PDES slowdown.
- Record scientific completion tasks and dependency decisions, including the existing RK4 and Sobol defects.
- Add machine-checked capability/claim records and focused negative tests. Structure validation is not runtime or scientific acceptance.

## Core improvement strategy

Keep one authoritative clock/state/seed contract. Improve allocation, queue and component access only after profiling. Reuse persistent workers and safely batch eligible events; preserve deterministic event and RNG semantics. Prefer independent scenario ensembles where they give the best measured return. Keep heavyweight data, transport, inference and accelerator integrations optional. Make absence and unsupported scope explicit.

Use existing maintained dependencies where they reduce actual custom work: Rayon, Criterion, proptest, rand, Arrow/Polars, petgraph/geo/rstar, Clarabel and wgpu/cudarc already have adoption/evaluation history. Review that history before adding alternatives. Full ECS replacement, new numerical libraries and inference runtimes require real consumer contracts and measurable value.

## Research hypotheses

1. **Recorded deterministic execution planning.** Can a model-aware planner select serial/parallel/GPU strategies with low regret against the fastest fixed strategy on held-out workloads while preserving documented result guarantees? Measure useful throughput, end-to-end latency, RSS, planning overhead and replay agreement. Decisions cannot depend on unrecorded wall-clock races.
2. **Efficient policy branching.** Can shared Rust checkpoints plus entity/purpose-addressed random streams reduce memory and time to a target confidence-interval width relative to full replay and conventional CRN? Measure estimator bias, interval coverage, variance, branch isolation and fork cost with entity birth/death, rescheduling and preemption. Model counterfactuals are conditional on model assumptions, not empirical causal proof.
3. **Executable scientific claims.** Can versioned method/data/runtime receipts detect wrong algorithm labels, identity inference, fake hardware, altered inputs and unsupported native paths more reliably and cheaply than manual prose? Pre-register seeded defect families, held-out variants, false-positive limits and reproduction-effort measures.
4. **Bounded hybrid semantics.** Evaluate explicit DES/ABM/continuous-event coupling, resource ownership and replay contracts against independent reference models. Identify which exact, tolerance-based or distributional guarantees survive backend changes. Formal reasoning and property tests complement executable adversarial cases.

These are research proposals. Established comparators already include [krABMaga](https://github.com/krABMaga/krABMaga), [ROSS](https://ross-org.github.io/about.html), [FLAME GPU](https://github.com/FLAMEGPU/FLAMEGPU2), [Madrona](https://madrona-engine.github.io/) and [AgentTorch](https://github.com/AgentTorch/AgentTorch). Rust, GPU, ECS, Time Warp or differentiability alone is not a novelty claim.

## Maintenance and automation

Generate current capability summaries from the same manifest used by docs/release checks. Tie evidence to source/dependency/input hashes, commands, hardware and raw results. Require actual native-artifact tests; metadata checks cannot substitute. Reuse existing dependency-update, security, coverage and provenance jobs. Add meaningful mutations for wrong-method/false-success claims. Run expensive cross-platform/performance/nightly jobs where they resolve concrete risks; retain focused PR checks.

Dependency automation may propose reviewed updates, never silently accept replay-changing RNG/numerical/schema changes. No automation publishes a release, changes protected branches, buys compute or emits messages without its separately authorised operation. This programme itself changes no external consumer pins or patient data.

## Acceptance order and detailed records

Immediate policy/docs -> independent scientific/native contracts -> performance and installed-consumer qualification -> beta artefacts -> independent beta use and RC -> stable scoped release. Research work runs behind explicit experimental APIs until accepted.

- [Native-only policy](native-only-policy.md)
- [Scientific completion register](scientific-completion.md)
- [Performance programme](performance-programme.md)
- [Dataset/oracle catalogue](validation-datasets.md)
- [Dependency/consumer contract](dependency-and-consumer-contracts.md)
- [Release gates](release-gates.md)
- [Machine-readable capabilities](capabilities.json)
- [Generated capability table](capabilities.md)

No track closes from a checklist alone. Each phase needs reviewed implementation, correct evidence, hosted gates where required, and an explicit recorded next-phase decision.

