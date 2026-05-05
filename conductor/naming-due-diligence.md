# KairoECS Naming Due Diligence

## Decision

The working public project name is **KairoECS**.

The public meaning of **ECS** is **Event-Component Simulation**. Internally, the engine uses Entity-Component-System architecture, so the acronym intentionally bridges the simulation and systems-programming audiences.

## Naming map

| Surface | Name |
|---|---|
| Project / ecosystem | `KairoECS` |
| Workspace repository | `kairos` |
| Rust root crate | `kairo-ecs` |
| Rust internal crates | `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-ffi`, `kairo-ecs-arrow`, `kairo-ecs-viz`, `kairo-ecs-experiment`, `kairo-ecs-conformance` |
| Python distribution | `kairo-ecs` |
| Python import | `kairo_ecs` |
| R package | `kairoECS` |
| Julia package | `KairoECS.jl` |
| npm scope | `@kairo-ecs` |
| TypeScript package | `@kairo-ecs/typescript` |
| NuGet package | `Kairo.ECS` |
| Go module | `github.com/edithatogo/kairos/bindings/go` |
| C library | `libkairo_ecs` |
| C header | `kairo_ecs.h` |
| C function prefix | `kairo_ecs_` |
| CLI | `kairoecs` |

## Why this track exists

Names are ecosystem commitments. A multi-language library must verify package, repository, domain, trademark, and common-law usage across every target surface before publishing. Do not assume availability from planning notes.

## Registry checklist

```text
crates.io: kairo-ecs, kairo-ecs-core, kairo-ecs-state, kairo-ecs-rng, kairo-ecs-ffi, kairo-ecs-arrow, kairo-ecs-viz
PyPI: kairo-ecs
npm: @kairo-ecs/typescript and @kairo-ecs organization/scope
NuGet: Kairo.ECS
R release channel: kairoECS
Julia General: KairoECS.jl
GitHub workspace repo: kairos
GitHub public release repo: confirm before first public publish
Go module path: github.com/edithatogo/kairos/bindings/go
Docs: kairo-ecs.dev / kairo-ecs.org / fallback domain
OpenCollective/project ecosystem check
Trademark/common-law usage check
```

## Dated offline evidence

Review date: 2026-05-06.

Reviewer: Worker B, Track 00 naming due-diligence evidence.

Scope: local/offline repository evidence only. Network access was restricted for this pass, so no live registry, domain, trademark, common-law web, GitHub availability, OpenCollective, PyPI, npm, crates.io, NuGet, Julia General, R-universe, CRAN, Go proxy, or domain-registrar availability result is claimed here.

### Local sources inspected

| Source | Evidence recorded |
|---|---|
| `conductor/package-matrix.md` | Preferred package family and current checked-in package surfaces. |
| `conductor/package-catalog.md` | Package-surface inventory and registry notes by ecosystem. |
| `conductor/tracks/00-project-foundation-governance-naming/spec.md` | Acceptance criteria require actual registry search results for all target registries. |
| `docs/adr/0004-project-name-kairoecs.md` | Planning decision accepts `KairoECS` while blocking public publishing until registry/legal due diligence is complete. |
| `Cargo.toml` and `crates/*/Cargo.toml` | Current Rust workspace package names and repository URL. |
| `bindings/python/pyproject.toml` | Python distribution and import package declaration. |
| `bindings/r/DESCRIPTION` | R package declaration. |
| `bindings/julia/Project.toml` | Julia package declaration. |
| `bindings/typescript/package.json` | npm package declaration. |
| `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` | NuGet package declaration. |
| `bindings/go/go.mod` | Go module declaration. |
| `CITATION.cff`, `.zenodo.json`, `README.md` | Project identity and repository-code URL. |

### Current checked-in package-name evidence

