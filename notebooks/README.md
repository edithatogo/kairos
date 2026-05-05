# KairoECS Notebooks

These notebooks are checked-in tutorial assets for the documentation site.
They are intentionally lightweight:

- no network access
- no native FFI dependency
- no package installation step
- deterministic inputs and assertions

## Notebooks

- [Python scheduler tutorial](python_scheduler_tutorial.ipynb) introduces the
  pure Python scheduler facade, deterministic event ordering, cancellation, and
  basic trace inspection.
- [Reproducible benchmark scenario](reproducible_benchmark_scenario.ipynb)
  reads the committed regression fixtures and threshold table to explain the
  offline performance guard.

## Validation

Run from the repository root:

```powershell
python notebooks\validate_notebooks.py
```

The validator parses every checked-in `.ipynb` file under this directory,
executes Python code cells in order, and checks that referenced local figures
exist.
