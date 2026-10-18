# Zero-Copy FFI Layout & Memory Governance

## Overview

KairoECS exposes high-performance C ABI and language binding surfaces across Python, R, Julia, TypeScript, C#, and Go. In high-frequency simulation runs, serializing state arrays across FFI boundaries creates severe bandwidth and latency bottlenecks. Track 50 formalizes zero-copy buffer sharing rules and validation contracts under the `numa` feature.

## Layout Specification

Zero-copy buffers passed across FFI boundaries are validated using `KairoEcsZeroCopyLayout`:

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
3. **Alignment Boundary**: Buffer address must be an exact multiple of the declared alignment (`KAIRO_ECS_LAYOUT_ERR_MISALIGNED`). Standard SIMD and vector alignments (16, 32, 64 bytes) are enforced.
4. **Explicit Ownership**: Buffers must declare explicit ownership (`Borrowed`, `OwnedByRust`, `OwnedByCaller`). Unspecified ownership is rejected (`KAIRO_ECS_LAYOUT_ERR_OWNERSHIP`).

## UniFFI and Diplomat Handoff Boundaries

- **UniFFI**: Used for higher-level bindings (Python, Swift, Kotlin). Record batches and component arrays are passed via structured descriptors rather than deep object graphs.
- **Diplomat**: Used for C-compatible zero-overhead bindings (C#, C, WASM). Pointers are borrowed and wrapped in RAII container types on the foreign host.

## Prohibited Serialization Fallbacks

In performance-critical loops (such as state queries, trajectory logging, and Arrow record batch reads), copying through intermediate JSON, MsgPack, or protobuf buffers is strictly prohibited. If alignment or ownership invariants fail, the runtime fails fast with typed error codes rather than degrading into invisible slow-path memory copies.
