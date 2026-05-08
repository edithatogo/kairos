# Test Matrix — 11 Go Binding

## Required tests

| Gate | Command | Status | Notes |
|---|---|---|---|
| Unit/smoke | `go test ./...` | Passing | Covers version smoke, deterministic scheduler ordering, cancellation rejection for unknown/duplicate/dispatched IDs, explicit close lifecycle, and native-not-configured behavior. |
| Static analysis | `go vet ./...` | Passing | Run inside `bindings/go`. |
| cgo header smoke | `CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...` | Passing | Compiles `include/kairo_ecs.h` through cgo and verifies canonical status-code and ABI struct declarations without linking a native runtime library. |
| Pure-Go fallback | `CGO_ENABLED=0 go test ./...` | Passing | Confirms the preview facade remains usable where cgo is disabled. |
| Module metadata | `go mod tidy` | Passing | No external dependencies after tidy. |
| Conformance fixtures | `go test -run TestConformance ./...` | Passing | Deterministic ordering, cancellation, zero-delay guard, and RNG replay metadata fixtures are covered from `bindings/go`. |
| Race tests | `go test -race ./...` | Not required for this slice | The preview scheduler is single-engine/single-goroutine and introduces no shared concurrent access API. |

## Future-surface controls

- Do not add module proxy publishing, release tags, or credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen beyond the Go binding and local validation boundary.
- Stop at local test/vet/tidy validation until Track 12 owns parity and Track 15 owns dry-runs.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims module release ownership and records the no-release boundary.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` verifies Go cancellation guards and module metadata without native artifacts.
- `go test ./...`, `go vet ./...`, `CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...`, `CGO_ENABLED=0 go test ./...`, and `go mod tidy` remain the offline binding gates from `bindings/go`.

## CI command

```bash
go test ./...
go vet ./...
CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...
CGO_ENABLED=0 go test ./...
go mod tidy
```

## Validation notes

- Go toolchain observed locally: `go1.26.2 windows/amd64`.
- `go version` emitted a telemetry token permission warning under `%APPDATA%\go\telemetry`, but `go test`, `go vet`, and `go mod tidy` completed successfully from `bindings/go`.
- The sandboxed Go cache paths under `%LOCALAPPDATA%`, `C:\tmp`, and repo-local `target\go-cache` were denied in this run; `go test ./...` and `go vet ./...` passed after rerunning with approved normal Windows cache access.
- 2026-05-07 Track 11 pass: `go test ./...`, `go vet ./...`, `go mod tidy`, and `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` passed after restoring the pure-Go preview facade and executable fixture bridge.
- 2026-05-08 Track 11 pass: `go test ./...` initially hit the Windows Go build-cache sandbox denial, then passed with normal Windows cache access. `go vet ./...`, `go mod tidy`, `CGO_ENABLED=1 go test -run TestNativeHeaderSmokeCompilesStableCABI ./...`, and `CGO_ENABLED=0 go test ./...` passed.
- Native runtime calls remain blocked because this repo does not yet package a linkable `kairo-ecs-ffi` library for the Go module; the cgo gate is therefore a header-smoke boundary, not a runtime FFI execution claim.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
