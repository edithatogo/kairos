# Release Critical Path

This file resolves the red-team concern that the roadmap is too large to ship.

## v0.1 hero path

The first public release should prove the central thesis without trying to complete every binding.

Required:

1. `kairo-ecs-types`
2. `kairo-ecs-core`
3. `kairo-ecs-state`
4. `kairo-ecs-ffi` C ABI preview
5. `kairo-ecs-arrow` event-log preview
6. Python 3.10-3.14 preview package
7. one DES example
8. one ABM example
9. one hybrid example
10. conformance fixture harness
11. docs site on GitHub Pages
12. package-name due diligence outcome
13. SBOM/checksums for native artifacts
14. red-team release review

Deferred from v0.1 stable promise:

- R, Julia, TypeScript/Wasm, C#, Go stable packages
- `kairo-ecs-viz`
- parallel/PDES claims
- universal zero-copy claims
- stable scenario manifest schema
- stable Arrow schemas beyond event-log preview

## v0.2

- Arrow schemas refined
- TypeScript/Wasm preview
- R/Julia preview wrappers
- experiment runner alpha
- comparative benchmark harness public

## v0.3

- C# .NET 10 stable preview, .NET 11 preview lane
- Go preview wrapper
- model zoo expansion
- OpenSSF Scorecard and badge progress

## v0.4-beta

- all published packages pass conformance fixtures
- schema/ABI migration policy proven
- release automation mostly keyless/provenance-enabled

## 1.0

- stable core API, C ABI major version, Arrow schema compatibility promise
- all release-critical docs and examples
- public benchmark harness and results
- governance, security, and maintenance processes exercised
