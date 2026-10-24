# ADR: Adoption of `zip` 9.1.0 and Pure-Rust Backend for FMU Archives

**Track:** 79 (`zip-robustness`)  
**Status:** Accepted  
**Deciders:** fmi-agent, lead-architect  
**Date:** 2026-10-25  

---

## 1. Context and Problem Statement

FMUs (Functional Mock-up Units) are distributed as `.fmu` files, which are standard ZIP archives containing a `modelDescription.xml` document, platform-specific binaries (`binaries/<platform>/...`), documentation, and simulation resources.

Prior to this track, `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` employed a rudimentary hand-rolled stored-entry ZIP reader that only supported uncompressed files (`compression_method == 0`), rejecting Deflate compression (`method 8`) with `UnsupportedArchiveCompression`. Furthermore, the engine had no built-in FMU archive packaging capabilities (`ZipWriter`) for model export.

Track 91 rejected `rust-fmi` due to 3 high-severity security vulnerabilities in its unmaintained `zip 0.8.6` dependency (`RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`), C/`libclang` toolchain coupling, and license friction (`zlib-rs`).

Track 79 addresses this by directly qualifying and adopting `zip` 9.1.0 with minimal features and a pure-Rust backend.

---

## 2. Decision and Architecture

We adopt:
- `zip = { version = "9.1.0", default-features = false, features = ["deflate-flate2"] }`
- `flate2 = { version = "1.1.10", default-features = false, features = ["rust_backend"] }`

### Pure-Rust Compression
Selecting `flate2` with `features = ["rust_backend"]` routes compression and decompression through pure-Rust `miniz_oxide`, which:
1. Completely avoids C compiler toolchain dependencies and host `libclang`.
2. Avoids license issues associated with `zlib-rs`.
3. Fully satisfies workspace license policies under `deny.toml`.

### API Compatibility
The public interface remains strictly backward-compatible:
- `FmuArchive::new(path)`
- `FmuArchive::path(&self) -> &Path`
- `FmuArchive::open_unpacked(root) -> FmiResult<FmuLayout>`
- `FmuArchive::extract_to(&self, destination) -> FmiResult<FmuLayout>`
- `export::package_fmu_archive(unpacked_root, output_fmu_path) -> FmiResult<()>`
- `export::write_unpacked_fmu(root, model_description) -> FmiResult<FmuPackageLayout>`

---

## 3. Red-Team Security Analysis and Mitigations

| Vulnerability Vector | Threat Scenario | Mitigation in Track 79 |
|---|---|---|
| **SUID/SGID Privilege Escalation (`RUSTSEC-2026-0367`)** | Malicious archive sets SUID/SGID bits during extraction | `zip 9.1.0` patches this vulnerability; extraction ignores external SUID bits and creates destination with strict `0o700` directory permissions. |
| **Symlink Directory Traversal (`RUSTSEC-2026-0368`)** | Archive extracts symlink pointing outside destination, followed by file write through symlink | Strict rejection of any entry where `entry.is_symlink() == true` with `FmiError::UnsafeArchiveEntry`. No symlinks are created. |
| **Unconfined Unix Permissions (`RUSTSEC-2026-0369`)** | Malicious permissions applied across symlinks | Pinned `zip 9.1.0` patches this; combined with symlink rejection, no foreign file system permissions can escape. |
| **Path Traversal (`../evil.txt`)** | Relative or absolute path escaping destination root | Both `entry.enclosed_name()` and `safe_archive_entry` validate each entry. Any traversal component (`..`), root prefix (`/`), or Windows drive letter is rejected with `FmiError::UnsafeArchiveEntry`. |
| **Decompression Bomb (ZIP Bomb)** | Small archive expanding to hundreds of gigabytes | Multi-layer budget enforcement: (1) Entry count capped at `MAX_ARCHIVE_FILES = 10,000`; (2) Total declared uncompressed size capped at `MAX_EXPANDED_BYTES = 500 MB`; (3) Decompression stream read wrapped in `take(remaining_budget + 1)` to abort if stream exceeds limit. |
| **Duplicate Filename Hijacking** | Archive contains duplicate entries to bypass validators | `archive.has_duplicate_names()` and runtime `HashSet<PathBuf>` reject any duplicate entry names. |
| **State Corruption / Partial Leaks** | Extraction fails midway due to invalid CRC or truncated stream | Extraction is performed transactionally: if any error occurs, `destination` is completely removed (`fs::remove_dir_all(destination)`). |

---

## 4. Consequences and Verification

- Supported compression methods: Stored (0) and Deflate (8). Unsupported methods return `FmiError::UnsupportedArchiveCompression`.
- 11 comprehensive oracle tests in `crates/kairo-ecs-fmi/tests/fmu_archive_oracles.rs` verify all security, budget, format, and packaging contracts.
- Deterministic roundtrip packaging verified bit-for-bit.
