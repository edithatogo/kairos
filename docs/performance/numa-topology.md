# NUMA Topology & CPU Core Affinity

## Overview

KairoECS provides feature-gated NUMA (Non-Uniform Memory Access) topology discovery and CPU core affinity management behind the `numa` feature flag. The runtime preserves `#![forbid(unsafe_code)]` workspace-wide, ensuring that memory management and thread pinning adhere to strict safety invariants without invoking unsafe C-FFI bindings.

## Topology Discovery

NUMA topology discovery is provided by `kairo_ecs_core::NumaTopology`:

```rust
use kairo_ecs_core::{NumaTopology, NumaSupport, NumaError};

let topology = NumaTopology::discover();
match topology.support {
    NumaSupport::Supported => {
        println!("Discovered {} NUMA nodes via {}", topology.nodes.len(), topology.source);
        for node in &topology.nodes {
            println!("Node {}: cores {:?}, memory: {:?}", node.id, node.cores, node.memory_bytes);
        }
    }
    NumaSupport::Unsupported => {
        println!("NUMA discovery unsupported on host: {}", topology.source);
    }
}
```

### Supported Discovery Modes
1. **Synthetic override.** An explicit `KAIRO_NUMA_NODES` count from 1 through 65536 takes precedence. The generated CPU IDs and memory sizes are synthetic.
2. **Linux sysfs.** The runtime reads host node IDs, logical CPU lists and available node memory totals from `/sys/devices/system/node`. A node can have memory but no CPUs. A thread's permitted CPU set can be narrower than the host topology.
3. **Fallback.** If the runtime cannot discover a topology, it returns `NumaSupport::Unsupported`.

## CPU affinity

On Linux, binding uses `/usr/bin/taskset` and verifies the result against
`/proc/thread-self/status`. Thread ID zero selects the calling thread. An explicit
ID must match that same thread. Select a CPU in the thread's current allowed mask.

```rust
use kairo_ecs_core::{AffinityBindingPlan, NumaError};

fn run_on_cpu(cpu: u32, work: impl FnOnce() -> Result<(), NumaError>) -> Result<(), NumaError> {
    let plan = AffinityBindingPlan::new(0, cpu, true)?;
    plan.apply()?;
    let result = work();
    plan.revert()?;
    result
}
```

Restore on the same thread before returning, including error paths. Callers whose
work can unwind must arrange cleanup during unwinding. Dropping a plan does
not restore the mask. A second bind is rejected while restoration is pending.
If the kernel restricts the mask during a binding, exact restoration can fail;
the original mask remains available for a subsequent restoration attempt.
