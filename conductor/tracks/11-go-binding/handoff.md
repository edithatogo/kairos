# Handoff — 11 Go Binding

## Summary

The Go binding now has a minimal real module slice: a pure-Go deterministic scheduler facade matching the core event ordering contract, explicit handle lifecycle via `Close()`, scheduler stats, executable conformance fixture checks, tests, and a cgo header-smoke boundary over the stable C ABI. Native runtime calls remain explicitly not configured until a linkable local `kairo-ecs-ffi` library is packaged for Go. No release or registry side effects were added.

## Files changed

- `bindings/go/kairoecs.go`
- `bindings/go/kairoecs_test.go`
- `bindings/go/conformance_test.go`
- `bindings/go/native_cgo.go`
- `bindings/go/native_nocgo.go`
- `bindings/go/native_cgo_test.go`
- `bindings/go/README.md`
- `bindings/go/go.mod`
- `conductor/tracks/11-go-binding/test-matrix.md`
- `conductor/tracks/11-go-binding/handoff.md`

## Contracts consumed

- Track 01 core type and scheduler contracts.
- Track 02 FFI contract and canonical C header for explicit native lifecycle/error boundaries.
- Track 12 conformance fixture contracts as a deferred fixture bridge.

## Contracts changed

- None outside the Go binding preview API surface.

## Tests added

- `TestSchedulerOrderingIsDeterministic`
- `TestCancellationSkipsEvent`
- `TestCancellationRejectsUnknownDuplicateAndDispatchedEvent`
- `TestStatsTrackScheduledCancelledDispatchedAndPending`
- `TestConformanceDeterministicOrdering`
- `TestConformanceCancellation`
- `TestConformanceZeroDelayGuard`
- `TestNativeHeaderSmokeCompilesStableCABI`
- `TestNativeFFIExplicitlyNotConfigured`
- `TestCloseIsExplicitAndIdempotent`
- Existing version/self-check smoke tests retained and widened for native status.

## Known risks

- Native runtime integration remains blocked until a stable local `kairo-ecs-ffi` library artifact is packaged for the Go module.
- Cross-language fixture parity beyond the bootstrap scheduler fixtures remains deferred until Track 12 fixture runners are wired.
- This slice is a deterministic facade, not yet a wrapper over the Rust-owned engine.

## Integration notes

- Keep the track bounded to the Go package and local validation commands.
- Do not add proxy publication, module signing, or registry credentials here.
- Validation run from `bindings/go`: `go test ./...`, `go vet ./...`, `CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...`, `CGO_ENABLED=0 go test ./...`, `go mod tidy`.
- The cancellation hardening validation was also covered by `conductor/tracks/06-python-binding-310-314/validate-bindings06-11.ps1`.
- 2026-05-07 validation reran `go test ./...`, `go vet ./...`, `go mod tidy`, and the cross-binding static validator successfully.
- 2026-05-08 validation reran the Go gates successfully. The first sandboxed `go test ./...` hit `%LOCALAPPDATA%\go-build` access denial, then passed with normal Windows Go cache access.
- Local `go version` reported `go1.26.2 windows/amd64` after a telemetry token permission warning.
- No release, registry, or remote publication side effects were performed.

## Follow-up issues

Package a linkable native runtime artifact for Go before enabling `NewNativeEngine()` or claiming native FFI runtime coverage.
## Phase closeout evidence

`$conductor-review` completed in this pass. Accepted fixes: add the cgo header-smoke gate and document the native runtime blocker. No rejected in-scope fixes remain. Phase closeout is not fully clean because the shared worktree contains unrelated worker edits outside Track 11. Before Done closeout, record commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
