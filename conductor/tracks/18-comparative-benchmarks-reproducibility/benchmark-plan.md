# Comparative Benchmark Plan

## Benchmark principles

- Benchmarks must be reproducible, version-pinned, and honest.
- Benchmark comparisons must avoid marketing claims until scripts are public.
- Track both performance and ergonomics: speed alone is not enough.

## Benchmark families

| Family | Measures | Initial competitors |
|---|---|---|
| Scheduler throughput | events/sec, cancellation/sec | SimPy, ConcurrentSim.jl, SimSharp, Rust DES crates |
| Entity scale | memory/entity, update throughput | Mesa, Agents.jl, krABMaga-style references |
| Resource queues | request/release throughput | SimPy, simmer, SimSharp |
| Binding overhead | per-call and batch-call overhead | Python/R/Julia/TS/C#/Go wrappers |
| Telemetry export | Arrow RecordBatch and IPC throughput | pyarrow/Arrow.jl/arrow R |
| Reproducibility | deterministic trace equality | all KairoECS bindings |

## Benchmark artifact layout

```text
benchmarks/
├── README.md
├── scenarios/
│   ├── scheduler_10m_events.toml
│   ├── mm1_queue.toml
│   ├── factory_bottleneck.toml
│   └── flocking_1m_agents.toml
├── harness/
│   ├── rust/
│   ├── python/
│   ├── r/
│   ├── julia/
│   ├── typescript/
│   ├── csharp/
│   └── go/
└── results/
    └── README.md
```

## Required output

Every benchmark run emits:

- machine profile
- package versions
- commit SHA
- scenario manifest
- seed manifest
- raw results in Arrow/JSON
- rendered summary table
