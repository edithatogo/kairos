# Benchmarks

Maturity: preview metadata gate, with separate bounded native measurements.
Broad comparative performance and HPC scaling certification remain incomplete.

## Read the evidence by type

| Evidence | What it establishes |
| --- | --- |
| [Measured conservative-PDES results](../pdes/benchmark-results.md) | Actual single-host timings and final-state parity for the recorded ring workloads. The parallel runtime was slower than its sequential baseline. |
| [GPU execution boundary](../gpu-compute/benchmark-results.md) | Bounded device/kernel qualification; no general end-to-end speedup claim. |
| [Scaling-report correction](scaling-certification-report.md) | The published local scaling figures were generated placeholders, not measurements. |
| [HPC summary](hpc-scaling-summary.json) | Schema/aggregation fixtures, explicitly blocked pending live scaling evidence. |
| Benchmark smoke and reproducibility checks | Metadata, source, fixture and raw-results-policy alignment. They do not time a simulator. |

## Policy and evaluation plan

- [Benchmark policy](benchmark-policy.md) and [reproduction guide](reproduce-comparison.md).
- [Raw-results policy](../../benches/raw-results-policy.json).
- [Measured-performance work](../../conductor/tracks/111-measured-performance/spec.md).
- [Datasets and independent analytical oracles](../../conductor/validation-datasets.md).

Compare equivalent models and record exact inputs, software, hardware and raw
outputs. Separate elapsed runtime, memory, numerical/model correctness and
repeatability. Separate running independent experiments in parallel from
parallelizing a single simulation. A fixture, imported result or formula cannot
satisfy a measurement gate.

## Local metadata validation

Use the qualified interpreter selected under [AGENTS.md](../../AGENTS.md):

```bash
.venv/bin/python benches/benchmark_smoke.py
.venv/bin/python benches/benchmark_reproducibility.py
```

Expected output: both commands emit JSON with `"status": "ok"`. These checks
validate benchmark metadata and reproducibility-evidence alignment only.
Hardware and runtime qualification require their own commands and receipts.