| Surface | Checked-in evidence | Local verdict |
|---|---|---|
| Project / ecosystem | `README.md`, `CITATION.cff`, `.zenodo.json`, and ADR 0004 use `KairoECS`. | Locally consistent. Live trademark/common-law search still required. |
| Repository URL in metadata | `CITATION.cff`, `.zenodo.json`, `Cargo.toml`, and C# package metadata point to `https://github.com/edithatogo/kairos`. | Locally consistent with the checked-in workspace. Live public-release repo decision still required. |
| Rust workspace packages | `Cargo.toml` members include `crates/kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-des`, `kairo-ecs-abm`, `kairo-ecs-ffi`, `kairo-ecs-uniffi`, `kairo-ecs-diplomat`, `kairo-ecs-arrow`, `kairo-ecs-bench`, `kairo-ecs-cli`, `kairo-ecs-gpu`, `kairo-ecs-webgpu`, `kairo-ecs-pdes`, `kairo-ecs-mpi`, `kairo-ecs-grpc`, `kairo-ecs-streaming`, `kairo-ecs-ml`, `kairo-ecs-fmi`, `kairo-ecs-debug`, `kairo-ecs-viz`, and `kairo-ecs-wasm`. Individual crate manifests declare matching `name` values. | Local manifests are explicit. crates.io availability for every planned publishable crate remains unchecked. |
| Rust root crate | `conductor/package-matrix.md` reserves `kairo-ecs` as the preferred root meta crate; no root `kairo-ecs` crate manifest exists in the current workspace. | Planned but not checked in as a root crate. crates.io availability still required. |
| Python distribution | `bindings/python/pyproject.toml` declares `[project] name = "kairo-ecs"`. | Local manifest exists. PyPI/TestPyPI availability still required. |
| Python import | `bindings/python/pyproject.toml` declares `packages = ["kairo_ecs"]`. | Local import package is explicit. |
| R package | `bindings/r/DESCRIPTION` declares `Package: kairoECS`. | Local manifest exists. CRAN/R-universe availability and policy review still required. |
| Julia package | `bindings/julia/Project.toml` declares `name = "KairoECS"`. | Local manifest exists. Julia General/dev registry availability still required. |
| npm package | `bindings/typescript/package.json` declares `"name": "@kairo-ecs/typescript"`. | Local manifest exists. npm scope and package availability still required. |
| NuGet package | `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` declares `<PackageId>Kairo.ECS</PackageId>`. | Local manifest exists. NuGet availability still required. |
| Go module | `bindings/go/go.mod` declares `module github.com/edithatogo/kairos/bindings/go`. | Local module path exists. Public repo/module path decision and Go proxy visibility still required. |
| C ABI | C# native binding tests and docs refer to `kairo_ecs.dll`, `libkairo_ecs.dylib`, and `libkairo_ecs.so`; naming map reserves `kairo_ecs_` prefixes. | Local ABI naming is present. Header/export availability must remain tied to Track 02 release evidence. |
| CLI | `conductor/trustworthy-simulation.md` uses `kairoecs` command examples; workspace includes `crates/kairo-ecs-cli`. | Local CLI naming is present. Package/installer name availability still required. |
| Docs domains | Naming checklist lists `kairo-ecs.dev` and `kairo-ecs.org`. | No local evidence of ownership or availability. Live registrar/domain check required. |
| OpenCollective/project ecosystem | Naming checklist requires review. | No local evidence found in this pass. Live ecosystem search required. |
| Trademark/common-law | ADR 0004 and this file explicitly block publishing until checked. | No local legal/trademark evidence found in this pass. Live trademark/common-law search required. |

### Registry search results

No live registry search results were recorded in this offline pass. The acceptance criterion requiring actual registry search results remains unmet.

| Registry / channel | Exact names requiring live check | 2026-05-06 result |
|---|---|---|
| crates.io | `kairo-ecs`, all checked-in `kairo-ecs-*` crate names planned for publication | Blocked: live search required. |
| PyPI / TestPyPI | `kairo-ecs` | Blocked: live search required. |
| npm | `@kairo-ecs` scope, `@kairo-ecs/typescript` | Blocked: live search required. |
| NuGet | `Kairo.ECS` | Blocked: live search required. |
| R release channel | `kairoECS` on R-universe and, if later targeted, CRAN | Blocked: live search required. |
| Julia registry | `KairoECS` / `KairoECS.jl` in Julia General and any dev registry target | Blocked: live search required. |
| GitHub | `edithatogo/kairos`, public release repo target, optional `kairo-ecs` repository/org targets | Blocked: live search required. |
| Go module path | `github.com/edithatogo/kairos/bindings/go` and final public module path | Blocked: live search required. |
| Docs domains | `kairo-ecs.dev`, `kairo-ecs.org`, fallback domain | Blocked: live registrar/DNS check required. |
| OpenCollective / ecosystem | `KairoECS`, `kairo-ecs`, `kairos` variants | Blocked: live search required. |
| Trademark/common-law | `KairoECS`, `Kairo ECS`, `kairo-ecs`, close variants in target jurisdictions and software/scientific simulation contexts | Blocked: live trademark/common-law search and legal review required. |

