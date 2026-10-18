---
title: 'KairoECS: A Rust-first multi-method simulation engine for DES and ABM'
tags:
  - simulation
  - discrete-event-simulation
  - agent-based-modeling
  - rust
  - apache-arrow
  - high-performance-computing
authors:
  - name: Dylan A Mordaunt
    affiliation: 1
  - name: KairoECS contributors
    affiliation: 1
affiliations:
  - name: KairoECS Project
    index: 1
date: 2026-05-05
bibliography: paper.bib
---

# Summary

`KairoECS` is a high-performance, deterministic, multi-method simulation engine implemented in Rust. It unifies Discrete-Event Simulation (DES) [@zeigler2000theory] and Agent-Based Modeling (ABM) [@north2013complex; @masad2015mesa] into a single, cohesive framework by combining a deterministic event-first scheduling kernel with an Entity-Component-System (ECS) data architecture. 

In complex socio-technical systems—such as healthcare delivery, logistics networks, epidemiological interventions, and ecological resource management—individual decision-makers interact with shared operational infrastructure, queues, and resource constraints. Traditional modeling approaches force researchers to choose between discrete-event queuing engines or autonomous agent models. `KairoECS` bridges this methodological divide, enabling fine-grained individual agent behaviors to directly interact with priority-driven discrete-event queues without sacrificing performance or execution determinism.

The planned target metadata version for the engine is `0.4.0-alpha.1`, with public source code, issue tracking, and contribution workflows hosted at `https://github.com/edithatogo/kairos`. `KairoECS` is registered in the SciCrunch Resource Identification Portal under persistent identifier `RRID:SCR_029085`.

# Statement of Need

Scientific computational modeling requires reproducible, auditable, and performant tools capable of scaling from local analytical workflows to High-Performance Computing (HPC) clusters. However, existing computational simulation ecosystems face significant structural limitations:

1. **Paradigm Fragmentation:** Popular modeling environments typically support either pure discrete-event queuing or agent-based interaction, forcing unnatural architectural compromises when modeling hybrid systems (e.g., patient decision-making in hospital emergency departments).
2. **Interpreter Overhead:** Python- and Java-based platforms often encounter severe performance bottlenecks when scaling to millions of concurrent agents or high-frequency event loops, restricting the feasibility of extensive sensitivity analysis and parameter calibration.
3. **Reproducibility Gaps:** Lack of strict seed-level determinism, cross-platform float inconsistencies, and opaque internal memory states make bit-for-bit replay challenging across heterogeneous operating environments.
4. **Data Interoperability:** Exporting telemetry across language boundaries typically incurs massive serialization penalties (e.g., JSON or CSV parsing), restricting real-time visualization and machine learning surrogate training.

`KairoECS` directly addresses these challenges by delivering a native Rust core that enforces `#![forbid(unsafe_code)]`, guaranteed reproducible replay schedules across platforms, columnar telemetry adapters aligned with Apache Arrow schemas [@arrow2021], and idiomatic polyglot bindings for Python, TypeScript/Wasm, R, Julia, C#, and Go.

# State of the Field

The scientific simulation ecosystem features established frameworks tailored to specific paradigms:

- **SimPy** [@simpy2020] provides an intuitive process-based discrete-event environment in Python. While widely adopted in operational research, its reliance on Python generators and synchronous event loops limits throughput in large-scale multi-agent experiments.
- **NetLogo** [@wilensky1999netlogo] has served as a benchmark for educational and exploratory agent-based modeling, but its custom language and runtime scale poorly to large-scale operational pipelines and lack native HPC integration.
- **Repast Simphony and Repast HPC** [@north2013complex] offer robust agent simulation in Java and C++/MPI. However, developing cross-language workflows remains cumbersome, and integrating modern columnar data formats requires external tooling.
- **Mesa** [@masad2015mesa] provides an extensible Python framework for ABM with visualization capabilities, but performance limits emerge when simulating complex spatial dynamics or high-density populations without Cython or Numba extensions.

`KairoECS` complements these tools by providing a modular, typed engine that combines the speed and memory safety of Rust with Arrow-aligned telemetry interoperability, enabling researchers in Python, R, and Julia to orchestrate simulations without sacrificing core execution speed.

# Software Design and Architecture

`KairoECS` is architected as a workspace of modular crates:

- **Deterministic Core Kernel (`kairo-ecs-core`):** Implements an event-first scheduling engine using integer-quantized virtual simulation time (`SimTime`). Event execution order is strictly deterministic, using hierarchical tie-breaking by timestamp, event priority, and unique event sequence identifiers.
- **Entity-Component-System Storage (`kairo-ecs-state`):** Organizes world state into cache-friendly columnar component arrays rather than deep object hierarchies. Systems operate over contiguous memory slices, maximizing CPU cache utilization during agent state evaluations.
- **Pseudo-Random Number Generation (`kairo-ecs-rng`):** Implements stateful SplitMix64 deterministic PRNG streams (`DeterministicStream`) with independent entity seed derivation, ensuring reproducible state trajectories across operating systems and compilers.
- **Columnar Telemetry Alignment (`kairo-ecs-arrow`):** Emits field-level schema-aligned telemetry records and event logs formatted for direct export and conversion into Apache Arrow record batches, supporting downstream scientific data pipelines (Polars, DuckDB, Arrow).
- **Parallel and Distributed Engines (`kairo-ecs-pdes`, `kairo-ecs-mpi`):** Provides Conservative (Chandy-Misra-Bryant) and Optimistic (Time Warp) parallel discrete-event simulation algorithms [@fujimoto1990parallel] with rollback mechanisms and distributed state synchronization.
- **Polyglot Foreign Function Interface (`kairo-ecs-ffi`):** Exposes C-ABI, UniFFI, Diplomat, and WebAssembly targets, powering native packages for Python (`pyarrow`), TypeScript/JavaScript (WebAssembly/WebGPU), R, Julia, C#, and Go.

# Research Impact and Application Scenarios

`KairoECS` is designed to support research across diverse computational domains:

- **Healthcare Operations and Capacity Management:** Simulating emergency department crowding, elective surgical scheduling, and inpatient bed allocation, where patient behavioral choices (abandonment, treatment compliance) intersect with physical resource constraints (operating theaters, nursing shifts).
- **Epidemiological Interventions:** Modeling infectious disease transmission across spatial contact networks while simultaneously tracking hospital bed availability, diagnostic queue wait-times, and vaccine distribution logistics.
- **Algorithmic Game Theory and Economic Systems:** Evaluating strategic agent interactions, mechanism design, and regulatory compliance under inspection constraints and incomplete information.
- **Digital Twins and Co-Simulation:** Interfacing with physical equipment models through the Functional Mock-up Interface (FMI 2.0/3.0) and Asset Administration Shell (AAS) connectors.

# AI Usage Disclosure

In compliance with the Journal of Open Source Software author guidelines on artificial intelligence tools: Artificial intelligence coding assistants (including Anthropic Claude and Google Antigravity) were utilized during the project for test authoring, syntactic refactoring, and documentation drafting. All system architectures, theoretical formulations, algorithms, mathematical invariant proofs, and code implementations were directed, reviewed, verified, and validated by human maintainers.

# Acknowledgements

The authors acknowledge the open-source communities behind the Rust language, the Apache Arrow project, and the discrete-event and agent-based simulation literature whose foundational work made this engine possible.

# References
