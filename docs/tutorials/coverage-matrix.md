# Learning Coverage Matrix

This matrix is the repo's honest learning-coverage inventory. It answers what
sort of beginner-friendly artifact exists for each supported language and
example family, and it marks notebook use explicitly instead of pretending a
notebook is always the right medium.

## Supported language surfaces

| Language | Tutorial | Example / quickstart | Notebook | Status |
|---|---|---|---|---|
| Rust | [Rust getting started](rust-getting-started.md) | [Factory bottleneck example](../../examples/des/factory_bottleneck/README.md) | not used | source-backed |
| Python | [Python getting started](python-getting-started.md) | [Documentation examples](../../examples/docs/README.md) | [Python scheduler tutorial](../../notebooks/python_scheduler_tutorial.ipynb) | source-backed + notebook |
| R | [R getting started](r-getting-started.md) | [R binding README](../../bindings/r/README.md) | not used | source-backed |
| Julia | [Julia getting started](julia-getting-started.md) | [Julia binding README](../../bindings/julia/README.md) | not used | source-backed |
| TypeScript/Wasm | [Wasm and TypeScript getting started](wasm-getting-started.md) | [TypeScript binding README](../../bindings/typescript/README.md) | not used | source-backed |
| C# | [C# getting started](csharp-getting-started.md) | [C# binding README](../../bindings/csharp/README.md) | not used | source-backed |
| Go | [Go getting started](go-getting-started.md) | [Go binding README](../../bindings/go/README.md) | not used | source-backed |

## Notebook coverage

Notebooks are used where an executable walkthrough materially helps the
learning path:

- [Python scheduler tutorial](../../notebooks/python_scheduler_tutorial.ipynb)
- [Reproducible benchmark scenario](../../notebooks/reproducible_benchmark_scenario.ipynb)
- [Colab GPU smoke route](../../notebooks/colab_gpu_smoke.ipynb)
- [Colab TPU smoke route](../../notebooks/colab_tpu_smoke.ipynb)
- [Colab TPU smoke route](../../notebooks/colab_tpu_dedicated_smoke.ipynb)

The repository does not require a notebook for every language. The notebook
medium is reserved for executable Python-centric walkthroughs and external GPU
or TPU smoke routes where that format is a better fit than a duplicated
language notebook.

## Docs platform status

The live docs site is the Astro/Starlight site under `website/`. Track 41 keeps
this matrix as checked-in source-of-truth learning coverage, and Track 45
validates that the active Starlight platform continues to expose versioned,
polyglot-aware entry points.

## Validation

Run this from the repository root:

```powershell
node scripts/validation/validate-learning-coverage.mjs
python notebooks\validate_notebooks.py
```

The validator checks that the matrix names the supported language surfaces,
the referenced tutorial/example/notebook paths exist, and the docs platform
note still describes the active Starlight platform boundary.
