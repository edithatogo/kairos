# Go Getting Started

**Runtime support:** Version and data helpers are distinct from execution. The former Go scheduler is unsupported; setting `CGO_ENABLED=1` does not create or qualify a native adapter.

Rust is the only supported simulation core. Follow [ADR 0010](../adr/0010-rust-only-execution.md), [installation](../install.md) and the [migration guide](../release/migration.md). The package checks below establish only their stated package/data scope.

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

## Native integration boundary

The versioned Rust C ABI is the intended integration surface. Native cgo support
requires an implementation, correct library/header linking and installed-consumer
acceptance. The source metadata package alone does not supply this runtime.

## Next steps

- Read the [Go binding README](../../bindings/go/README.md)
- Read the [FFI documentation](../ffi/ffi-guide.md)
