# Handoff — 11 Go Binding

## Summary

The Go binding now has a minimal real module slice: a pure-Go deterministic scheduler facade matching the core event ordering contract, explicit handle lifecycle via `Close()`, tests, and an explicit native/cgo FFI not-configured path. No release or registry side effects were added.

## Files changed

- `bindings/go/kairoecs.go`
- `bindings/go/kairoecs_test.go`
- `bindings/go/README.md`
- `bindings/go/go.mod`
- `packaging/go/README.md`
- `conductor/tracks/11-go-binding/test-matrix.md`
- `conductor/tracks/11-go-binding/handoff.md`

## Contracts consumed

- Track 01 core type and scheduler contracts.
- Track 02 FFI contract for explicit native lifecycle/error boundaries.
- Track 12 conformance fixture contracts as a deferred fixture bridge.

## Contracts changed

- None outside the Go binding preview API surface.

## Tests added

- `TestSchedulerOrderingIsDeterministic`
- `TestCancellationSkipsEvent`
- `TestNativeFFIExplicitlyNotConfigured`
- `TestCloseIsExplicitAndIdempotent`
- Existing version/self-check smoke tests retained and widened for native status.

## Known risks

- Native/cgo integration remains blocked until Track 02 provides stable local FFI artifacts.
- Cross-language fixture parity remains deferred until Track 12 fixture runners are wired.
- This slice is a deterministic facade, not yet a wrapper over the Rust-owned engine.

## Integration notes

- Keep the track bounded to the Go package and local validation commands.
- Do not add proxy publication, module signing, or registry credentials here.
- Validation run from `bindings/go`: `go test ./...`, `go vet ./...`, `go mod tidy`.
- Local `go version` reported `go1.26.2 windows/amd64` after a telemetry token permission warning.
