# Reproducing Benchmark Comparisons

This page defines the minimum replayable evidence for a KairoECS benchmark
comparison before native benchmark binaries are available.

Start from [`docs/benchmarks/README.md`](README.md) for the benchmark
overview and then use this page for the comparison replay path.

## Source Of Truth

The committed benchmark inventory is the source of truth:

| Evidence | Path | Required IDs |
|---|---|---|
| Ready fixture manifest | `conformance/fixtures/manifest.json` | `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1` |
| Benchmark smoke metadata | `benches/benchmark-smoke.json` | `schedule_1m_events`, `pop_1m_events`, `schedule_cancel_1m_mixed`, `create_1m_entities`, `component_insert_1m`, `hybrid_des_abm_smoke_100k` |
| Benchmark measurement contract | `benches/benchmark-plan.md` | canonical scenario names and output expectations |
| Raw-results policy | `benches/raw-results-policy.json` | required artifacts and result fields before public performance claims |
| Smoke workflow | `.github/workflows/benchmark-smoke.yml` | repo shape check and `cargo bench --workspace --no-run` |

Do not publish a comparison claim from a renamed local scenario or from a
fixture that is not marked `ready` in the manifest.

## Concrete Replay Path

1. Validate benchmark smoke metadata:

   ```powershell
   python benches/benchmark_smoke.py
   ```

2. Validate Track 18 reproducibility evidence:

   ```powershell
   python benches/benchmark_reproducibility.py
   ```

3. For native benchmark readiness, run the same command as the smoke workflow:

   ```powershell
   cargo bench --workspace --no-run
   ```

The Python checks are metadata gates. They prove that published scenario names,
ready fixture IDs, source files, and smoke-scale records are aligned. They do
not prove runtime performance, statistical stability, or ecosystem fairness.

They also check the raw-results policy gate. That gate does not require raw
timings during metadata validation; it records the command capture, environment
metadata, raw output, seed, fixture, scenario, toolchain, feature-flag, and
baseline-version evidence that must exist before a performance comparison can
be published.

## Seed And Fixture Control

The initial reproducibility claim is limited to deterministic fixture replay:

- `scheduler_ordering_v1` proves event ordering semantics from
  `deterministic_ordering.json`.
- `scheduler_cancellation_v1` proves cancellation semantics from
  `cancellation.json`.
- `rng_reproducibility_v1` proves seed replay from `rng_replay.json`.

Every later comparison result must record the fixture ID, benchmark scenario
ID, command, commit SHA, host details, compiler/toolchain versions, feature
flags, raw output, and any external baseline version.

## Comparison Criteria

A result is comparable only when:

- KairoECS and the baseline run the same model configuration and input scale.
- The KairoECS result uses one of the canonical benchmark scenario IDs.
- The deterministic fixture source is committed and named in the result.
- The seed or replay source is recorded for every stochastic path.
- The result includes raw output and environment metadata, not only a summary.
- The result satisfies `benches/raw-results-policy.json`.

Host variance is expected. Treat a single local run as a smoke result unless it
is repeated on a pinned runner with archived environment metadata.
