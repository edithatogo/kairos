# Handoff: Track 30 Toolchain & Version Support Matrix

## Summary

Defined the cross-language toolchain version support matrix covering 8 language/ecosystem rows (Rust, Python 3.10-3.14, .NET 10-11, Julia, R, Go, Node/Wasm) across 5 OS/arch targets. Established a version-drop policy requiring 2-cycle/6-month deprecation notice and a `toolchain-check.yml` CI workflow that validates live runner tooling against the declared matrix.

## Files changed

`conductor/tracks/30-toolchain-version-support-matrix/plan.md`, `conductor/tracks/30-toolchain-version-support-matrix/spec.md`, `conductor/tracks/30-toolchain-version-support-matrix/test-matrix.md`, `conductor/tracks/30-toolchain-version-support-matrix/handoff.md`, `conductor/toolchain-matrix.md`, `.github/workflows/toolchain-check.yml`, `conductor/quality-gates.md`

## Contracts consumed

- `conductor/track-map.md` — binding track language list.
- `conductor/tracks.yaml` — track inventory.
- `rust-toolchain.toml` — Rust toolchain reference (read-only).
- Existing `.github/workflows/` for CI runner reference (read-only).

## Release gates affected

- **toolchain-matrix-current**: Blocks release if any declared supported version lacks CI verification.
- **version-drop-policy-check**: Blocks release if a version was dropped without prior deprecation notice.
- Both gates are referenced in `conductor/quality-gates.md` and the release checklist (Track 15).

## Risks and unresolved questions

- Several binding tracks (07, 08, 11) may not have explicit minimum version declarations yet — the matrix must be backfilled once those tracks produce package manifests.
- CI runner OS/arch coverage for aarch64 Linux and macOS remains sparse; matrix cells for those targets may be marked "best-effort" until runners are provisioned by Track 13.
- Rapid upstream release cycles (Node, Go, Julia) may require matrix updates more frequently than the release cycle — a "latest supported" column with a lag tolerance should be considered.
