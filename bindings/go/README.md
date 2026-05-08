# Go Binding

Track 11 owns this binding. The current module is a preview facade that gives Go callers the deterministic scheduler semantics required by the core contract while the stable native C ABI is still being integrated.

## Supported toolchain

- Go 1.23 or newer for the current scaffold.
- cgo is used for a header compatibility smoke test when `CGO_ENABLED=1`.
- Native FFI runtime calls are deliberately not enabled by default because no linkable local `kairo-ecs-ffi` library artifact is packaged for this module yet.

## Current API slice

- `NewEngine()` creates a pure-Go preview scheduler.
- `ScheduleAt()` and `ScheduleAfter()` enqueue events.
- `Step()` dispatches one event using `(time_ticks ASC, priority ASC, sequence ASC)`.
- `RunFor()` dispatches a bounded number of events.
- `CancelEvent()` skips a scheduled event.
- `Close()` explicitly closes an engine and is safe to call more than once.

## Native FFI status

`NativeHeaderSmoke()` compiles the stable C ABI header through cgo and checks the status-code and struct declarations. `NativeAvailable()` still returns `false` and `NewNativeEngine()` returns `ErrNativeNotConfigured` until a stable, locally discoverable `kairo-ecs-ffi` library is packaged for Go. This prevents accidental claims that native calls are active when the runtime dependency is absent.

## Local validation

```bash
go test ./...
go vet ./...
CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...
CGO_ENABLED=0 go test ./...
```
