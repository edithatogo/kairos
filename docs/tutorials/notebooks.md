# Notebook Tutorials

KairoECS keeps runnable notebook assets in `notebooks/` for users who want a
guided, executable path through the scheduler and reproducibility controls.
The local tutorials are documentation artefacts, not benchmark producers.
The Python scheduler notebook requires the binding installed in its selected
kernel and the actual Rust shared library built from the matching source.
Set `KAIRO_ECS_FFI_LIBRARY` to that library's absolute path; missing or incompatible
native capabilities fail explicitly. Follow [installation](../install.md).
The regression notebook reads committed fixtures. Colab accelerator notebooks
use provider-managed runtimes and may install JAX dependencies.

## Available notebooks

- [Python scheduler tutorial](../../notebooks/python_scheduler_tutorial.ipynb)
  uses `NativeEngine` for Rust scheduling, bounded execution, cancellation and
  statistics. It does not reconstruct an event trace or run a host-language core.
- [Reproducible benchmark scenario](../../notebooks/reproducible_benchmark_scenario.ipynb)
  reads committed regression fixtures and the performance threshold table to
  explain the offline regression guard.
- [Colab GPU smoke route](../../notebooks/colab_gpu_smoke.ipynb) documents
  the hosted GPU smoke route without requiring local GPU hardware.
- [Colab TPU smoke route](../../notebooks/colab_tpu_smoke.ipynb) documents
  the hosted TPU smoke route at the notebook level.
- [Colab TPU dedicated smoke route](../../notebooks/colab_tpu_dedicated_smoke.ipynb)
  records the dedicated TPU smoke path used by the learning-coverage matrix.

## Figures

- [Scheduler timeline](../../notebooks/figures/scheduler-timeline.svg) is a
  checked-in SVG used by the Python scheduler notebook.

## Coverage matrix

- [Learning Coverage Matrix](coverage-matrix.md) explains which languages use
  notebook walkthroughs and which ones intentionally do not.

## Local validation

Run this from the repository root:

```powershell
.venv\Scripts\python.exe scripts\python_runtime.py
.venv\Scripts\python.exe notebooks\validate_notebooks.py
```

The validator parses the notebooks under `notebooks/`, checks their metadata
and referenced local figures, and syntax-checks non-Colab Python cells. It
rejects network/package-install patterns except permitted Colab smoke magics.
It does not execute notebook cells or qualify a Jupyter kernel, Rust native
library, provider runtime, accelerator or package installation. For local use,
select and verify a kernel running the qualified repository-local Python 3.15.2
environment; the `python3`
kernelspec alone does not prove its version or prefix. Colab-managed kernel and
dependency compatibility remains a separate qualification.

Native notebook acceptance must run the executable cells with the actual built
library and record source revision, library checksum, features, Python/kernel
identity and command/output evidence. A passing syntax check is a separate result.
