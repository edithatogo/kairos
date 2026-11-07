# Equilibrium and CFR Solvers

The Rust [normal-form](../../crates/kairo-ecs-game-theory/src/normal_form.rs) and
[extensive-form](../../crates/kairo-ecs-game-theory/src/extensive_form.rs) (`graph-relations` feature) modules
contain operations with different maturity. Use the [capability register](../../conductor/capabilities.md)
to distinguish implemented operations from qualified mathematical guarantees.

## Solver scope

| Surface | Current implementation and limit |
| --- | --- |
| `BestResponseSolver`, `PureNashSolver`, `StrictDominanceSolver` | Enumerate best responses, pure equilibria and pure-strategy strict dominance for the supplied finite matrix. See the [normal-form guide](normal-form-runtime.md). |
| `MixedNashSolver`, `LemkeHowsonSolver` | A 2×2 indifference calculation with clamping, not general support enumeration or complementary pivoting. Dominance and degenerate cases require correction. |
| `CorrelatedEquilibriumSolver::verify_and_solve` | Forms a product from the preceding mixed-strategy routine; it does not independently verify incentive constraints. |
| `ClarabelConvexSolver` | Provides separate zero-sum minimax and social-welfare correlated-equilibrium optimization paths. Solver success must be checked alongside independent feasibility, incentive and objective residuals. |
| `ReplicatorDynamics::step_continuous_rk4` | Performs an Euler-style update, clipping and normalization; the name does not establish fourth-order integration. |
| `CfrSolver::solve_kuhn_poker` | A Kuhn-specific routine with CFR variants; it does not accept arbitrary information-set graphs or establish Leduc/general-game support. |

## Independent validation

- S01: demonstrate analytical replicator trajectories and step-refinement order, conservation and discontinuity handling before advertising RK4.
- S02–S03: compare dominance, boundary, degenerate and rectangular games with independent Gambit/OpenSpiel solutions; check simplex constraints, best-response gains and CE incentive residuals.
- S04: qualify graph-based CFR/CFR+ on Kuhn and Leduc using independent policy value/exploitability, convergence and perfect-recall checks.

These obligations remain open in [S01–S04](../../conductor/scientific-completion.md)
and the [Track 110 plan](../../conductor/tracks/110-scientific-method-completion/plan.md).
Reference tools are external test oracles; production simulation remains Rust.
Certification manifests and validator self-tests are evidence bookkeeping,
not execution or independent mathematical proof of generalized game solvers.
