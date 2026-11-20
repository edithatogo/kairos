# Equilibrium and CFR Solvers

The Rust [normal-form](../../crates/kairo-ecs-game-theory/src/normal_form.rs) and
[extensive-form](../../crates/kairo-ecs-game-theory/src/extensive_form.rs) (`graph-relations` feature) modules
contain operations with different maturity. Use the [capability register](../../conductor/capabilities.md)
to distinguish implemented operations from qualified mathematical guarantees.

## Solver scope

| Surface | Current implementation and limit |
| --- | --- |
| `BestResponseSolver`, `PureNashSolver`, `StrictDominanceSolver` | Enumerate best responses, pure equilibria and pure-strategy strict dominance for the supplied finite matrix. See the [normal-form guide](normal-form-runtime.md). |
| Mixed Nash | Paignia owns bounded support search and candidate admission. Kairos game-theory 0.5.0 withdraws its legacy mixed-strategy API. |
| Correlated equilibrium | Paignia owns the bounded two-player program and independent primal and dual admission. Kairos game-theory 0.4.0 retires both former CE paths. |
| `ClarabelConvexSolver` | Retains experimental zero-sum minimax and its separate equilibrium and residual checks. |
| `ReplicatorDynamics::step_discrete` and `run_trajectory` | Experimental references. The trajectory uses the historical clipped and normalized Euler update with fixed fitness by strategy index. Game-theory 0.3.0 retires the old public RK4 symbol. |
| CFR and CFR+ | Paignia owns training on its declared perfect-recall information-set graphs. Kairos game-theory 0.2.0 retires the Kuhn-specific public solver and retains its historical regression as test-only source. |

## Replicator migration

Kairos game-theory 0.3.0 withdraws `ReplicatorDynamics::step_continuous_rk4`.
Its existing calculation remains private for the experimental trajectory and
historical controls. The maintained caller inventory found only that trajectory
within the reviewed sources. Existing Git pins still reproduce their
historical outputs.

For supported continuous replicator work, use Paignia's
`replicator_euler_step` or `replicator_rk4_step` with the selected model
and the declared problem and step settings. Requalify trajectories against
that contract when changing methods. [Paignia S01 validation](https://github.com/edithatogo/paignia/blob/b8f21da9088fd3d6071121e165ff10973bec527e/docs/validation/replicator-rk4.md)

## Mixed-Nash migration

Kairos game-theory 0.5.0 withdraws `MixedStrategyProfile`, `MixedNashSolver`,
`LemkeHowsonSolver`, `nash_conv`, and `exploitability`. Existing Git pins retain
the historical matching-pennies control and outputs.

Use Paignia's `solve_mixed_nash` with complete row and column payoff matrices.
Inspect the typed status, supported game class and search scope before using
any witness. To check a specific profile, call `evaluate_mixed_nash_candidate`
and read its individual deviation gains and `sum_gain`. The previous average
exploitability convention is half of `sum_gain` for an admitted candidate.
Paignia explicitly reports Lemke–Howson as unsupported.

[S02 migration contract](../../conductor/contracts/game-extraction-map.md) ·
[Paignia mixed-Nash validation](https://github.com/edithatogo/paignia/blob/f363954fe17e2648942d1ac944280b485f865757/docs/validation/mixed-nash.md)

## Correlated-equilibrium migration

Kairos game-theory 0.4.0 withdraws `CorrelatedEquilibriumSolver`, the
`CorrelatedEquilibriumResult` type, and the CE solve and incentive methods on
`ClarabelConvexSolver`. Existing Git pins preserve the former API and historical
outputs. The bounded caller inventory found only the legacy CE oracle tests.

Enable `paignia-solve`'s `correlated-equilibrium` feature. Supply complete row
and column payoff tables in the same action order and an explicit objective to
`solve_correlated_equilibrium`. To reproduce the social-welfare objective, use
the cellwise sum of the two payoff tables. Inspect the returned status and the
admitted joint candidate before using its raw weights. Use
`evaluate_correlated_equilibrium_candidate` for primal diagnostics and
`evaluate_correlated_equilibrium_certificate` to evaluate a primal and dual
certificate. These APIs require one to eight actions per player. They preserve
the original payoff units and report approximate admission.

[S03 migration contract](../../conductor/contracts/game-extraction-map.md) ·
[Paignia CE validation](https://github.com/edithatogo/paignia/blob/b8f21da9088fd3d6071121e165ff10973bec527e/docs/validation/correlated-equilibrium.md) ·
[Migrated historical oracles](https://github.com/edithatogo/paignia/pull/64)

## CFR migration

For vanilla CFR, replace the legacy `CfrVariant::Vanilla` with
`paignia_solve::CfrVariant::Cfr`. Paignia also exposes `CfrVariant::CfrPlus`.
Call `paignia_solve::train_counterfactual_regret` with the game specification,
information-set declaration, and a `CfrConfig` with an iteration count in
`1..=100_000`. Linear CFR and zero-budget training are unsupported. The
versioned result includes a root value for each player and the average strategy
with each information set’s declared legal-action order. Choose a player's
value and any action probability from the model's own identifiers. The old
Kuhn aggregate bet probability has no generic result field.

Existing Git pins retain their historical outputs. The withdrawn surface had no
maintained runtime callers in the reviewed application snapshots. The
feature-enabled compile-fail fixture verifies that the public Kairos symbols
are unavailable, and the test-only fixture retains the historical Kuhn controls.
[Paignia CFR and independent controls](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/validation/cfr.md) ·
[Source and caller disposition](../../conductor/contracts/game-extraction-map.md)

## Independent validation

- S01: demonstrate analytical replicator trajectories and step-refinement order, conservation and discontinuity handling before advertising RK4.
- S02–S03: compare dominance, boundary, degenerate and rectangular games with independent Gambit/OpenSpiel solutions; check simplex constraints, best-response gains and CE incentive residuals.
- S04: qualify graph-based CFR/CFR+ on Kuhn and Leduc using independent policy value/exploitability, convergence and perfect-recall checks.

Track 110 retains the independent method, source and caller obligations.
[Scientific programme](../../conductor/scientific-completion.md) ·
[Track 110 plan](../../conductor/tracks/110-scientific-method-completion/plan.md)
Reference tools are external test oracles; production simulation remains Rust.
Certification manifests and validator self-tests are evidence bookkeeping,
not execution or independent mathematical proof of generalized game solvers.