### Follow-up checklist to unblock Track 00 naming acceptance

- Record the live search date, reviewer, registry URL or tool used, exact query string, exact package/repo/domain name checked, and observed result for each registry/channel above.
- Capture fallback names for each ecosystem before any production publish, including Rust root crate, Python distribution, npm scope/package, NuGet package, R package, Julia package, Go module path, docs domain, and public GitHub release repo.
- Confirm whether the public release repository remains `github.com/edithatogo/kairos` or moves to a `kairo-ecs` repository/org before Go module tagging or package metadata publication.
- Record any trademark/legal advice received, including target jurisdictions and whether the review covers `KairoECS`, `Kairo ECS`, `kairo-ecs`, and confusingly similar simulation/software marks.
- Reconcile ADR 0004 planning names that differ from current manifests, notably repository `kairo-ecs` vs current `edithatogo/kairos`, TypeScript package `@kairo-ecs/core` vs current `@kairo-ecs/typescript`, and Go module `github.com/<org>/kairo-ecs` vs current `github.com/edithatogo/kairos/bindings/go`.

### Required Done evidence structure

Track 00 can only satisfy the naming acceptance criterion when the live evidence is recorded in this file using one row per checked name and one decision row per target surface. Screenshots or command transcripts may be linked, but they do not replace the structured rows.

Live search rows must include:

| Field | Required content |
|---|---|
| Review date | Date the live search was run. |
| Reviewer | Person or worker who ran the search. |
| Surface | Registry, repository host, domain registrar/DNS source, trademark register, common-law search source, OpenCollective, or Go proxy. |
| Query/source | URL, registry tool, CLI command, legal database, or search source used. |
| Exact name checked | Exact package, scope, repository, module path, domain, project name, or mark variant. |
| Observed result | Available, unavailable, conflicting, reserved, inconclusive, or policy-blocked, with source-backed detail. |
| Evidence pointer | Checked-in transcript, screenshot path, issue, legal memo, or reviewer note. |
| Decision impact | Selected, fallback required, legal review required, blocked, or no release impact. |

Surface decision rows must include:

| Field | Required content |
|---|---|
| Surface | Target ecosystem or public identity surface. |
| Selected public name | Final name approved for release metadata. |
| Fallback name | Approved fallback if the selected name cannot be used. |
| Public repo/module decision | Final GitHub repository and Go module path where applicable. |
| Legal/trademark advice | Advice source, date, jurisdiction/scope, and outcome, or explicit maintainer waiver with owner and expiry. |
| Release stage allowed | Planning only, alpha, beta, RC, or 1.0. |
| Approver | Maintainer or release owner accepting the evidence. |

## Naming policy

- Use `KairoECS` in prose and public branding.
- Use `kairo-ecs` for package and repository names where hyphenation is idiomatic.
- Use `kairo_ecs` for Python imports and C ABI names.
- Use `Kairo.ECS` for NuGet packages and C# namespaces.
- Use `KairoECS.jl` for Julia.
- Avoid bare `kairo` for registries unless a future legal/package review explicitly approves it.
- Keep the checked-in workspace repository name distinct from the eventual public package names; the repo name does not drive registry names.

## Release blocker

Public publishing is blocked until a maintainer records:

```text
- registry search date
- reviewer
- exact package names checked
- search results
- chosen package names
- fallback names
- current checked-in package surfaces
- any legal/trademark advice received
```

## Rationale

`KairoECS` is distinctive, technical, and clear enough for the engine architecture. It lets Rust/systems contributors understand the ECS foundation while giving simulation users an expandable definition: Event-Component Simulation.
