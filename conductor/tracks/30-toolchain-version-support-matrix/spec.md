# Track 30: Toolchain & Version Support Matrix

## Purpose

Own the cross-language toolchain version matrix — which Rust/Python/.NET/Julia/R/Go/Node versions are supported, when versions can be dropped, and CI runner coverage.

## Why this track exists

KairoECS targets 7+ language ecosystems across 4+ OS platforms. Without a central version matrix, each binding track may drift to different minimum-supported-version floors, creating confusion for users and CI fragmentation. This track defines the supported range, drop policy, and verification coverage.

## Primary subagent

`toolchain-agent`

## Parallelization model

This track starts immediately alongside all tracks. It owns policy documents and CI verification workflow definitions only. It does not modify binding source code or runtime behavior.

## Inputs

- `conductor/track-map.md` — binding track language list.
- `conductor/tracks.yaml` — track inventory and dependency structure.
- `.github/workflows/` — existing CI workflows (read-only reference).
- `rust-toolchain.toml` — current Rust toolchain definition.
- `pyproject.toml` equivalents from each binding track.

## Outputs

- A support matrix document (`conductor/toolchain-matrix.md`) listing every language, its minimum and latest supported version, and the supported OS/arch combinations per binding.
- A version-drop policy defining when a version can be deprecated and removed.
- A CI verification workflow (`.github/workflows/toolchain-check.yml`) that validates the matrix against actual installed tooling.
- Gate definitions for `toolchain-matrix-current` and `version-drop-policy-check`.
- Handoff notes for CI, release, and binding subagents.

## Owned paths

- `conductor/toolchain-matrix.md`
- `.github/workflows/toolchain-check.yml`
- `conductor/tracks/30-toolchain-version-support-matrix/`

## Blocked paths

- Binding source files in `bindings/` — owned by Tracks 06-11.
- Package manifests (`Cargo.toml`, `pyproject.toml`, `Package.swift`, etc.) — owned by respective binding tracks.
- Release packaging workflows — owned by Track 15.
- Rust toolchain definition (`rust-toolchain.toml`) — owned by Track 13, but this track reads and validates it.

## Acceptance criteria

- The support matrix names every language binding track (06-11) plus Rust and Wasm.
- Every language row includes min version, max version, deprecation horizon, and supported OS/arch.
- The version-drop policy specifies minimum notice period, deprecation signalling, and removal criteria.
- `toolchain-check.yml` runs on CI and fails if the installed tooling falls outside the matrix.
- The `toolchain-matrix-current` gate passes only when the matrix matches the live CI runners.
- The `version-drop-policy-check` gate passes only when no version is dropped without documented notice.

## Release implications

- This track is **release-critical**. A release cannot ship if the toolchain matrix is stale or CI runners do not match declared support.
- Binding tracks must consult the matrix before bumping minimum versions.
- The version-drop policy gates removal of any language version from the support surface.

## Non-goals

- Installing or managing toolchains on CI runners (owned by Track 13).
- Defining language-specific API compatibility rules (owned by Track 25).
- Building or testing binding packages (owned by Tracks 06-11).
- Setting MSRV for Rust crates (owned by Track 13 via `rust-toolchain.toml`).
