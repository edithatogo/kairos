# Phase 0 Qualification: zip FMU Archive Replacement

**Track:** 79 (`zip-robustness`)  
**Dependency Qualified:** `zip = { version = "9.1.0", default-features = false, features = ["deflate-flate2"] }`  
**Compression Backend:** `flate2 = { version = "1.1.10", default-features = false, features = ["rust_backend"] }` (pure-Rust `miniz_oxide`)  
**Decision:** **ADOPT (GO)**  
**Date:** 2026-10-25  

---

## 1. Context and Motivation

Track 91 formally rejected `rust-fmi` (v0.8.0) due to 3 high-severity security vulnerabilities in its unmaintained `zip 0.8.6` dependency (`RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`), host `libclang` C-binding coupling, and license friction (`zlib-rs` under `Zlib`).

Pursuant to `conductor/dependency-value-policy.md` and the sequencing in `conductor/tracks/79-zip-robustness/spec.md`, Track 79 qualifies a direct, minimal, secure adoption of `zip` to replace custom stored-entry ZIP extraction and provide full FMU archive packaging.

The legacy `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` implementation:
1. Supported only uncompressed stored ZIP files (`compression_method == 0`), rejecting real-world Deflated FMUs (`method 8`) with `UnsupportedArchiveCompression`.
2. Lacked packaging capabilities (`ZipWriter`) to package exported FMUs (`.fmu`).
3. Lacked complete atomic extraction cleanup and strict decompression bomb limits.

---

## 2. Supply Chain and Maintenance Audit

| Attribute | Assessment | Result |
|---|---|---|
| **Repository** | https://github.com/zip-rs/zip2 | Active, maintained |
| **Version** | `9.1.0` (pinned) | Passed |
| **License** | MIT (`zip`), MIT OR Apache-2.0 (`flate2`), MIT OR Zlib OR Apache-2.0 (`miniz_oxide`) | Clean against `deny.toml` |
| **Security Advisories** | `cargo audit` across 398 crates found 0 vulnerabilities; `cargo deny check advisories sources licenses bans` passed with 0 errors | Clean |
| **C Toolchain Coupling** | Zero C compiler / `libclang` dependencies (`miniz_oxide` pure-Rust backend) | Clean |
| **Transitive Bloat** | Pinned minimal features (`deflate-flate2`, `rust_backend`). No bzip2, zstd, time, aes, or c-runtimes | Clean |

---

## 3. Security and Containment Contracts

1. **Path Traversal Prevention:** Rejects any entry where `enclosed_name()` is `None` or contains `..` or absolute paths (`FmiError::UnsafeArchiveEntry`).
2. **Symlink Rejection:** Rejects all symlink entries (`is_symlink()`) to prevent symlink traversal outside destination.
3. **Decompression Bomb Protection:**
   - Maximum archive files: `10,000` entries.
   - Maximum expanded uncompressed size: `500 MB` (`524,288,000` bytes).
   - Enforced both at entry metadata inspection and during stream decompression.
4. **Duplicate Entry Rejection:** Rejects duplicate relative paths inside an archive.
5. **Atomic Cleanup on Failure:** If extraction fails for any reason (CRC error, traversal, budget overflow, corrupted stream), any created files and directories under `destination` are cleaned up before returning `Err`.
6. **Deterministic Packaging:** `package_fmu_archive` walks directory trees in deterministic sorted order with Deflate compression.

---

## 4. Frozen Budgets

- **Max Entries:** 10,000 entries.
- **Max Uncompressed Byte Limit:** 500 MB (524,288,000 bytes).
- **Extraction Latency Floor:** Extract 10 MB FMU under 100 ms on test hardware.
- **Pure-Rust Guarantee:** Zero foreign C libraries.

---

## 5. Qualification Disposition

**Decision: ADOPT.**  
Proceed to Phase 1 replacement behind `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` and `crates/kairo-ecs-fmi/src/export/packager.rs`.
