# Handoff: Track 79 (zip FMU Archive Replacement)

Updated: 2026-10-25. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `zip = "9.1.0"` with pure-Rust `flate2 = "1.1.10"` (`rust_backend` via `miniz_oxide`). Replaced custom stored-only ZIP extraction in `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` with robust `zip::ZipArchive` decoding. Implemented strict multi-vector security protections: symlink rejection, enclosed path traversal prevention, duplicate name rejection, SUID/SGID bit stripping with 0o700 permissions, file count limit (`MAX_ARCHIVE_FILES = 10,000`), expansion stream byte limit (`MAX_EXPANDED_BYTES = 500 MB`), and atomic directory cleanup on extraction failure. Implemented deterministic FMU export archive packaging with Deflate compression in `crates/kairo-ecs-fmi/src/export/packager.rs`. Added 11 independent oracle tests in `tests/fmu_archive_oracles.rs`.

## Files changed

- `Cargo.lock`
- `crates/kairo-ecs-fmi/Cargo.toml`
- `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`
- `crates/kairo-ecs-fmi/src/export/packager.rs`
- `crates/kairo-ecs-fmi/tests/fmu_archive.rs`
- `crates/kairo-ecs-fmi/tests/fmu_archive_oracles.rs`
- `conductor/tracks/79-zip-robustness/evidence/phase0-qualification.md`
- `conductor/tracks/79-zip-robustness/adr-api-compatibility-and-redteam.md`
- `conductor/tracks/79-zip-robustness/review-report.md`
- `conductor/tracks/79-zip-robustness/plan.md`
- `conductor/tracks/79-zip-robustness/handoff.md`
- `conductor/tracks/79-zip-robustness/metadata.json`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `fmi-agent` ownership contract, and `spec.md`.

## Contracts changed

Extended `crates/kairo-ecs-fmi` to support both Deflated (method 8) and Stored (method 0) FMU archives and ZIP64 extensions. Added `export::package_fmu_archive(unpacked_root, output_fmu_path)` for deterministic `.fmu` packaging.

## Tests added

- `crates/kairo-ecs-fmi/tests/fmu_archive_oracles.rs`:
  - `oracle_compressed_deflate_extraction_and_layout`
  - `oracle_stored_archive_backward_compatibility`
  - `oracle_zip64_archive_extraction`
  - `oracle_path_traversal_rejection_variations`
  - `oracle_symlink_rejection`
  - `oracle_duplicate_entries_rejection`
  - `oracle_expansion_limit_enforcement`
  - `oracle_file_count_budget_enforcement`
  - `oracle_corrupted_crc_and_cleanup_on_failure`
  - `oracle_unsupported_compression_method`
  - `oracle_fmu_packager_roundtrip`
- `crates/kairo-ecs-fmi/tests/fmu_archive.rs`:
  - Updated to compliant standard ZIP fixtures with `ZipWriter`.

## Known risks

Zip extraction is confined to verified sandboxed extraction targets with secure parent ownership. Symlinks and paths escaping destination roots are strictly rejected.

## Follow-up issues

Advance to Track 80 (`quick-xml FMU metadata replacement`), which consumes the extracted `modelDescription.xml` produced by `FmuArchive`.

## Integration notes

All 42 tests in `kairo-ecs-fmi` passed. Pure-Rust compilation verified with zero C or `libclang` dependencies. Passes `cargo deny check advisories sources licenses bans` (0 advisories, 0 bans, 0 license errors) and `cargo audit` (398 dependencies scanned, 0 vulnerabilities).

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Adopted `zip 9.1.0` and pure-Rust `flate2 1.1.10` (`miniz_oxide`); transactional extraction cleanup; multi-layer traversal and decompression bomb protection.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Command: `cargo test -p kairo-ecs-fmi --all-features`
  - CWD: `/Volumes/PortableSSD/github/kairos`
  - Toolchain: `rustc 1.99.1`
  - Commit SHA: 4ec9bbdc23145c0de47479804fb586d1c925d48e
  - Exit code: `0`
- pushed ref: origin/main
- next-phase decision: Track 79 is Done; advance to Track 80.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
