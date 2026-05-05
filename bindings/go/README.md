# Go Binding

Track 11 owns this binding. The current module is a preview facade that gives Go callers the deterministic scheduler semantics required by the core contract while the stable native C ABI is still being integrated.

## Supported toolchain

- Go 1.23 or newer for the current scaffold.
- cgo/native FFI is deliberately not enabled by default.

## Current API slice

- `NewEngine()` creates a pure-Go preview scheduler.
- `ScheduleAt()` and `ScheduleAfter()` enqueue events.
- `Step()` dispatches one event using `(time_ticks ASC, priority ASC, sequence ASC)`.
- `RunFor()` dispatches a bounded number of events.
- `CancelEvent()` skips a scheduled event.
- `Close()` explicitly closes an engine and is safe to call more than once.

## Native FFI status

`NativeAvailable()` returns `false` and `NewNativeEngine()` returns `ErrNativeNotConfigured` until Track 02 provides a stable, locally discoverable `kairo-ecs-ffi` library and header. This prevents accidental claims that cgo is active when the native dependency is absent.

## Local validation

```bash
go test ./...
go vet ./...
```
