# Toolchain & Version Support Matrix

Last refreshed: 2026-05-07.

This document is the single source of truth for KairoECS language and runner version support. Binding tracks may support a narrower feature surface while they are still scaffolding, but they must not raise a minimum version or drop a supported version without following the version-drop policy below.

## Current Source Evidence

| Source | Evidence used |
|---|---|
| `rust-toolchain.toml` | Rust channel is `stable`; components are `rustfmt` and `clippy`. |
| `Cargo.toml` | Workspace `rust-version` is `1.76`; edition is `2021`. |
| `mise.toml` | Repo-local developer defaults are Rust `stable`, Python `3.14`, Node `lts`, Go `latest`, Julia `latest`, R `latest`, .NET `10.0`. |
| `bindings/python/pyproject.toml` | Python binding declares `requires-python = ">=3.10"`. |
| `bindings/r/DESCRIPTION` | R binding declares `Depends: R (>= 4.2)`. |
| `bindings/julia/Project.toml` | Julia binding declares `[compat] julia = "1.10"`. |
| `bindings/typescript/package.json` | TypeScript binding declares `engines.node = ">=22 <25"` for the production Node support floor. |
| `bindings/csharp/global.json` | C# SDK default is `10.0.202` with `latestFeature` roll-forward. |
| `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` | C# binding targets `net10.0;net11.0`. |
| `bindings/go/go.mod` | Go binding declares `go 1.23`. |
| `.github/workflows/ci-bindings.yml` | Current binding CI covers Python 3.10-3.14, .NET 10/11, Node LTS, Go stable, and Ubuntu hosted runners. |

## Support Matrix

Support labels:

- `CI-covered`: Track 30 or existing binding CI runs the lane on GitHub-hosted runners.
- `best-effort`: Supported by policy, but not yet covered by hosted CI.
- `unsupported`: Not part of the promised support surface for this release stage.

| Ecosystem | Binding track | Minimum supported version | Latest/current supported version | Experimental or preview lane | CI selector | Deprecation horizon | Linux x86_64 | Linux aarch64 | macOS x86_64 | macOS aarch64 | Windows x86_64 |
|---|---:|---|---|---|---|---|---|---|---|---|---|
| Rust core | 00/01/13 | MSRV `1.76`; default channel `stable` | Rust `1.95.x` stable as of 2026-05-06 | Rust `beta` advisory lane | `stable`, `beta` | MSRV may rise only after 2 release cycles or 6 months notice; `stable` tracks upstream stable. | CI-covered | best-effort | best-effort | best-effort | best-effort |
| Python binding | 06 | CPython `3.10` | CPython `3.14.x` | CPython 3.14 free-threaded smoke where runner support exists | `3.10`, `3.11`, `3.12`, `3.13`, `3.14` | Drop only after upstream PSF security support ends and 2 cycles/6 months notice is complete. | CI-covered | best-effort | best-effort | best-effort | best-effort |
| R binding | 07 | R `4.2` package floor; CI floor is previous CRAN release | R `4.6.x` current release | R-devel advisory lane only | `oldrel-1`, `release` | Drop a package floor only after CRAN support pressure or dependency incompatibility is documented for 2 cycles/6 months. | CI-covered | best-effort | best-effort | best-effort | best-effort |
| Julia binding | 08 | Julia `1.10` LTS-compatible floor | Julia `1.12.x` current stable | Julia `1.13` beta advisory lane only | `1.10`, `1.12` | Drop an LTS-compatible floor only after Julia LTS guidance changes and 2 cycles/6 months notice is complete. | CI-covered | best-effort | best-effort | best-effort | best-effort |
| TypeScript/Wasm binding | 09 | Node `22` LTS | Node `24` Active LTS/current production lane | Node `25` current advisory lane; Wasm browser smoke best-effort | `22`, `24` | Drop an LTS major only after Node EOL and 2 cycles/6 months notice is complete. | CI-covered | best-effort | best-effort | best-effort | best-effort |
| C# binding | 10 | .NET SDK `10.0.x`; target `net10.0` | .NET SDK `10.0.x` LTS | .NET SDK `11.0.x` preview; target `net11.0` is allowed to fail only while explicitly marked experimental | `10.0.x`, `11.0.x` | Drop a TFM only after Microsoft support ends or package validation becomes unsustainable with 2 cycles/6 months notice. | CI-covered | best-effort | best-effort | best-effort | best-effort |
| Go binding | 11 | Go module floor `1.23`; CI support floor `1.25` | Go `1.26.x` | Go tip advisory lane only | `1.25.x`, `1.26.x` | Drop a Go release only when it is older than the two supported upstream releases and 2 cycles/6 months notice is complete. | CI-covered | best-effort | best-effort | best-effort | best-effort |

