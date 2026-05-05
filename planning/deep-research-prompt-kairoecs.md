# Updated Deep Research Prompt: KairoECS + Conductor

Task: Research and design a comprehensive implementation roadmap for a new simulation library called **KairoECS** using the Conductor CLI extension framework.

Project Identity: KairoECS is a next-generation, multi-method simulation engine. It treats Discrete Event Simulation (DES) and Agent-Based Modeling (ABM) as equal paradigms, using an ECS architecture for extreme performance and memory safety.

Core Requirement: The library is written in Rust and must be surfaced via high-performance, idiomatic bindings to Python, R, Julia, TypeScript, C#, and Go.

Version targets:

- Python 3.10 through 3.14.
- C#/.NET 10 through .NET 11.

Architecture:

- `kairo-ecs-core`: scheduler and event logic.
- `kairo-ecs-state`: entity/component storage.
- `kairo-ecs-ffi`: universal bridge definitions with stable C ABI, UniFFI, and Diplomat where useful.
- `kairo-ecs-arrow`: Apache Arrow telemetry/export.
- `kairo-ecs-viz`: optional WGPU/Bevy visualization.

Research requirements:

1. Produce `conductor/tech-stack.md` for a polyglot Rust project.
2. Produce `conductor/workflow.md` for cross-language FFI stability.
3. Break work into Conductor tracks that can run in parallel using subagents.
4. Include tracks for testing, documentation, governance, CI/CD, publishing, delivery, automation, and maintenance.
5. Include Mermaid diagrams for architecture, track dependencies, subagent swimlanes, CI/CD, and release pipeline.
6. Provide detailed `spec.md` and `plan.md` for Track 1: `kairo-ecs-core` and `kairo-ecs-state`.
7. Include naming/legal due diligence because “KairoECS” may already have existing ecosystem use.

Reference material: KairoECS seeks to fill the gap left by SimPy's performance limitations and AnyLogic's closed-source hybrid capability.
