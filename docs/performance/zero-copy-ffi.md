# Zero-Copy FFI Layout & Memory Governance

## Overview

KairoECS exposes high-performance C ABI and language binding surfaces across Python, R, Julia, TypeScript, C#, and Go. In high-frequency simulation runs, serializing state arrays across FFI boundaries creates severe bandwidth and latency bottlenecks. Track 50 formalizes zero-copy buffer sharing rules and validation contracts under the `numa` feature.

## Layout Specification

`KairoEcsZeroCopyLayout` provides an optional check of declared layout metadata:

This helper validates declared metadata only. It cannot establish that an
address refers to a live allocation of the declared size, prove ownership or
lifetime, or demonstrate that a binding actually avoids copying. Those remain
the responsibility of the calling binding and its native qualification.

```rust
use kairo_ecs_ffi::{KairoEcsZeroCopyLayout, KairoEcsOwnership, KairoEcsLayoutStatus};

let layout = KairoEcsZeroCopyLayout::new(
    ptr_addr,
    byte_length,
    alignment_bytes,
    KairoEcsOwnership::Borrowed,
);

let status = layout.validate();
assert_eq!(status, KairoEcsLayoutStatus::KAIRO_ECS_LAYOUT_OK);
```

### Invariants & Validation Rules
1. **Non-Null Address**: Pointer addresses must be non-zero (`KAIRO_ECS_LAYOUT_ERR_NULL`).
2. **Non-Empty Length**: Buffer length and alignment must be greater than zero (`KAIRO_ECS_LAYOUT_ERR_EMPTY`).
3. **Alignment Boundary**: Buffer address must be an exact multiple of the declared alignment (`KAIRO_ECS_LAYOUT_ERR_MISALIGNED`). The caller chooses the declared alignment.
4. **Explicit Ownership**: Buffers must declare explicit ownership (`Borrowed`, `OwnedByRust`, `OwnedByCaller`). Unspecified ownership is rejected (`KAIRO_ECS_LAYOUT_ERR_OWNERSHIP`).

## UniFFI and Diplomat Handoff Boundaries

- **UniFFI and Diplomat**: Binding owners must qualify the concrete buffer,
  copy behavior and lifetime contract before claiming zero-copy support.

## Prohibited Serialization Fallbacks

The metadata validator returns typed errors for invalid declarations. It does
not enforce serialization or fallback behavior across binding implementations.
