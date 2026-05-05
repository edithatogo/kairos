# R Binding

This package root contains the Track 07 R binding surface.

Current package files:

- `DESCRIPTION`
- `LICENSE`
- `NAMESPACE`
- `R/kairoecs.R`
- `man/kairoECS-package.Rd`
- `tests/testthat.R`
- `tests/helper-load.R`
- `tests/testthat/helper-load.R`
- `tests/testthat/test-smoke.R`

Local validation from `bindings/r/`:

- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"`
- `R CMD check --no-manual .`
- `Rscript -e "devtools::check(document = FALSE)"`
