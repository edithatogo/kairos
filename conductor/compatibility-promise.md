# Compatibility Promise

This file prevents accidental over-promising.

| Surface | Pre-1.0 promise | 1.0 target |
|---|---|---|
| `kairo-ecs-core` Rust API | may change with changelog | semver respected |
| `kairo-ecs-state` Rust API | may change with changelog | semver respected |
| C ABI | versioned once public preview begins | stable within major ABI version |
| UniFFI/Diplomat facades | experimental until beta | generated outputs reproducible |
| Python API | preview until conformance suite passes 3 releases | semantic versioning |
| R API | preview until Arrow + native lifecycle stable | semantic versioning |
| Julia API | preview until artifact delivery stable | semantic versioning |
| TypeScript/Wasm API | preview until browser/node matrix stable | semantic versioning |
| C# API | .NET 10 stable, .NET 11 preview until GA | semantic versioning |
| Go API | preview until native library distribution finalized | semantic versioning |
| Arrow schemas | versioned from first telemetry preview | schema migration policy |
| Scenario manifests | experimental until experiment runner beta | versioned schema |
