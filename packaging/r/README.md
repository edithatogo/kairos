# R Packaging

Track 07 owns only the local R binding/package validation slice. Release
publication, registry automation, and native runtime artifact packaging remain
gated on the packaging/release tracks.

Current local packaging gate:

- `Rscript` is available through the local Scoop R installation.
- Base-R and `testthat` package smoke checks are runnable from `bindings/r/`.
- `Rcmd check --no-manual r` is the preferred Windows check command from
  `bindings/`, because `R` can resolve to PowerShell history aliases.
- Optional Arrow-backed roundtrip coverage is present in the test suite and is
  skipped unless the R `arrow` package is installed.
