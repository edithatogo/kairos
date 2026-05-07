# 10 C# Binding .NET 10-11 — spec.md

## Mission

Provide C# bindings targeting net10.0 and net11.0 with SafeHandle ownership, P/Invoke, Arrow integration, tests, and NuGet package.

## Primary subagent

```text
csharp-agent
```

## Dependencies

```text
Track 02 FFI RC and Track 04 Arrow schema RC.
```

## Owned paths

```text
bindings/csharp/Kairo.ECS.sln, bindings/csharp/src/Kairo.ECS, bindings/csharp/tests/Kairo.ECS.Tests
```

Package publishing, registry, and release dry-run work is explicitly out of
scope for this binding slice.

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Stable C ABI from Track 02 FFI RC.
- Arrow schema from Track 04.
- Conformance fixtures from Track 12.
- Compatibility policy from Track 25.

## OS matrix note

NativeLibrary path resolution must be tested on the following OS targets due to divergent library naming and search paths:
- Windows (x64): `kairo_ecs.dll`
- Linux glibc (x64, arm64): `libkairo_ecs.so`
- Linux musl (x64): `libkairo_ecs.so` (static-aware fallback)
- macOS (x64, arm64): `libkairo_ecs.dylib`

## Outputs

- Implementation in owned paths exists and is wired to the workspace, including the C# solution, library project, and test project.
- Tests or test-plan.
- Docs updates.
- Release notes or compatibility notes when public surfaces change.

## Blocked paths

- Full `net11.0` validation remains experimental until stable .NET 11 SDK tooling is available in the target CI/local environment.
- Live native FFI smoke tests remain blocked until Track 02 supplies a stable native runtime artifact discoverable through `KAIRO_ECS_NATIVE_LIB_DIR` or `runtimes/{rid}/native/`.
- NuGet publishing, signing, registry credentials, and release dry-runs are out of scope for this binding track and remain owned by release-engineering tracks.

## C#/.NET version matrix

Required target frameworks:

```xml
<TargetFrameworks>net10.0;net11.0</TargetFrameworks>
```

Use:

```text
SafeHandle for native ownership
NativeLibrary for platform-specific loading
P/Invoke for C ABI calls
Apache.Arrow for telemetry reading
BenchmarkDotNet for performance smoke tests
DocFX for docs
```

If .NET 11 is preview in the active CI environment, mark that matrix lane experimental until stable tooling is available.



## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.

## Release implications

- Adds a public C# package surface for `Kairo.ECS` targeting `net10.0;net11.0`, including managed scheduler types, package metadata, native binding status reporting, and low-level FFI declarations.
- Local package validation creates `Kairo.ECS.0.1.0-preview.1.nupkg`, but this track does not publish, sign, or push that package to any registry.
- The `net11.0` lane and live native FFI execution must remain explicitly marked experimental/blocked until their upstream tooling/runtime artifacts are available.
