# Test Matrix — 11 Go Binding

## Required tests

| Gate | Command | Status | Notes |
|---|---|---|---|
| Unit/smoke | `go test ./...` | Passing | Covers version smoke, deterministic scheduler ordering, cancellation rejection for unknown/duplicate/dispatched IDs, explicit close lifecycle, and native-not-configured behavior. |
| Static analysis | `go vet ./...` | Passing | Run inside `bindings/go`. |
| Module metadata | `go mod tidy` | Passing | No external dependencies after tidy. |
| Conformance fixtures | `go test -run TestConformance ./...` | Deferred | Track 12 fixtures are not wired into the Go binding yet. |
| Race tests | `go test -race ./...` | Not required for this slice | The preview scheduler is single-engine/single-goroutine and introduces no shared concurrent access API. |

## Future-surface controls

- Do not add module proxy publishing, release tags, or credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen beyond the Go binding and local validation boundary.
- Stop at local test/vet/tidy validation until Track 12 owns parity and Track 15 owns dry-runs.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims module release ownership and records the no-release boundary.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` verifies Go cancellation guards and module metadata without native artifacts.
- `go test ./...`, `go vet ./...`, and `go mod tidy` remain the offline binding gates from `bindings/go`.

## CI command

```bash
go test ./...
go vet ./...
go mod tidy
```

## Validation notes

- Go toolchain observed locally: `go1.26.2 windows/amd64`.
- `go version` emitted a telemetry token permission warning under `%APPDATA%\go\telemetry`, but `go test`, `go vet`, and `go mod tidy` completed successfully from `bindings/go`.
- The sandboxed Go cache paths under `%LOCALAPPDATA%`, `C:\tmp`, and repo-local `target\go-cache` were denied in this run; `go test ./...` and `go vet ./...` passed after rerunning with approved normal Windows cache access.

