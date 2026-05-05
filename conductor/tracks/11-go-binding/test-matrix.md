# Test Matrix — 11 Go Binding

## Required tests

- `go test ./...` for the Go binding coverage.
- `go test -run TestConformance ./...` or equivalent when Track 12 fixtures are wired in.
- `go test -race ./...` if the binding introduces concurrency-sensitive code.
- `go vet ./...` to catch static issues before package validation.
- `go mod tidy` to keep module metadata consistent before any future release work.

## Future-surface controls

- Do not add module proxy publishing, release tags, or credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen beyond the Go binding and local validation boundary.
- Stop at local test/vet/tidy validation until Track 12 owns parity and Track 15 owns dry-runs.

## CI command

```bash
go test ./... && go vet ./... && go mod tidy
```

