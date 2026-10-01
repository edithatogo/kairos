# Track 50 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Affinity binding is unsafe or sticky | Host instability | Opt-in API, typed errors, and reversible binding tests |
| Allocator introduces data races | Undefined behavior | Prefer safe Rust, narrow unsafe blocks, and concurrency tests |
| NUMA feature breaks non-NUMA hosts | Portability regression | Unsupported fallback tests |
| Zero-copy layout leaks lifetime bugs | FFI memory corruption | Alignment, ownership, and lifetime tests |
| Arena reuse hides stale events | Incorrect simulation | Generation and lifecycle validation |
