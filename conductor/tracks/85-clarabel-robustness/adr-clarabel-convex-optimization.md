# ADR: Clarabel Convex Optimization & Game-Theoretic Problem Formulation

**Track:** 85 (`85-clarabel-robustness`)  
**Status:** Approved  
**Date:** 2026-10-25  

---

## 1. Context & Architecture Decision

`crates/kairo-ecs-game-theory` requires rigorous mathematical optimization for agent-based decision modeling, game-theoretic inspection games, and correlated multi-agent equilibria.

### Key Decisions:
1. **Conic Interior-Point Solver:** Adopt `clarabel = { version = "0.11.1", features = ["serde"] }` using pure-Rust direct linear algebra (zero ambient BLAS/LAPACK/C compiler dependencies).
2. **Zero-Sum Matrix Game Minimax Duality:**
   Formulate general $M \times N$ matrix games as a canonical linear program:
   $$\min_{x, v} -v \quad \text{s.t.} \quad \sum x_i = 1, \quad v - \sum_i x_i A_{ij} \le 0, \quad x_i \ge 0$$
   Extract Player 1's optimal mixed strategy $x^*$ from primal variables, game value $v^*$, and Player 2's optimal mixed strategy $y^*$ from dual multipliers on the payoff inequality constraints.
3. **Correlated Equilibrium Linear Program:**
   Formulate correlated equilibrium distributions $p \in \Delta^{M \times N}$ maximizing total social welfare subject to Player 1 and Player 2 obedience incentive constraints:
   $$\sum_{j} p(i, j) [u_1(i, j) - u_1(i', j)] \ge 0 \quad \forall i, i'$$
   $$\sum_{i} p(i, j) [u_2(i, j) - u_2(i, j')] \ge 0 \quad \forall j, j'$$
4. **Boundary Honesty:**
   No claim that Clarabel implements general non-convex bimatrix Nash equilibrium, Lemke-Howson complementary pivoting, or extensive-form CFR. Those remain distinct algorithms.

---

## 2. Threat Modeling & Failure Recovery

| Threat / Failure Mode | Mechanism | Mitigation in Track 85 |
|---|---|---|
| **Degenerate / Jagged Payoffs** | Irregular row dimensions or empty matrices | Strict shape validation returning `Err(NormalFormError::DegeneratePayoffs)` or `EmptyPayoffMatrix`. |
| **Non-finite Values** | Payoffs contain NaN or Inf | Validated prior to solver allocation, returning `Err(NormalFormError::NonFiniteUtility)`. |
| **Infeasible Problem / Solver Stall** | Pathological constraints cause numerical divergence | Explicit `solver.solution.status` check returning `Err(NormalFormError::ConvexSolverError)` on non-solved outcomes. |
| **Residual Drift** | Numerical rounding exceeds physical tolerance | Independent `verify_minimax_residuals` and `verify_correlated_equilibrium_incentives` verify primal/dual residuals. |
