# Kairos Conductor index

## Ecosystem boundaries and executable work

[ADR 0011](../docs/adr/0011-ecosystem-boundaries.md) defines the simulation/game/domain/HEOR
boundaries. [Consumer inventory](ecosystem-consumers.md) records actual source
dependencies, [movement and usage](contracts/movement-resource-usage.md) defines
the next neutral contracts, and the [bounded workpack](execution/README.md)
provides small tasks with exact scopes and commands. Track 114 coordinates the
cross-repository handoffs; implementation and consumer pin qualification remain
with their source owners. Routine tasks use proportionate checks under the
updated workflow, without new human approval gates.

## Rust-only maturity programme — 7 November 2026

[Approved programme](frontier-programme-20261107.md) coordinates Tracks 109–115. [Capability evidence](capabilities.md) governs current public claims, [release gates](release-gates.md) define beta/stable scope, and [scientific completion](scientific-completion.md) retains every outstanding method as an explicit obligation. [Native-only policy](native-only-policy.md) applies immediately. [Performance](performance-programme.md), [dependencies and CareOps contracts](dependency-and-consumer-contracts.md), and [validation datasets](validation-datasets.md) provide executable follow-up plans.

Historical track completion is preserved as source-specific evidence, not inherited qualification. New phases remain pending until actual implementation, independent review and runtime evidence are recorded.

Use the [track registry](tracks.yaml) as the status and ownership source, then read
the selected track's specification, plan, agent contract, test matrix and handoff.
A status label alone does not prove implementation or release acceptance.

- [Product definition](product.md)
- [Product guidelines](product-guidelines.md)
- [Technical stack](tech-stack.md)
- [Implementation workflow](workflow.md)
- [Track index](tracks.md)
- [Ownership and dependency map](track-map.md)
- [Parallel execution](parallel-execution.md)
- [Quality gates](quality-gates.md)
- [Implementation readiness](implementation-readiness.md)
- [Phase closeout ledger](phase-closeout.yaml)
- [Current status evidence](status.md)

- [Pending CareOps composition owner plan](design/careops-composition-plan-20261020.md)

## Polars migration

[Track 70](tracks/70-polars-native-data-processing/index.md) completed upstream Phases 0–4 in Kairos, including native tables, Arrow C Data Schema v2, the Python Polars handoff and durable publication. CareOps `polars_data_processing_20261022` remains a separate downstream adoption; its source, interoperability and consumer acceptance must be qualified before use. Dependencies 04/62/63 provide routing context, not runtime proof.

## Separate library adoption tracks

[Library adoption programme](library-adoption.md) registers rand, rand_distr, ECharts, petgraph and Three.js separately, after accepted Polars delivery in both repositories. All implementation remains pending.

## Robustness-first dependency decisions

[Separate dependency tracks and sequencing](robust-dependencies.md) prioritise existing-system robustness. [Value gates](dependency-value-policy.md) prevent adoption for its own sake. SUMO conditional; Rapier deferred.

- [Dependency sequencing and prerequisite gates](dependency-sequencing.md)

[Validation/tooling planning review](validation-tooling-review.md)

## Python runtime migration

[Python 3.15 migration and qualification](tracks/105-python315-runtime-qualification/index.md) records the accepted Python 3.15.1 baseline, current provider follow-through and [revised dependency policy](dependency-value-policy.md). Necessary qualified compatibility profiles are allowed; historical receipts are preserved.
