# Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles

This document describes the native discrete-event simulation (DES) and agent-based modeling (ABM) architecture for runtime linked entities, dynamic entity spawning, and cascaded lifecycle policies in KairoECS.

## Architecture Overview

KairoECS distinguishes generational ECS handles (`EntityId { index, generation }`) from stable external model identifiers (`ModelEntityId(u64)`). It supports typed 1-to-many relationships without pointer chasing or unsafe memory access.

```
+-----------------------------------------------------------+
|                      EntityHierarchy                      |
|                                                           |
|   +-------------------+          +--------------------+   |
|   |  ModelEntityId    |<-------->|     EntityId       |   |
|   +-------------------+          +--------------------+   |
|                                            |              |
|                     1-to-Many Typed Links  |              |
|                                            v              |
|        Parent  === [ChildLink: Kind, Policy] ===> Child   |
+-----------------------------------------------------------+
```

### 1. Stable External Identities vs Generational Handles

- **`EntityId`**: 64-bit dense index with 32-bit generation count. When an entity is despawned, its index is recycled and its generation incremented.
- **`ModelEntityId`**: Stable identifier mapping persistent domain identities (such as patient IDs, job tickets, or parcel identifiers) to live generational entities. Lookups automatically invalidate when the underlying entity is despawned.

### 2. Relationship Kinds and Typed Links

Links are typed via `RelationshipKind`:
- `ParentChild`: General hierarchical composition (e.g. facility to ward, unit to station).
- `SpawnedFrom`: Dynamic runtime lineage (e.g. mother to newborn, order to shipment).
- `SubtaskOf`: Task and job scheduling decomposition.
- `CargoOf`: Transient containment or transport binding.
- `Custom(u32)`: Domain-specific user extensibility.

### 3. Explicit Cascaded Lifecycle Policies

When a parent entity transitions or is despawned, its attached children are processed according to their configured `LifecyclePolicy`:
- `Survive`: Independent children survive parent termination. They are automatically detached and become independent root entities.
- `CascadeDelete`: Coupled children (and their cascaded descendants) are deterministically despawned along with the parent.
- `Detach`: Explicit decoupling before or during lifecycle events.

Cycle prevention is strictly enforced: attempting to attach an entity as its own child or creating an ancestral cycle returns `LinkedEntityError::CycleDetected`. Stale entity handles are validated and rejected before any hierarchy mutation.

### 4. Dynamic Spawning & Safe Mid-Iteration Buffering

When discrete-event agents execute their decision logic, spawning entities directly inside iteration loops can invalidate active queries or exceed allocation budgets.

KairoECS provides:
- **`SpawnBuffer`**: A bounded pre-allocated queue for collecting spawn requests during inner event loops. If the buffer capacity is reached, it fails safely with `LinkedEntityError::CapacityExceeded` without corrupting iteration.
- **`DynamicSpawner`**: Manages execution budgets, deterministic generation increments, and automatic derivation of child PRNG streams (`DeterministicStream`) from parent seeds, guaranteeing bit-exact reproducible replays across simulation runs.

### 5. Telemetry & Arrow Export

Hierarchy mutations produce `RelationshipLifecycleEvent` records:
- `Attached`
- `Detached`
- `Reparented`
- `CascadedDespawn`

These records are packaged into `RelationshipTelemetryBatch` containers adhering to schema `kairo.relationship.telemetry.v1` (`RELATIONSHIP_SCHEMA_VERSION = 1`), with full NDJSON and byte serialization for downstream Apache Arrow and Parquet ingest.
