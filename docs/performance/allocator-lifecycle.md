# Allocator Lifecycle & Contention Budget

## Overview

KairoECS provides experimental reusable allocation primitives under the `numa` feature:
1. `EventPool<T>`: Reusable object pool for event instances.
2. `EventArena<T>`: Chunked lifecycle arena for bulk event creation and epoch reset.

## Ownership and contention boundary

The primitives use owned vectors. The caller owns placement and
synchronization.

The production scheduler uses a reusable heap and a pending-event set.
`Scheduler::with_capacity(peak)` reserves space up front when the
expected queued-event peak is known. The pending set reserves extra
storage for bucket reuse. Cancelled entries can remain in the heap
until pruning or compaction, and the scheduler grows as needed above
the initial reservation.

The reserved path made zero system allocator calls after warm-up and
retained zero requested bytes after drop in 320 measured serial and
parallel scheduler samples on Linux and macOS. These samples describe
independent schedulers at peaks of 256 and 2,048 queued entries.
[Allocation protocol and measured storage](../../conductor/tracks/50-numa-topology-affinity-memory-lifecycle/evidence/native-qualification/README.md#scheduler-allocation).

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
  - `created`: New pooled event wrappers when the recycled free list is empty.
  - `reused`: Checkouts served from the recycled free list.
  - `recycled`: Objects successfully returned to the free list.
  - `uses_global_lock`: Constant `false`; this is not a contention measurement.
- `EventArenaMetrics`:
  - `allocated_items`: Total active items in the current epoch.
  - `chunk_count`: Number of memory chunks allocated.
  - `reset_count`: Number of epoch resets performed.
  - `uses_global_lock`: Constant `false`.
