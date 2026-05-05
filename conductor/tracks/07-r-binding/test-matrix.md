# Test Matrix — 07 R Binding

## Required tests

- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` from `bindings/r/`.
- `R CMD check --no-manual .` from `bindings/r/`.
- `Rscript -e "devtools::check(document = FALSE)"` once `devtools` is available in the local toolchain.
- `R CMD build .` when artifact validation is needed from `bindings/r/`.
- `Rscript -e "testthat::test_dir('tests/testthat', reporter = 'summary')"` if you want to narrow execution to the package smoke tests.

## Future-surface controls

- Do not add CRAN submission automation, registry credentials, or release publication here.
- Do not pull in Julia, Python, TypeScript, C#, or Go binding concerns.
- Do not widen to core runtime changes; remain at the R package boundary.
- Stop after local package validation until Track 12 owns fixture parity and Track 15 owns release dry-runs.

## CI command

```bash
Rscript -e "testthat::test_dir('tests', reporter = 'summary')" && R CMD check --no-manual .
```

