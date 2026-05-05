# Risk Register — 11 Go Binding

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| cgo cross-compilation overhead (`CGO_ENABLED` per target) | 4 | 4 | 16 | CI builds natively on each target; linux/amd64, linux/arm64, windows/amd64, macos/amd64, macos/arm64 as required targets; CGO_ENABLED=0 lane for pure-Go fallback | go-agent | Any required target platform build fails |
| C linker dependency in CI | 3 | 3 | 9 | Pin runner images in CI; add `CGO_LDFLAGS_ALLOW` for sanitizer flags; `ldd` / `otool -L` verification on built artifacts | go-agent | CGO build fails on any CI runner |
| Go module proxy unavailability | 3 | 3 | 9 | Vendor dependencies with `go mod vendor`; CI uses `GOPROXY=off` after vendor step; document enterprise env configuration | go-agent | `go mod vendor` or `GOFLAGS=-mod=vendor` fails |
| Go versioning semantics (`/v2` breaking changes) | 3 | 5 | 15 | Version the module path as `module github.com/org/kairo-ecs-go/v2` from first v2 tag; use `go.work` for local multi-module development | go-agent | v2+ import path mismatch detected in any consumer |
| `runtime.SetFinalizer` vs explicit close semantics | 3 | 5 | 15 | Prefer explicit `Close()` with `defer` over finalizer-only cleanup; add `-race` flag to all test lanes; `runtime.KeepAlive` wrappers on hot paths | go-agent | Race detector flags any native-handle path |
