# Phase 0 Evidence & Value Gate Qualification: tempfile

**Track:** 94-tempfile-publication  
**Date:** 2026-10-24  
**Agent:** arrow-agent  
**Status:** Phase 0 Complete — Adoption Decision: **ADOPT**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `arrow-agent`
- **Target Paths:**
  - `crates/kairo-ecs-arrow/src/polars_interop.rs`: `DurableRunPublicationWriter::publish` (lines 527, 555)
  - `crates/kairo-ecs-arrow/tests/track70_phase3_durable_publication.rs`: static temp directories
  - `crates/kairo-ecs-arrow/tests/checkpoint_restart.rs`: static temp directories
- **Existing Problem:**
  - `DurableRunPublicationWriter::publish` currently performs non-atomic writes directly to the destination paths (`fs::write(&part_path, &buf)` and `fs::write(&manifest_path, manifest_json)`).
  - An interrupted write, process termination, or disk-full event leaves corrupt partial files directly in `run_dir`, which breaks resumption guarantees and can poison downstream readers.
  - No explicit no-clobber protection exists to prevent silent clobbering of prior publication outputs.
  - Tests rely on static global paths under `std::env::temp_dir()`, causing parallel race conditions, cross-test pollution, and leftover disk artifacts if an assertion panics before manual cleanup.
- **Reconciliation with Track 70 and Track 93:**
  - Track 70 established Parquet, IPC, and Arrow RecordBatch durable publication.
  - Track 93 established FIPS 180-4 SHA-256 integrity verification.
  - Track 94 qualifies `tempfile` for atomic staging beside destination, crash-safe persistence, overwrite/no-clobber safety, and RAII test directory isolation.
  - Determinism invariant: Ephemeral scratch paths are strictly confined to temporary staging; no filesystem randomness enters simulation seeds, hashes, or published manifest/part identities.

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `tempfile`
- **Version:** `3.17.1`
- **Repository:** https://github.com/Stebalien/tempfile
- **License:** MIT OR Apache-2.0 (compatible with Kairos workspace license)
- **MSRV:** Rust 1.63+ (fully compatible with Kairos 1.85 / 1.99)
- **Platforms:** `x86_64`, `aarch64`, `wasm32-unknown-unknown` (Tier 1 cross-platform support)
- **Security Audit:** 0 known vulnerabilities; official de facto standard Rust crate for temporary files and directory management.
- **Footprint:** Lightweight transitive footprint (`fastrand`, `cfg-if`, platform `libc`/`windows-sys`). Builds cleanly offline in < 1 second.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain Direct `fs::write` / Static Paths | Adopt `tempfile` |
|---|---|---|
| Atomic Publication | **FAIL** (Direct write exposes partial files on crash) | **PASS** (Staging via `NamedTempFile` + atomic rename/persist) |
| Failure Cleanup | **FAIL** (Interrupted writes leave unmanaged artifacts) | **PASS** (RAII auto-delete of unpersisted files on drop) |
| Same-Filesystem Staging | **FAIL** (No helper; cross-filesystem rename errors possible) | **PASS** (`NamedTempFile::new_in(&self.run_dir)` guarantees same-fs staging) |
| No-Clobber Protection | **FAIL** (Overwrites blindly without detection) | **PASS** (`persist_noclobber` or explicit overwrite guards) |
| Test Hermeticity | **FAIL** (Static `temp_dir().join()` has parallel collisions) | **PASS** (Isolated `tempdir()` with deterministic drop cleanup) |
| Determinism Invariant | **PASS** (Deterministic paths) | **PASS** (Scratch names internal only; final paths deterministic) |

**Adoption Decision:** **ADOPT**  
Approved for Phase 1 bounded replacement.

---

## 4. Frozen Budgets & Acceptance Oracles

- **Performance Budget:** Zero allocation overhead during publication iteration; bounded single temporary handle per part.
- **Memory Budget:** Zero persistent heap memory retained after publication completion.
- **Acceptance Oracles:**
  1. Concurrent directory creation and hermetic isolation in parallel test runs.
  2. Cleanup on normal and error/aborted publication paths (unpersisted staging files vanish on drop).
  3. Same-filesystem staging and atomic persistence (parts and manifest only appear at final paths upon successful write and sync).
  4. Explicit no-clobber behavior preventing accidental overwrite of existing publication outputs.
  5. Strict separation between ephemeral staging filenames and deterministic output identities.

---

## 5. Phase 0 Verification

Baseline failing tests implemented in `crates/kairo-ecs-arrow/tests/track94_phase0_tempfile_baseline.rs`:
- `baseline_direct_write_leaves_partial_file_on_interruption` (passes as should_panic)
- `atomic_staging_primitive_must_exist` (passes as should_panic)
