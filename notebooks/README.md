# KairoECS Notebooks

These notebooks are checked-in tutorial assets for the documentation site.
Their runtime requirements depend on the notebook:

- The Python scheduler tutorial requires the Python binding installed in the
  selected kernel and the actual matching Rust shared library. Set
  `KAIRO_ECS_FFI_LIBRARY` to its absolute path. Missing or incompatible native
  capabilities fail explicitly; no host-language core is substituted.
- The reproducibility tutorial reads committed benchmark fixtures and does not
  generate new performance measurements.
- Colab accelerator notebooks use provider-managed runtimes and can install
  dependencies. Their hardware and package execution need separate qualification.

Use the [installation guide](../docs/install.md) to prepare the native notebook
and select a qualified Python environment.

## Notebooks

- [Python scheduler tutorial](python_scheduler_tutorial.ipynb) calls the Rust
  scheduler through `NativeEngine` for bounded execution, cancellation and
  statistics. It does not fabricate a trace or substitute a Python scheduler.
- [Reproducible benchmark scenario](reproducible_benchmark_scenario.ipynb)
  reads the committed regression fixtures and threshold table to explain the
  offline performance guard.
- [Colab GPU smoke route](colab_gpu_smoke.ipynb) documents the hosted GPU
  smoke route without requiring local GPU hardware.
- [Colab TPU smoke route](colab_tpu_smoke.ipynb) documents the hosted TPU
  smoke route at the notebook level.
- [Colab TPU dedicated smoke route](colab_tpu_dedicated_smoke.ipynb) records
  the dedicated TPU smoke path used by the learning-coverage matrix.

## Validation

Run from the repository root:

```powershell
python notebooks\validate_notebooks.py
```

The validator parses every checked-in `.ipynb` file under this directory,
checks Python syntax without executing code cells, and verifies that referenced
local figures exist. Colab smoke notebooks allow notebook magics and skip Python
syntax parsing; their code cells are still checked for forbidden text patterns.

Successful structure/syntax validation does not load the Rust library or execute
simulation cells. Native acceptance must run the notebook with the installed
engine and retain its source revision, artefact checksum, feature set, kernel
version and actual results. Committed empty outputs do not claim a completed run.
