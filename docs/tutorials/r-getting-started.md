# R Getting Started

**Runtime support:** Package information, surface checks and Arrow field definitions are metadata/data helpers. The former R scheduler is unsupported; native scheduling remains unavailable until its adapter passes installed-library acceptance.

Rust is the only supported simulation core. Follow [ADR 0010](../adr/0010-rust-only-execution.md), [installation](../install.md) and the [migration guide](../release/migration.md). The package checks below establish only their stated package/data scope.

## Prerequisites
- R >= 4.2 installed
- The KairoECS repository cloned

## Quickstart

### 1. Install the R package
```r
install.packages("bindings/r", repos = NULL, type = "source")
library(kairoECS)
```

### 2. Verify the package surface
```r
# Check package info
kairoecs_package_info()

# Verify surface readiness
kairoecs_surface_ready()
```

### 3. Run smoke tests
```bash
cd bindings/r
Rscript -e 'testthat::test_dir("tests/testthat")'
```

## Package structure

| File | Purpose |
|---|---|
| `R/kairoecs.R` | Package source (package info, surface checks, Arrow field definitions) |
| `tests/testthat/test-smoke.R` | Smoke tests |
| `DESCRIPTION` | Package metadata |
| `NAMESPACE` | Exported symbols |

## Next steps

- Read the [R binding README](../../bindings/r/README.md)
- Explore [Arrow schema reference](../arrow/schema-reference.md)
- Try the [factory bottleneck tutorial](../scenarios/factory-bottleneck-run-replay.md)

## Validation

```bash
R CMD check bindings/r
```
