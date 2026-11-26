# Next Native Language Adapter Contract (`K04.adapters`)

## Selected Consumer, Language & Operation

- **Language:** C# / .NET (`bindings/csharp`, `Kairo.ECS` v0.1.0, `net10.0` / `net11.0`)
- **Operation:** Packaged native C ABI v1 engine lifecycle and deterministic scheduling (`NativeMethods` + `KairoEcsEngineHandle`) for `linux-x64` and `osx-arm64`, matching the `native-core-v1` platform matrix.
- **Current status:** Local execution against `target/debug/libkairo_ecs_ffi.dylib` passes all 15 `Kairo.ECS.Tests` cases on `net11.0` (`KAIRO_ECS_REQUIRE_NATIVE=1`). Packaged NuGet/archive qualification and hosted `dotnet test` native CI remain pending.

## C ABI v1 Symbols & Error Contract

- **Version & Lifecycle:**
  - `kairo_ecs_ffi_version() -> u32` (must equal `1`; mismatch rejects before handle allocation)
  - `kairo_ecs_engine_new() -> u64` (returns `0` on failure)
  - `kairo_ecs_engine_free(handle: u64) -> KairoEcsStatusCode`
  - `kairo_ecs_engine_reset(handle: u64) -> KairoEcsStatusCode`
- **Scheduling & Stepping:**
  - `kairo_ecs_engine_current_time(handle: u64) -> u64`
  - `kairo_ecs_schedule_at(handle: u64, at_ticks: u64, priority: i32, kind: u32) -> u64`
  - `kairo_ecs_schedule_after(handle: u64, delta_ticks: u64, priority: i32, kind: u32) -> u64`
  - `kairo_ecs_cancel_event(handle: u64, event: u64) -> KairoEcsStatusCode`
  - `kairo_ecs_step(handle: u64) -> KairoEcsStatusCode`
  - `kairo_ecs_run_for(handle: u64, max_events: u64) -> KairoEcsStatusCode`
  - `kairo_ecs_run_until(handle: u64, time_limit_ticks: u64) -> KairoEcsStatusCode`
  - `kairo_ecs_run_until_or_for(handle: u64, time_limit_ticks: u64, max_events: u64) -> KairoEcsStatusCode`
- **Inspection & Error Diagnostics:**
  - `kairo_ecs_stats(handle: u64) -> KairoEcsStats`
  - `kairo_ecs_last_error_message() -> *const c_char` (thread-local UTF-8 diagnostic string)

## Ownership, Lifetime & Failure Semantics

1. **Handle ownership:** `KairoEcsEngineHandle` (`SafeHandle`) owns the `u64` Rust engine handle and invokes `kairo_ecs_engine_free` exactly once on `ReleaseHandle()`. Double-free returns `KAIRO_ECS_ERR_ALREADY_FREED` (`3`).
2. **Explicit library resolution:** `KAIRO_ECS_FFI_LIBRARY` must be an existing absolute shared-library path. Relative or missing paths fail closed without ambient `PATH`/`LD_LIBRARY_PATH` fallback.
3. **No host fallback:** Retired `DeterministicScheduler` throws immediately before mutating any simulation state.

## Next Bounded Implementation Leaves (<= 5 files)

1. **`K04.csharp.pack` (packaging + installed test):**
   - `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj`
   - `bindings/csharp/tests/Kairo.ECS.Tests/FfiSmokeTests.cs`
   - `.github/workflows/native-beta.yml` (or dedicated C# native CI job)
   - Acceptance: `dotnet pack` + clean consumer project referencing the packed `.nupkg` executes `KAIRO_ECS_REQUIRE_NATIVE=1` against the `native-core-v1` C ABI archive on `ubuntu-24.04` and `macos-15`.
2. **`K04.wasm.pack` (TypeScript/Wasm follow-on):**
   - `crates/kairo-ecs-wasm/src/lib.rs`, `bindings/typescript/scripts/native-wasm-smoke.mjs`
   - Acceptance: build `wasm32-unknown-unknown` / `wasm-bindgen` artefact and pass `KAIRO_ECS_WASM_MODULE=<path> npm run test:native`.
