# Go binding

Track 11 owns this package. The pure-Go scheduler is retired immediately.
`NewEngine()` now returns `(*Engine, error)`, matching `NewNativeEngine()`;
both return `nil, ErrNativeNotConfigured` until a qualified Rust bridge exists.
This is a breaking API change. An `Engine` zero value cannot bypass the native
requirement, and all execution methods fail explicitly.

Package metadata, Event/Stats DTOs and cgo header compatibility remain
available. `NativeAvailable()` returns false; `SelfCheck()` reports
`metadata_only`. Compiling `include/kairo_ecs.h` does not load a Rust engine.
No alternate core or fallback is supported.

Go runtime support requires a Rust C ABI adapter with owned handles, checked
integer conversion, explicit status/error propagation, native artifact loading,
and actual scheduling/lifetime/conformance tests. It must remain unavailable
until that work is complete. Existing fixture tests only verify metadata.

Validation with Go 1.23 or newer:

```bash
go test ./...
go vet ./...
CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...
CGO_ENABLED=0 go test ./...
```

Passing these checks qualifies the current metadata and failure boundary, not
simulation capability or a native runtime release.
