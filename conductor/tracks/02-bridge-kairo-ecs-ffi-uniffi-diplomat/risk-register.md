# Risk Register — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Cross-platform ABI differences (MSVC/GCC/Clang on x86_64/aarch64) | 4 | 4 | 16 | CI matrix covering all target ABI/platform combinations; contract tests per target | bridge-agent | Any binding target fails CI after a compiler upgrade |
| Three-agent coordination overhead (ffi/uniffi/diplomat) | 4 | 3 | 12 | Single source-of-truth C API; agents consume shared header and regeneration rules | bridge-agent | Agents produce divergent surface for same header |
| UniFFI/Diplomat generated code divergence | 3 | 4 | 12 | Golden-file tests for generated output; diff-check against canonical header | bridge-agent | Golden-file test fails |
| Toolchain version lock | 4 | 3 | 12 | Document minimum and maximum supported versions; pin in CI but allow range in cargo | bridge-agent | Pinned version goes EOL without migration path |
| Missing ABI version negotiation semantics | 3 | 5 | 15 | Embed ABI version constant; library exports version query; reject at client load on mismatch | bridge-agent | Client crash traced to ABI mismatch |
