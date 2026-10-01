# Track 52 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| GPU backend silently falls back to CPU | False acceleration claim | Typed backend mode and hardware evidence |
| Device memory is copied every tick | No HPC benefit | Persistent buffer lifecycle tests |
| CUDA and wgpu diverge semantically | Incorrect results | Shared CPU parity fixtures |
| Driver/runtime unavailable in CI | Blocked validation | Separate local compile gates from live hardware gates |
| Unsafe device interop corrupts host state | Data loss | Narrow unsafe boundaries and checksum tests |
