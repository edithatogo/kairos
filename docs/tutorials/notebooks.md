# Notebook Tutorials

KairoECS keeps runnable notebook assets in `notebooks/` for users who want a
guided, executable path through the scheduler and reproducibility controls.
The notebooks are documentation artifacts, not benchmark producers: they are
designed to run from a repository checkout without network access, package
installation, or native FFI wheels.

## Available notebooks

- [Python scheduler tutorial](../../notebooks/python_scheduler_tutorial.ipynb)
  covers deterministic event ordering, cancellation semantics, scheduler stats,
  and trace inspection using the pure Python facade.
- [Reproducible benchmark scenario](../../notebooks/reproducible_benchmark_scenario.ipynb)
  reads committed regression fixtures and the performance threshold table to
  explain the offline regression guard.

## Figures

- [Scheduler timeline](../../notebooks/figures/scheduler-timeline.svg) is a
  checked-in SVG used by the Python scheduler notebook.

## Local validation

Run this from the repository root:

```powershell
python notebooks\validate_notebooks.py
```

The validator parses all checked-in notebooks, confirms that referenced local
figures exist, rejects network/package-install shell patterns in code cells, and
executes each Python code cell in order.
