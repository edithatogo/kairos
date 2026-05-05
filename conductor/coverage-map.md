# KairoECS Setup Coverage Map

This file maps the “everything needed for a professional release” concerns to concrete Conductor artifacts.

| Concern | Primary files/tracks | Status in setup |
|---|---|---|
| Architecture | `conductor/tech-stack.md`, `planning/diagrams/architecture.mmd` | Covered |
| Parallel subagents | `conductor/subagents.md`, `conductor/parallel-execution.md`, per-track `agent-contract.md` | Covered |
| Core implementation | Track 01 | Covered |
| FFI stability | Track 02, `conductor/contracts/ffi-contract.md` | Covered |
| DES/ABM APIs | Track 03 | Covered |
| Arrow telemetry | Track 04, `conductor/contracts/arrow-schema-contract.md` | Covered |
| Visualization | Track 05 | Covered as optional module |
| Python 3.10-3.14 | Track 06, CI matrix, package matrix | Covered |
| R binding | Track 07, package matrix | Covered |
| Julia binding | Track 08, package matrix | Covered |
| TypeScript/Wasm | Track 09, package matrix | Covered |
| C# .NET 10 | Track 10, CI matrix, C# template | Covered |
| Go binding | Track 11, package matrix | Covered |
| Testing | Track 12, `conductor/testing-strategy.md`, `conductor/quality-gates.md` | Covered |
| Benchmarks | Track 12, `kairo-ecs-bench`, Criterion/benchmark plan | Covered |
| CI/CD | Track 13, `.github/workflows/*` | Covered |
| Code quality | Track 13, `conductor/quality-gates.md` | Covered |
| Supply chain/security | Track 13, `SECURITY.md`, `.github/workflows/codeql.yml`, secret scan, audit, deny, SBOM release gate | Covered |
| Documentation | Track 14, `website` template, docs release checklist | Covered |
| GitHub Pages | Track 14, `.github/workflows/docs.yml` | Covered |
| Publishing | Track 15, `conductor/package-matrix.md`, release workflow | Covered |
| Delivery | Track 15, `conductor/delivery-readiness-checklist.md` | Covered |
| Governance | Track 00, Track 16, `conductor/maintenance-governance.md` | Covered |
| Maintenance | Track 16, `conductor/maintenance-governance.md`, dependency automation | Covered |
| Naming/legal | Track 00, `conductor/naming-due-diligence.md` | Covered |
| Version compatibility | `conductor/contracts/versioning-compatibility.md`, Track 16 | Covered |
| Release checklist | `docs/release/release-checklist.md` | Covered |

---

# Expanded coverage commitments

## Python

Required CI coverage:

```text
CPython 3.10
CPython 3.11
CPython 3.12
CPython 3.13
CPython 3.14
CPython 3.14 free-threaded smoke lane where toolchain/runner support exists
```

The Python binding must not rely on the GIL for Rust-side memory safety. The GIL may be used only for Python callback execution or Python object ownership.

## C#

Required coverage:

```text
net10.0 stable lane
net10.0 stable lane
```

Stable NuGet promises are tied to .NET 10 for this repo baseline.

## Community/trust artifacts

Coverage now also includes:

```text
model zoo examples
benchmark reproducibility metadata
seed/scenario manifests
artifact SBOM/provenance
citation metadata
OpenSSF Scorecard/Best Practices readiness
API compatibility review records
```
