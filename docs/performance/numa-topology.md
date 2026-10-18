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
1. **Linux sysfs (`/sys/devices/system/node`)**: When running on Linux hosts with NUMA hardware, the runtime scans nodes, active cores, and memory capacity directly from the kernel filesystem.
2. **Environment Variable Override (`KAIRO_NUMA_NODES`)**: For HPC batch runners (Slurm/PBS) or containerized environments, setting `KAIRO_NUMA_NODES=<count>` provides synthetic NUMA domain definitions matching the job cgroup allocation.
3. **Typed Fallback**: On platforms without NUMA support (such as macOS, development laptops, or non-NUMA VMs), the discovery function returns `NumaSupport::Unsupported` with typed `NumaError::UnsupportedHost`, preventing silent misconfiguration or panics.

## CPU Core Affinity

CPU affinity is modeled via `AffinityBindingPlan`:

```rust
use kairo_ecs_core::{AffinityBindingPlan, AffinityAction, NumaError};

// Affinity requires explicit opt_in = true
let plan = AffinityBindingPlan::new(thread_id, core_id, true)?;

// Plan provides an exact reversal plan for cleanup
let reversal = plan.reversal();
assert_eq!(reversal.action, AffinityAction::RestorePrevious);

// Applying affinity is safe and returns typed unsupported status on non-supported platforms
let result = plan.apply();
```

Affinity bindings are:
- **Opt-in**: Setting `opt_in = false` fails at creation time with `NumaError::InvalidAffinity`.
- **Observable**: Plans expose target thread, target core, and scheduled action.
- **Reversible**: Every plan generates a paired `reversal()` plan restoring previous thread state.
