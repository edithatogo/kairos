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
