# Linear Issue Plan

The connected Linear workspace already has a `Conductor Next` project, which can host these tracks if desired.

Recommended issue hierarchy:

```text
Project: KairoECS Engine
Epic/Milestone: Wave 0 Foundation
  - Track 00 Foundation/Governance/Naming
  - Track 13 CI/CD skeleton
  - Track 14 Docs skeleton
  - Track 16 Release governance
Epic/Milestone: Wave 1 Contracts
  - Track 01 Core contracts
  - Track 02 FFI contract
  - Track 04 Arrow schema contract
  - Track 12 Conformance fixture design
Epic/Milestone: Wave 2 Core
  - Scheduler
  - ECS
  - RNG
  - DES/ABM surfaces
Epic/Milestone: Wave 3 Bindings
  - Python 3.10-3.14
  - R
  - Julia
  - TypeScript/Wasm
  - C# .NET 10-11
  - Go
Epic/Milestone: Wave 4 Release
  - Packaging
  - Docs publish
  - Registry dry-runs
  - Release candidate
```

Labels should mirror `conductor/maintenance-governance.md`.
