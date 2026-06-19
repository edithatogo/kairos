# Track 51 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Arrow schema breaks binding consumers | Compatibility regression | Keep schema compatibility tests and version notes |
| HDF5/ADIOS2 libraries unavailable in CI | False failures or skipped proof | Feature-gate and require live evidence for `Done` |
| Checkpoint restore misses pending events | Incorrect restart | Restart parity tests with pending event queues |
| Filesystem benchmark is not reproducible | Misleading throughput | Record stripe, rank, block size, and checksum |
| Writer falls back to serialization copies | Violates zero-copy priority | Record copy boundaries and reject hidden conversions |