## Rust

The Rust support row is the source of truth for the core workspace MSRV, stable CI lane, and beta advisory lane.

On Windows developer hosts, `scripts/validate_conductor_setup.ps1` prefers the installed `stable-x86_64-pc-windows-gnu` Rust toolchain for local workspace tests when it is available. This avoids accidental resolution of Git's `link.exe` on hosts without a working MSVC linker while keeping Windows runner coverage `best-effort` until Track 13 provisions hosted or self-hosted Windows lanes.

## Python

The Python support row is the source of truth for the CPython binding floor, active CI versions, and free-threaded smoke posture.

## R

The R support row is the source of truth for the R package floor, CRAN release lane, and R-devel advisory posture.

## Julia

The Julia support row is the source of truth for the Julia package floor, stable CI lane, and beta advisory posture.

## TypeScript

The TypeScript/Wasm support row is the source of truth for Node/Wasm binding CI selectors and browser smoke posture.

## C#

The C# support row is the source of truth for .NET SDK lanes, target frameworks, and preview handling.

Local Windows validation must not set `MSBuildSDKsPath` to the .NET 11 preview SDK when running the Track 10 net10 lane. The repository `global.json` selects SDK `10.0.202`; a stale preview `MSBuildSDKsPath` causes MSBuild task-host resolution failures before project compilation. The local net10 lane passed after clearing `MSBuildSDKsPath`, setting `DOTNET_CLI_TELEMETRY_OPTOUT=1`, disabling shared compilation, and using single-node MSBuild with node reuse disabled.

The experimental net11 lane requires invoking the preview SDK outside the `bindings/csharp` `global.json` scope. Local restore passes with `C:\Users\60217257\scoop\apps\dotnet-sdk-preview\current\dotnet.exe` from the repository root, but the preview build is blocked in this shell by Roslyn named-pipe access denial under `\\.\pipe\LOCAL\dotnet_*`.

## Go

The Go support row is the source of truth for the Go module floor, CI release lanes, and Go tip advisory posture.

## Node/Wasm

The Node/Wasm support row is represented by the TypeScript/Wasm binding row above.

## Runner Coverage Policy

- Track 30 validates Ubuntu-hosted `x86_64` lanes because every existing workflow currently uses `ubuntu-latest` except the Conductor setup validation workflow.
- macOS, Windows, and Linux aarch64 cells stay `best-effort` until Track 13 provisions corresponding hosted or self-hosted runner lanes.
- A release candidate must not mark any required platform `best-effort`; either promote it to `CI-covered` with evidence or mark it `unsupported`.
- The matrix must be refreshed within one KairoECS release cycle after a new Rust, Python, .NET, Julia, R, Go, or Node major/minor release becomes generally available.

## Version-Drop Policy

Dropping a supported version means removing a version from the support matrix, raising a minimum supported version, removing a CI lane, or deleting a package target framework/runtime lane.

Required sequence:

1. Record the proposed drop in this document with the affected ecosystem, current version, replacement version, reason, and first release where removal is allowed.
2. Announce the deprecation in release notes and the relevant binding README for at least two KairoECS release cycles or six calendar months, whichever is longer.
3. Keep a CI lane, compatibility smoke, or explicit waiver while the version is in the notice period.
4. Before removal, cite at least one removal criterion: upstream vendor EOL, unsupported runner image, unmaintained dependency chain, security exposure, or documented packaging impossibility.
5. For exceptions, add an ADR or release-governance waiver naming the approver, expiry, affected users, and compensating control.

The `version-drop-policy-check` gate fails if a version disappears from the matrix without a deprecation notice or waiver. The `toolchain-matrix-current` gate fails if a `CI-covered` selector cannot be installed or reports a version outside the declared row.

## Proposed Drops

| Ecosystem | Version or lane | Notice started | Earliest removal | Reason | Status |
|---|---|---|---|---|---|
| Node.js | `20.x` | 2026-05-06 | 2026-11-06 | Node 20 reached EOL on 2026-04-30; KairoECS CI now prefers Node 22 and 24. | Deprecated; no new CI lanes should be added. |
| Go | `1.24.x` package dry-run lane | 2026-05-06 | 2026-11-06 | Go supports the two most recent releases; Track 30 CI now covers 1.25 and 1.26. | Deprecated; package dry-run now uses the supported Go 1.25 floor. |

## Maintainer Update Procedure

1. Update the row in `conductor/toolchain-matrix.md`.
2. Update `.github/workflows/toolchain-check.yml` selectors for any `CI-covered` lane changes.
3. Run `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1`.
4. Run focused workflow syntax/static validation if available locally.
5. Update Track 30 `test-matrix.md`, `risk-register.md`, and `handoff.md` with exact evidence and commands.
