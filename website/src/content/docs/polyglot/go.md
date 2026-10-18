---
title: Go
description: Go 1.24–1.26 cgo bindings for distributed microservices and digital-twin orchestration.
---

The Go binding (`bindings/go/`) connects Go 1.24–1.26 services to `libkairo_ecs_ffi` via `cgo`, supporting cloud-native digital-twin orchestration and gRPC/Kafka streaming pipelines.

## Go Package Overview

| Function / Type | Purpose |
| --- | --- |
| `kairo.NewSimulation(seed uint64)` | Allocate a native KairoECS simulation context |
| `(*Simulation).Step(steps uint64) (uint64, error)` | Advance the simulation deterministically |
| `(*Simulation).SnapshotDigest() uint64` | Compute the deterministic FNV-1a / xxHash state digest |
| `(*Simulation).Close()` | Release native Rust memory safely |

## Example

```go
package main

import (
	"fmt"
	"github.com/edithatogo/kairos/bindings/go/kairo"
)

func main() {
	sim, err := kairo.NewSimulation(42)
	if err != nil {
		panic(err)
	}
	defer sim.Close()

	now, _ := sim.Step(500)
	fmt.Printf("SimTime=%d Digest=0x%x\n", now, sim.SnapshotDigest())
}
```

Canonical sources:

- `bindings/go/README.md`
- `docs/tutorials/go-getting-started.md`
