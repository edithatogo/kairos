# Track 67 Plan: Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles

## Phase 0 - Core Entity Relations & Dynamic Spawning Contracts

- [ ] Task 0.1: Define `LinkedEntityPair`, `ParentChildBinding`, and `EntitySpawningRelationship` in `crates/kairo-ecs-core`.
- [ ] Task 0.2: Implement dynamic entity instantiation during event handlers without invalidating concurrent archetype iteration in `crates/kairo-ecs-state`.

## Phase 1 - Cascaded Lifecycle & Shared Context Progression

- [ ] Task 1.1: Implement coupled vs independent transition policies (co-movement, separate egress, shared components).
- [ ] Task 1.2: Add cascaded despawn and terminal state cleanup with causal conservation invariants.

## Phase 2 - Arrow Telemetry & ABM Spatial Integration

- [ ] Task 2.1: Implement dual-key composite entity recording in `crates/kairo-ecs-arrow` zero-copy stream.
- [ ] Task 2.2: Add spatial binding so spawned child entities inherit initial coordinates or relative offsets.

## Phase 3 - Closeout

- [ ] Task 3.1: Run track-specific test matrix, workspace tests, and Conductor validators.
- [ ] Task 3.2: Run `$conductor-review`, auto-apply accepted fixes, and update `handoff.md`.
- [ ] Task 3.3: Commit and push the cleaned slice and verify GitHub Actions.
