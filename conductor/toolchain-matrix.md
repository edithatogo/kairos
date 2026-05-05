# Toolchain & Version Support Matrix

This document records the supported toolchain floor for the repo and the verification path for each ecosystem.

## Rust

- Minimum: `rustc` from `rust-toolchain.toml`
- Validation: `cargo fmt --all --check`, `cargo check --workspace`, `cargo test --workspace --no-run`

## Python

- Supported: CPython 3.10, 3.11, 3.12, 3.13, 3.14
- Validation: `pytest`, `ruff`, type checks, wheel build

## R

- Supported: current CRAN-style toolchain on the CI runners
- Validation: `R CMD build`, `R CMD check --no-manual`

## Julia

- Supported: current LTS-compatible Julia with project env
- Validation: `Pkg.test`, `Pkg.build`

## TypeScript

- Supported: current Node LTS with `npm`
- Validation: `npm ci`, `npm run typecheck`, `npm test`, `npm pack`

## C#

- Supported: .NET 10 stable
- Preview: .NET 11 experimental lane only
- Validation: `dotnet test`, `dotnet pack`

## Go

- Supported: current stable Go toolchain
- Validation: `go test`, `go vet`, `gofmt`

## Control tracks

- Track 30 owns this matrix and keeps it synchronized with CI runner reality.
- Track 31 watches for regressions when the declared toolchain floor changes.
