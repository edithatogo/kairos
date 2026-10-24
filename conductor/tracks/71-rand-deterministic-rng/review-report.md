# Review Report: Track 71 — rand Deterministic RNG Adoption

**Track:** 71-rand-deterministic-rng  
**Date:** 2026-10-24  
**Reviewer:** Principal Software Engineer / Conductor Review Agent  
**Outcome:** PASS — Phase 0, 1, and 2 accepted; Ready for PR and merge

---

## 1. Scope & Ownership Audit

- **Lead Agent:** `rng-agent` + `conformance-agent`
- **Owned Paths Modified:**
  - `crates/kairo-ecs-rng/Cargo.toml`
  - `crates/kairo-ecs-rng/src/lib.rs`
  - `Cargo.lock`
  - `conformance/fixtures/rng_replay.json`
  - `tests/conformance/runner.mjs`
  - `conductor/tracks/71-rand-deterministic-rng/*`
- **No Unowned or Blocked Paths Touched:** Verified. Scheduling semantics, PDES, Arrow tabular processing, and other repositories remain completely untouched.

---

## 2. Value Policy & Contract Conformance

- **Target Replacement:**
  Adopted `rand = "0.10"` and `rand_chacha = "0.10"` (with `default-features = false`). Replaced bespoke SplitMix64 generator with cryptographically robust `ChaCha8Rng`.
- **Determinism & Stream Isolation:**
  - Zero OS/thread-local randomness in simulation.
  - Full stream isolation via `set_stream(u64)` and `get_stream() -> u64`.
  - Canonical derivation from run seed, full generational entity ID (`EntityId`), and purpose namespace string via `derive_canonical_seed` and `derive_stream_id`.
  - Intended Common Random Number (CRN) coupling preserved across scenarios.
- **Mid-Buffer Checkpoint Restoration:**
  - `RngCheckpoint` captures `generator`, `version`, `seed: [u8; 32]`, `stream_id: u64`, and `word_pos: u128`.
  - Native word position tracking via `set_word_pos(u128)` and `get_word_pos() -> u128`.
  - Incompatible generator names and versions reject cleanly with `RngError::UnsupportedGenerator` and `RngError::UnsupportedVersion`.
- **Alpha Trajectory Break:**
  - Replaced SplitMix64 trajectory with ChaCha8 trajectory as explicitly planned in `spec.md`.
  - Rebased `conformance/fixtures/rng_replay.json` and conformance runner to verified ChaCha8 draws `[4087599687, 2415409399, 675359370, 3234763134]`.

---

## 3. Test Matrix & Gate Verification

| Gate | Status | Evidence |
|---|---|---|
| `rng-versioned-replay` | **PASS** | Known-answer ChaCha8 vectors, full entity generation divergence, purpose namespace isolation (`purpose_isolation_and_crn_coupling`), and CRN coupling verified in unit tests. |
| `rng-checkpoint-compatibility` | **PASS** | `checkpoint_roundtrip_and_mid_buffer_restoration` passes; partial-buffer draws and nonzero stream positions yield identical subsequent draws upon restore; malformed generator and version reject. |
| `rng-dependency-cost` | **PASS** | Dual MIT/Apache-2.0 licenses; 0 vulnerabilities found via `cargo audit`; `cargo deny check advisories sources licenses bans` passed. MSRV Rust 1.99.1 verified. |

---

## 4. Validation Receipts & Execution Ledger

All validation runs were executed on working tree `/Volumes/PortableSSD/github/kairos`.

| Command | Working Directory | Toolchain / Environment | Exit Code | Artifact / Output Summary |
|---|---|---|---|---|
| `cargo test -p kairo-ecs-rng` | `/Volumes/PortableSSD/github/kairos` | `rustc 1.99.1 (stable-aarch64-apple-darwin)` | `0` | 14 tests in `kairo-ecs-rng` passed (0 failed, 0 filtered out) |
| `cargo test -p kairo-ecs-core --test conformance_fixtures` | `/Volumes/PortableSSD/github/kairos` | `rustc 1.99.1 (stable-aarch64-apple-darwin)` | `0` | 4 conformance fixture integration tests passed |
| `cargo test --workspace` | `/Volumes/PortableSSD/github/kairos` | `rustc 1.99.1 (stable-aarch64-apple-darwin)` | `0` | All workspace unittests and integration tests passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `/Volumes/PortableSSD/github/kairos` | `rustc 1.99.1 / clippy 1.99.1` | `0` | Finished `dev` profile; 0 warnings, 0 errors |
| `cargo fmt --all --check` | `/Volumes/PortableSSD/github/kairos` | `rustfmt 1.99.1` | `0` | All files formatted cleanly; 0 diffs |
| `cargo deny check advisories sources licenses bans` | `/Volumes/PortableSSD/github/kairos` | `cargo-deny 0.16.x` | `0` | Advisories ok, bans ok, licenses ok, sources ok |
| `cargo audit` | `/Volumes/PortableSSD/github/kairos` | `cargo-audit 0.21.x` | `0` | 0 vulnerabilities found across 374 crate dependencies |
| `node tests/conformance/conformance-check.mjs` | `/Volumes/PortableSSD/github/kairos` | `Node.js v26.12.1` | `0` | All 7 ready fixtures pass including `rng_reproducibility_v1` |
| `pwsh -NoProfile -File scripts/validate_cross_binding_conformance.ps1` | `/Volumes/PortableSSD/github/kairos` | PowerShell 7.5.x | `0` | Cross-binding conformance: ALL PASSED |
| `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | `/Volumes/PortableSSD/github/kairos` | PowerShell 7.5.x | `0` | 0 error(s), 0 warning(s); Phase gates validation passed |
| `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | `/Volumes/PortableSSD/github/kairos` | PowerShell 7.5.x | `0` | 106 tracks parsed; 0 errors, 0 warnings; DAG validation passed |
| `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` | `/Volumes/PortableSSD/github/kairos` | PowerShell 7.5.x | `0` | 106 track directories checked; 0 error(s), 0 warning(s); Artifact validation passed |
| `node scripts/validation/validate-track13-metadata.mjs` | `/Volumes/PortableSSD/github/kairos` | Node.js v26.12.1 | `0` | `{"status": "ok", "checked_tracks": 106}` |

### Skipped or Unavailable Checks
- None skipped. All required gate oracles and Conductor validators executed directly and passed with zero errors.

---

## 5. Phase Disposition

- Phase 0: **ACCEPTED** (ADOPT decision)
- Phase 1: **ACCEPTED** (Pinned ChaCha8 generator with stream isolation and checkpointing)
- Phase 2: **ACCEPTED** (All unit, workspace, deny, clippy, conformance, and Conductor gates passing)
- Track 71 is complete and ready for PR and merge into `main`. Track 72 (`rand_distr statistical sampling adoption`) will proceed next in sequence.
