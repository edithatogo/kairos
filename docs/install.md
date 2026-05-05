# Installing KairoECS

## Rust (core engine)

### Prerequisites
- Rust toolchain (stable, via rustup)
- `cargo` package manager

### Install
```bash
git clone https://github.com/edithatogo/kairos.git
cd kairos
cargo build --workspace
```

### Verify
```bash
cargo test --workspace
```

## Python binding

### Prerequisites
- Python 3.10 - 3.14
- pip / venv

### Install
```bash
cd bindings/python
pip install -e .
```

### Verify
```bash
pytest
```

## R binding

### Prerequisites
- R >= 4.2
- R package manager

### Install
```r
install.packages("bindings/r", repos = NULL, type = "source")
```

### Verify
```r
library(kairoECS)
kairoecs_surface_ready()
```

## Julia binding

### Prerequisites
- Julia >= 1.10

### Install
```julia
using Pkg
Pkg.activate("bindings/julia")
Pkg.instantiate()
```

### Verify
```julia
using KairoECS
KairoECS.self_check()
```

## TypeScript/Wasm binding

### Prerequisites
- Node.js >= 20 LTS
- npm

### Install
```bash
cd bindings/typescript
npm install
npm run build
```

### Verify
```bash
npm test
```

## C# binding

### Prerequisites
- .NET SDK 10.0 or 11.0 preview

### Install
```bash
cd bindings/csharp
dotnet restore Kairo.ECS.sln
dotnet build Kairo.ECS.sln -c Release
```

### Verify
```bash
dotnet test Kairo.ECS.sln
```

## Go binding

### Prerequisites
- Go >= 1.23

### Install
```bash
cd bindings/go
go mod tidy
```

### Verify
```bash
go test ./...
```

## Development environment

For a reproducible polyglot development environment:
```bash
just dev-validate    # Check all toolchains
just dev-setup       # Install required Rust components
just docs-build      # Build documentation site
```

## Docker

```bash
docker build -t kairo-ecs-cli -f docker/Dockerfile .
```

## Troubleshooting

- **Rust build fails**: Ensure `rustup` is installed and `rust-toolchain.toml` matches.
- **Python import fails**: Ensure `KAIRO_ECS_FFI_LIBRARY` environment variable points to the native library.
- **C# build fails**: Ensure .NET SDK matches `bindings/csharp/global.json`.
- **Go build fails**: Ensure Go >= 1.23 and CGO_ENABLED is set.
