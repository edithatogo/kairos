# Phase 0 Qualification: Track 78 — Rayon Bounded Independent-Work Execution

Date: 2026-10-24  
Lead Agent: experiment-agent  
Adoption Decision: **ADOPT**

---

## 1. Candidate Evaluation & Justification

- **Library Candidate:** `rayon = "1.12"` (pinned in `Cargo.lock` at `1.12.0`).
- **Features Qualified:** default features (`rayon-core` with bounded work-stealing thread pool).
- **Problem Solved:**
  - In `crates/kairo-ecs-cli/src/ensemble.rs`, parameter sweeps (`ParallelEnsembleRunner`), Sobol global sensitivity analysis (`SobolSensitivityAnalyzer`), and ABC-SMC calibration (`AbcSmcCalibrator`) currently spawn and join new OS threads on every single call using `std::thread::scope`.
  - For Sobol analysis across $d$ dimensions, `std::thread::scope` is entered and torn down $d + 1$ times in a single analysis run, causing heavy OS thread lifecycle churn.
  - Manual chunking via `div_ceil` causes severe tail latency when workload execution time varies across parameter points (skewed tasks).
  - Rayon replaces this ad-hoc machinery with a bounded, reusable work-stealing `ThreadPool`, eliminating thread startup churn and balancing skewed workloads.
- **Scope & Boundaries:**
  - Rayon is used strictly for independent embarrassingly parallel workloads (Monte Carlo ensemble sweeps, Latin Hypercube parameter evaluations, particle filter evaluations).
  - Rayon is NOT used for discrete-event simulation (DES) scheduling or event queue sequencing.
  - Reductions remain sequentially ordered or indexed to guarantee bit-for-bit replay determinism across arbitrary thread counts (1, 2, 4, 8 workers).
  - Synthetic vector-env code in `crates/kairo-ecs-ml/src/lib.rs` is acknowledged as a scaffold and does not inflate deletion claims.
- **Compatibility & MSRV:** Compiles cleanly under Rust 1.99.1 with zero warnings.
- **Security & License Audit:** Dual MIT/Apache-2.0 license; 0 vulnerabilities found by `cargo audit` and `cargo deny`.

---

## 2. Value Policy Assessment

| Criterion | Evaluation |
|---|---|
| Value Policy Compliance | Full compliance with `conductor/dependency-value-policy.md`. |
| Replay Determinism | Bit-for-bit reproducible results across 1, 2, 4, and 8 worker threads verified via indexed output placement and fixed-order reductions. |
| Elimination of Hand-Rolled Thread Machinery | Eliminates 4 separate `std::thread::scope` manual chunking and thread spawning routines in `crates/kairo-ecs-cli/src/ensemble.rs`. |
| Workload Skew & Work Stealing | Rayon work stealing eliminates head-of-line blocking on heterogeneous parameter evaluation latencies. |
| Bounded Resource Footprint | Uses caller-bounded thread pools (`rayon::ThreadPoolBuilder`) to ensure process CPU budgets are strictly respected and avoid unbounded thread growth. |

---

## 3. Qualification Receipts Table

| Check | Tool / Version | Exit Status | Result |
|---|---|---|---|
| Cargo Build | `cargo 1.99.1` | 0 | Compiles cleanly with `rayon 1.12` |
| Cargo Test | `cargo 1.99.1` | 0 | CLI and ensemble test suites pass |
| Cargo Deny | `cargo-deny 0.16.x` | 0 | Advisories, bans, licenses, sources OK |
| Cargo Audit | `cargo-audit 0.21.x` | 0 | 0 vulnerabilities found |
| Conformance | `node 26.x`, `pwsh 7.5` | 0 | Phase gates, DAG, and conformance pass |
