# Phase 0 Qualification: Track 72 — rand_distr Statistical Sampling Adoption

Date: 2026-10-24  
Lead Agent: rng-agent + vv-uq-agent + des-api-agent  
Adoption Decision: **ADOPT**

---

## 1. Candidate Evaluation & Justification

- **Library Candidate:** `rand_distr = { version = "0.6", default-features = false, features = ["alloc"] }`.
- **Features Qualified:** `default-features = false, features = ["alloc"]`. Free of system entropy, OS calls, and threading hooks. Strictly compatible with `rand 0.10` and `rand_chacha 0.10`.
- **Problem Solved:**
  - Replaces bespoke, biased modulo sampling (`rng.next_u64() % span`) and manual clamping with mathematically sound, rejection-sampled `Uniform` distributions.
  - Replaces handwritten float inversion and manual exponential approximations with library-backed `rand_distr::Open01` and `rand_distr::Exp1` / `rand_distr::Exp`.
  - Exposes standard statistical distribution sampling directly on `DeterministicStream::sample<T, D: Distribution<T>>` for discrete-event and agent-based simulation pipelines.
- **Compatibility & MSRV:** Rust 1.99.1 verified. Compiles cleanly with zero warnings under MSRV.
- **Security & License Audit:** Verified with `cargo deny check advisories sources licenses bans` and `cargo audit`. Dual MIT/Apache-2.0 licenses; 0 vulnerabilities, 0 advisories.

---

## 2. Value Policy Assessment

| Criterion | Evaluation |
|---|---|
| Value Policy Compliance | Full compliance with `conductor/dependency-value-policy.md`. |
| Replay Determinism | Bit-for-bit reproducible across runs; mid-buffer checkpoint restoration retains bit-identical subsequent draws across distributions. |
| Elimination of Hand-Rolled Distribution Code | Replaced handcrafted 53-bit mantissa shift in `competing_risks.rs` with `rand_distr::Open01` and standard exponential threshold with `rand_distr::Exp1`. |
| Domain Hazard Retention | Retained non-Markovian survival integration/inversion (`HazardRate::invert_cumulative_hazard`, `cumulative_hazard`, `hazard_at`), tie-breaking, and censoring rules as commanded by spec. |
| Stream Isolation & Replay | Independent draws across entity handles and purpose namespaces preserved without interference. |

---

## 3. Qualification Receipts Table

| Check | Tool / Version | Exit Status | Result |
|---|---|---|---|
| Cargo Build | `cargo 1.99.1` | 0 | Compiles cleanly |
| Cargo Test | `cargo 1.99.1` | 0 | 31 unit & integration tests in `kairo-ecs-rng` pass |
| Cargo Deny | `cargo-deny 0.16.x` | 0 | Advisories, bans, licenses, sources OK |
| Cargo Audit | `cargo-audit 0.21.x` | 0 | 0 vulnerabilities found |
| Conformance | `node 26.x`, `pwsh 7.5` | 0 | Cross-binding and fixture validation pass |
