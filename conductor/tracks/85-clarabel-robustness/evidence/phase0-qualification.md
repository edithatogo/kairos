# Phase 0 Qualification: Clarabel Convex Solver

**Track:** 85 (`85-clarabel-robustness`)  
**Date:** 2026-10-25  
**Owner:** `vv-uq-agent`  
**Status:** Accepted (ADOPT)

---

## 1. Candidate Library Qualification

- **Crate:** `clarabel`
- **Version Qualified:** `0.11.1`
- **Repository:** `https://github.com/oxfordcontrol/Clarabel.rs`
- **License:** Apache-2.0 (Permitted under workspace license policy)
- **MSRV:** Rust 1.70.0 (Fully compatible with Kairos MSRV 1.99.1)
- **Selected Features:**
  - `serde` (required for build compilation; includes `serde-big-array`)
- **Excluded Features:**
  - `blas`, `lapack`, `intel-mkl`, `openblas` (zero system C library or ambient host toolchain dependencies; uses pure-Rust direct linear solvers QDLDL/AMD).

---

## 2. Supply Chain & Security Audits

- **`cargo deny check advisories sources licenses bans`**:
  - Result: **PASSED** (0 bans, 0 license violations, 0 source mismatches).
- **`cargo audit`**:
  - Result: **PASSED** (0 security vulnerabilities across 446 crate dependencies).

---

## 3. Mandatory Value Gate Assessment

1. **Replaced Production Code:**
   - Pre-existing game theory solver code only supported small 2x2 analytical indifference formulas (`MixedNashSolver`) and trivial fallbacks.
   - Clarabel provides general-purpose interior-point convex conic and linear programming optimization.
2. **Authoritative Boundaries Preserved:**
   - Clarabel is adopted for independently formulated LP/QP problems:
     - Arbitrary $M \times N$ two-player zero-sum matrix game minimax equilibria via linear programming duality.
     - General two-player correlated equilibria (CE) maximizing social welfare via obedience linear inequality constraints.
   - Retains existing analytical solvers and formulation models.
   - Explicitly rejects any false claim that Clarabel implements non-convex general bimatrix Nash, Lemke-Howson, or Counterfactual Regret (CFR).
3. **Adoption Decision:** **ADOPT**.
