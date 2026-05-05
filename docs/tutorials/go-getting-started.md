# Go Getting Started

## Prerequisites
- Go >= 1.23 installed
- The KairoECS repository cloned

## Quickstart

### 1. Tidy the module
```bash
cd bindings/go
go mod tidy
```

### 2. Verify the package surface
```go
package main

import (
    "fmt"
    "github.com/edithatogo/kairos/bindings/go"
)

func main() {
    fmt.Println(kairoecs.Version)
    fmt.Println(kairoecs.SelfCheck())
}
```

### 3. Run tests
```bash
go test ./...
go vet ./...
```

## Package structure

| File | Purpose |
|---|---|
| `kairoecs.go` | Package source (Version constant, SelfCheck function) |
| `kairoecs_test.go` | Unit tests |
| `go.mod` | Module manifest |

## cgo integration

The Go binding will use `cgo` to link against the C ABI library:
- Set `CGO_ENABLED=1` for native builds
- The C headers are in `include/kairo_ecs.h`

## Cross-compilation

For cross-platform builds, set `CGO_ENABLED=1` and `CC` to the target cross-compiler:
```bash
GOOS=linux GOARCH=amd64 CGO_ENABLED=1 go build ./...
```

## Next steps

- Read the [Go binding README](../../bindings/go/README.md)
- Read the [FFI documentation](../ffi/ffi-guide.md)
