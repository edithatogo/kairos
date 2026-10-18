# Feature-Gated Graph Relations ECS Module

## Overview

Track 59 introduces graph relations into KairoECS behind the `graph-relations` feature flag on `kairo-ecs-game-theory`. Unlike traditional pointer-based tree structures, graph relations in KairoECS are modeled purely as contiguous component stores holding `EntityId` links.

## Architectural Invariants

The module enforces strict architectural rules:
1. **Zero Raw Pointers**: No raw pointers (`*const T`, `*mut T`) are permitted anywhere in the graph topology.
2. **Zero `Box` Allocations**: Nodes and edges do not own child nodes via heap boxes.
3. **Flat `ComponentStore` Layout**: Graph traversal scans contiguous arrays indexed by dense entity IDs, maximizing CPU cache locality.
4. **Cycle Safety**: Descendant traversal tracks visited entities to prevent infinite loops in cyclic or malformed topologies.

## Core Relational Components

```rust
use kairo_ecs_types::EntityId;

/// Represents a parent-child relationship in an extensive-form game tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChildOf(pub EntityId);

/// Represents a state transition edge leading to a target node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransitionTo(pub EntityId);
```

## Traversal Helpers

- `children_of(parent, child_of_store)`: Scans the flat `ComponentStore<ChildOf>` and yields all entity IDs that have `ChildOf(parent)`.
- `transition_target(edge, transition_store)`: Retrieves the target `EntityId` of a directed transition.
- `depth_first_descendants(root, child_of_store)`: Non-recursive, iterative depth-first traversal that visits all reachable descendants while safely detecting cycles.

## Verification

The graph relations implementation is verified by:
- `cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations`
- `node scripts/validation/validate-graph-relations-no-pointer-topology.mjs --self-test`: AST and regex scanner guaranteeing zero pointer or box allocations in graph relation definitions.
