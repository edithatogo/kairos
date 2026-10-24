# Phase 0 Evidence & Value Gate Qualification: sha2

**Track:** 93-sha2-integrity  
**Date:** 2026-10-24  
**Agent:** arrow-agent  
**Status:** Phase 0 Complete — Adoption Decision: **ADOPT**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `arrow-agent`
- **Target Paths:**
  - `crates/kairo-ecs-arrow/src/lib.rs`: `sha256_simple` (lines 2063–2075), `ParquetZstdCheckpointWriter::write_checkpoint` (line 2019), `ParquetZstdCheckpointReader::read_checkpoint` (line 2036).
- **Existing Problem:**
  `sha256_simple` is a handwritten 64-bit linear congruential generator (acc-based) that replicates 8 bytes across a 32-byte slice. It is **not** SHA-256. It fails standard NIST CAVP vectors and provides zero cryptographic preimage or collision resistance.
- **Reconciliation with Track 70:**
  Track 70 completed official Parquet, IPC, Arrow RecordBatch, and C Data Interface Schema v2. Track 93 replaces `sha256_simple` with standard FIPS 180-4 SHA-256 (`compute_sha256` and `compute_sha256_hex`) and updates checkpoint verification to standard SHA-256. Noncryptographic hashes (`fnv1a_hash`) and RNG streams remain untouched.

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `sha2` (RustCrypto)
- **Version:** `0.10.8` (or `0.10.9`)
- **Repository:** https://github.com/RustCrypto/hashes
- **License:** MIT OR Apache-2.0 (compatible with Kairos workspace license)
- **MSRV:** Rust 1.41+ (compatible with Kairos 1.85 / 1.99)
- **Platforms:** `x86_64`, `aarch64`, `wasm32-unknown-unknown`
- **Security Audit:** 0 known vulnerabilities; official RustCrypto maintenance; fuzz testing upstream.
- **Transitive Footprint:**
  - `digest v0.10.7`
  - `crypto-common v0.1.7`
  - `block-buffer v0.10.4`
  - `generic-array v0.14.7`
  - `typenum v1.20.1`
  - `cpufeatures v0.2.17`
  No network or heavy runtime dependencies; builds cleanly offline in < 4 seconds.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain Custom (`sha256_simple`) | Adopt `sha2` |
|---|---|---|
| Standard Compliance | **FAIL** (Not SHA-256; fails NIST CAVP) | **PASS** (FIPS 180-4 SHA-256) |
| Polyglot Interop | **FAIL** (Python `hashlib` incompatible) | **PASS** (Bit-for-bit `hashlib.sha256`) |
| Collision Resistance | **FAIL** (64-bit state, trivial collisions) | **PASS** (Cryptographic 256-bit security) |
| Allocation Cost | 0 heap allocations | 0 heap allocations (fixed `[u8; 32]`) |
| Maintained Code | 25 lines custom pseudo-hash | 0 lines custom crypto; standard API |

**Adoption Decision:** **ADOPT**  
Approved for Phase 1 replacement.

---

## 4. Frozen Budgets & Acceptance Oracles

- **Performance Budget:** Throughput > 200 MB/s; 0 heap allocations for core `compute_sha256`.
- **Memory Budget:** Exact 32-byte digest array.
- **Acceptance Oracles:**
  1. Standard NIST CAVP vectors: Empty string `""`, short string `"abc"`, 56-byte, 64-byte block boundary, and multiblock payloads.
  2. Python `hashlib.sha256` bit-for-bit equivalence.
  3. Single-bit corruption detection in checkpoint body and checksum trailer.
  4. Explicit rejection of invalid length or legacy pseudo-hash format.

---

## 5. Phase 0 Verification

Baseline failing tests implemented in `crates/kairo-ecs-arrow/tests/track93_phase0_sha2_baseline.rs`:
- `baseline_pseudo_checkpoint_trailer_fails_standard_sha256` (passes as should_panic)
- `standard_sha256_primitive_must_exist` (passes as should_panic)
