# KairoECS Product Context

## Product identity

KairoECS is a next-generation, multi-method simulation engine for building trustworthy, reproducible, high-performance simulations across scientific, operational, policy, and industrial domains.

The name KairoECS combines **Kairo** (evoking the right or opportune moment) with **ECS**, publicly defined as **Event-Component Simulation** and internally implemented with Entity-Component-System architecture. The product identity emphasizes precise event timing, deterministic replay, and a kernel that treats simulation time as a first-class abstraction.

## Core promise

KairoECS provides a Rust-owned simulation core that treats Discrete Event Simulation (DES) and Agent-Based Modeling (ABM) as equal paradigms, surfaced through idiomatic bindings for Python, R, Julia, TypeScript, C#, and Go.

## Primary users

- Operations researchers building queues, resources, logistics, and capacity models.
- Agent-based modellers studying social, health, biological, market, and mobility systems.
- Data scientists who need fast simulation outputs in Arrow/Parquet for analysis.
- Rust developers who need a safe, deterministic simulation core.
- Python/R/Julia analysts who want high performance without hand-writing Rust.
- Enterprise and public-sector users who need governance, reproducibility, and release trust.
- Educators and researchers who need examples, notebooks, citations, and archived releases.

## Product pillars

1. **Precision:** deterministic virtual time, fixed-tick ordering, stable event traces.
2. **Performance:** Rust core, ECS-style storage, batch APIs, Arrow telemetry.
3. **Polyglot access:** stable C ABI, generated binding adapters, idiomatic wrappers.
4. **Trust:** replay, seed manifests, V&V/UQ tools, transparent benchmarks.
5. **Community usability:** examples, model zoo, docs site, playground, governance.

## Non-goals for early releases

- Do not promise fully parallel deterministic PDES before the sequential kernel is trusted.
- Do not make visualization a dependency of the headless core.
- Do not publish six stable language APIs before the C ABI and conformance suite are stable.
- Do not claim universal zero-copy; document exact copy and lifetime semantics.
- Do not claim nanosecond wall-clock performance; support nanosecond-resolution simulation ticks when configured.
