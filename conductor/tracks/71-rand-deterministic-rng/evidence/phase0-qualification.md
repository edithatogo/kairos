# Phase 0 Qualification: Track 71 — rand Deterministic RNG Adoption

Date: 2026-10-24  
Lead Agent: rng-agent + conformance-agent  
Adoption Decision: **ADOPT**

---

## 1. Candidate Evaluation & Justification

- **Library Candidate:** `rand = "0.10"`, `rand_chacha = "0.10"` (with `rand_core = "0.9"` / `0.6`).
- **Features Qualified:** `default-features = false`. Strictly deterministic; no OS entropy, no thread-local random, no non-deterministic device hooks in simulation runs.
- **Problem Solved:** Replaces bespoke handwritten SplitMix64 generator with cryptographically robust, peer-reviewed, reproducible ChaCha8 generator (`ChaCha8Rng`). Provides native stream isolation (`set_stream(u64)` / `get_stream() -> u64`) and word position tracking (`set_word_pos(u128)` / `get_word_pos() -> u128`) enabling exact mid-buffer stream state restoration.
- **Compatibility & MSRV:** Rust 1.99.1 verified. `rand 0.10` and `rand_chacha 0.10` compile cleanly with zero warnings under MSRV.
- **Security & License Audit:** Verified with `cargo deny check advisories sources licenses bans` and `cargo audit`. Dual MIT/Apache-2.0 licenses; 0 vulnerabilities, 0 advisories.

---

## 2. Value Policy Assessment

| Criterion | Evaluation |
|---|---|
| Value Policy Compliance | Full compliance with `conductor/dependency-value-policy.md`. |
| External Entropy | Zero OS/thread-local randomness used. All streams derived from explicit `run_seed`, `EntityId`, and purpose namespace. |
| Replay Determinism | Bit-for-bit reproducible across runs and architectures. |
| Stream Isolation | Unique independent streams derived via `derive_stream_id(entity, purpose)`. Common random number (CRN) coupling preserved across scenarios. |
| Mid-Buffer Checkpoint Restoration | `RngCheckpoint` captures `generator`, `version`, `seed`, `stream_id`, and `word_pos`. Exact mid-buffer restoration verified by unit tests. |

---

## 3. Qualification Receipts Table

| Check | Tool / Version | Exit Status | Result |
|---|---|---|---|
| Cargo Build | `cargo 1.99.1` | 0 | Compiles cleanly |
| Cargo Test | `cargo 1.99.1` | 0 | 14 tests in `kairo-ecs-rng` pass |
| Cargo Deny | `cargo-deny 0.16.x` | 0 | Advisories, bans, licenses, sources OK |
| Cargo Audit | `cargo-audit 0.21.x` | 0 | 0 vulnerabilities found |
| Conformance | `node 26.x`, `pwsh 7.5` | 0 | Cross-binding and fixture validation pass |
