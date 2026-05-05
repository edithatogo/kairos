# ADR 0004: Project Name and Package Naming

## Status

Accepted for planning; public publishing blocked until registry/legal due diligence is complete.

## Context

The project needs a name that is distinctive, technically meaningful, and usable across Rust, Python, R, Julia, TypeScript, C#, Go, C ABI artifacts, documentation, and research citations.

The earlier bare `Kairos` direction is evocative but too crowded for a multi-registry public project. A package family needs clearer names that avoid forcing users into ambiguous `pip install kairos`, `npm install kairos`, or similar commands.

## Decision

Use **KairoECS** as the public project and ecosystem name.

Define **ECS** publicly as **Event-Component Simulation**. Internally, the engine uses Entity-Component-System architecture, so the acronym intentionally bridges simulation terminology and systems-programming terminology.

Use this package map:

| Surface | Name |
|---|---|
| Repository | `kairo-ecs` |
| Rust root crate | `kairo-ecs` |
| Rust internal crates | `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-ffi`, `kairo-ecs-arrow`, `kairo-ecs-viz` |
| Python distribution | `kairo-ecs` |
| Python import | `kairo_ecs` |
| R package | `kairoECS` |
| Julia package | `KairoECS.jl` |
| npm scope | `@kairo-ecs` |
| TypeScript package | `@kairo-ecs/core` |
| NuGet package | `Kairo.ECS` |
| Go module | `github.com/<org>/kairo-ecs` |
| C ABI library/header | `libkairo_ecs`, `kairo_ecs.h` |
| CLI | `kairoecs` |

## Consequences

- The name is clear enough for simulation users and technical enough for Rust/ECS contributors.
- The public package family is more distinctive than bare `kairo` or `kairos`.
- Documentation must explain that KairoECS is a hybrid DES/ABM engine, not just a game-style ECS crate.
- Public publishing remains blocked until registry, domain, trademark, and common-law checks are complete.
