# Go Style Guide

- Use cgo over the stable C ABI.
- Expose explicit `Close()` methods for native handles.
- Run `go test`, `go vet`, `gofmt`, `staticcheck`, and `golangci-lint`.
- Include cgo caveats in docs and avoid per-event Go callbacks in hot paths.
- Use semantic Git tags for module releases.
