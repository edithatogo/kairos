# Risk Register — 08 Julia Binding

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| `ccall` type marshalling bugs | 3 | 5 | 15 | Autogenerate `ccall` signatures from Track 02 header via `Clang.jl`; add fuzz harness with `@ccall` roundtrip for every FFI handle type | julia-agent | Fuzz harness detects any type mismatch |
| Julia LTS vs rolling version skew | 3 | 3 | 9 | CI matrix: julia-1.10 (LTS), julia-1 (latest stable), julia-nightly (allowed failure); gate release on LTS + stable green | julia-agent | LTS or stable lane fails |
| Pkg artifact image compilation overhead | 4 | 3 | 12 | Precompile baking in CI; set `JULIA_CPU_TARGET=generic`; benchmark `@time using KairoECS` < 30s on cold cache | julia-agent | Cold-cache load time exceeds 30s |
| Aqua.jl compatibility drift | 3 | 2 | 6 | Run `Aqua.test_all(KairoECS)` in CI; allow minor version range; gate on `strict=true` only for release candidates | julia-agent | Aqua test fails on release candidate |
| BinaryBuilder artifact platform gaps | 3 | 3 | 9 | Tier support policy: linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, win64 required; other platforms build but are allowed-fail | julia-agent | Any required platform fails to build |
