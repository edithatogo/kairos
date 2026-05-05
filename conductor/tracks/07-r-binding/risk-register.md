# Risk Register — 07 R Binding

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| CRAN submission latency | 4 | 3 | 12 | Maintain GitHub R-universe binary repo as fast release channel; queue CRAN submission on tag but do not block downstream on CRAN acceptance | r-agent | CRAN submission pending >2 weeks after release |
| R C ABI fragility (R CMD check NOTE/WARNING) | 3 | 5 | 15 | Run `R CMD check --as-cran` in CI on all platforms; fuzz C call interfaces with sanitizers on Linux | r-agent | Sanitizer flags any C call interface |
| System Arrow library dependency resolution | 3 | 4 | 12 | Bundle Arrow C Data Interface header; ship pkg-config fallback script in `tools/`; accept `LIBARROW_HOME` env var at install time | r-agent | `configure`/`configure.win` fails on any CI target |
| testthat/pkgdown CI complexity | 3 | 2 | 6 | Pin testthat/pkgdown versions in `DESCRIPTION` Suggests; use `renv.lock` for CI reproducibility | r-agent | CI R-lang lane fails for >3 consecutive runs |
| CRAN NOTE about compiled code size/portability | 3 | 4 | 12 | Strip debug symbols before packaging; avoid `-march=native` in `Makevars`; validate with `rhub::check_for_cran()` | r-agent | `rhub::check_for_cran()` reports NOTE/WARNING |
