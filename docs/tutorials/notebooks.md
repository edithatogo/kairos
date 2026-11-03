# Notebook Tutorials

KairoECS keeps runnable notebook assets in `notebooks/` for users who want a
guided, executable path through the scheduler and reproducibility controls.
The two local tutorials are documentation artifacts, not benchmark producers;
their examples are designed for a repository checkout without network access,
package installation, or native FFI wheels. The Colab accelerator notebooks
instead use provider-managed runtimes and can install JAX dependencies.

## Available notebooks

- [Python scheduler tutorial](../../notebooks/python_scheduler_tutorial.ipynb)
  covers deterministic event ordering, cancellation semantics, scheduler stats,
  and trace inspection using the pure Python facade.
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
It does not execute notebook cells or qualify a Jupyter kernel, provider runtime,
accelerator or package installation. For local use, select and verify a kernel
running the qualified repository-local Python 3.15.2 environment; the `python3`
kernelspec alone does not prove its version or prefix. Colab-managed kernel and
dependency compatibility remains a separate qualification.
