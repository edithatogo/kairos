# Allocator Lifecycle & Contention Budget

## Overview

High-throughput discrete-event simulation (DES) and agent-based modeling (ABM) workloads generate massive allocations and deallocations during the simulation lifecycle. To avoid memory fragmentation and lock contention on global allocators, KairoECS provides dedicated cache-local allocation primitives under the `numa` feature:
1. `EventPool<T>`: Reusable object pool for event instances.
2. `EventArena<T>`: Chunked lifecycle arena for bulk event creation and epoch reset.

## Zero Global Lock Contention

Both `EventPool` and `EventArena` operate within thread-local or LP-local memory boundaries without acquiring global mutexes or synchronizing across distinct CPU cores.

```rust
use kairo_ecs_core::{EventPool, EventArena};

// Thread-local event pool
let mut pool = EventPool::with_capacity(1024);
let event = pool.checkout("payload");
pool.recycle(event);

assert!(!pool.metrics().uses_global_lock);

// Chunked lifecycle arena
let mut arena = EventArena::new(256);
arena.alloc(42);
arena.reset();

assert!(!arena.metrics().uses_global_lock);
```

## Scaling Metrics for Track 55

Both allocators export structured telemetry for weak and strong scaling validation:
- `EventPoolMetrics`:
  - `created`: Total object allocations when pool capacity was exceeded.
  - `reused`: Fast reuse hits from preallocated pool slots.
  - `recycled`: Objects successfully returned to the free list.
  - `uses_global_lock`: Constant `false`, verifying lock-free execution.
- `EventArenaMetrics`:
  - `allocated_items`: Total active items in the current epoch.
  - `chunk_count`: Number of memory chunks allocated.
  - `reset_count`: Number of epoch resets performed.
  - `uses_global_lock`: Constant `false`.
