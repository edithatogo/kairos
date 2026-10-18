# Track 64: Archetype/SoA ECS Storage, Full DES Process/Resource Kernel & ABM Spatial Topologies

## Objective

Close the core modeling ergonomics and performance gap against Bevy/Flecs, SimPy/Salabim/AnyLogic, and Agents.jl/Mesa by upgrading `kairo-ecs-state` to 64-byte aligned Structure-of-Arrays (SoA) columnar storage with multi-component queries, adding full DES resource/queue/process primitives to `kairo-ecs-des`, and adding 2D/3D grid, continuous spatial-hash/k-d tree, and CSR network spaces to `kairo-ecs-abm`.

## Inputs

- Existing contracts and scaffolds from Tracks 01, 03, 59, 62.
- Upstream ecosystem crate policies in `deny.toml` and `conductor/tech-stack.md`.
- Protected surface inventory in `docs/design/protected-surface-inventory.json`.

## Outputs

- `crates/kairo-ecs-state/`
- `crates/kairo-ecs-des/`
- `crates/kairo-ecs-abm/`
- `examples/flow/`
- `conductor/tracks/64-soa-ecs-des-resources-abm-spatial-kernel/`

## Owned paths

- `crates/kairo-ecs-state/`
- `crates/kairo-ecs-des/`
- `crates/kairo-ecs-abm/`
- `examples/flow/`
- `conductor/tracks/64-soa-ecs-des-resources-abm-spatial-kernel/`

## Blocked paths

- Do not modify unrelated track directories or bypass `deny.toml` license and advisory checks when adding upstream dependencies.
- Do not claim production HPC cluster scaling from single-host or free-tier smoke runs.
- Do not break existing deterministic conformance fixtures (`scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`).

## Dependencies

- Depends on Tracks: 01, 03, 59, 62

## Parallel-safe tracks

- Can execute in parallel with tracks outside `crates/kairo-ecs-state/, crates/kairo-ecs-des/` provided shared contracts in `conductor/contracts/` remain unchanged or are updated via ADR.

## Acceptance criteria

- Gate `phase-closeout-check` passes with recorded command receipts.
- Gate `soa-column-query-simd-parity` passes with recorded command receipts.
- Gate `des-resource-preemption-reneging-simpy-parity` passes with recorded command receipts.
- Gate `abm-spatial-grid-continuous-network-parity` passes with recorded command receipts.
- Gate `hybrid-des-abm-100k-benchmark` passes with recorded command receipts.
- All unit, integration, and conformance tests pass under `just ci` and Conductor validators.

## Quality gates

- `phase-closeout-check`
- `soa-column-query-simd-parity`
- `des-resource-preemption-reneging-simpy-parity`
- `abm-spatial-grid-continuous-network-parity`
- `hybrid-des-abm-100k-benchmark`

## Release implications

- Promotes KairoECS from dependency-free R2 contract scaffolds to R3/R4 ecosystem-integrated runtime execution for Archetype/SoA ECS Storage, Full DES Process/Resource Kernel & ABM Spatial Topologies.
- Any new external Cargo or language dependencies introduced in this track must pass `cargo deny`, `dependency-review.yml`, and SBOM generation before RC/1.0 release signoff.
