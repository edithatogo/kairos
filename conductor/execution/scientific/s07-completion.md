# S07 Sequential ABC

`AbcSmcCalibrator` uses a rectangular uniform prior and a finite, strictly decreasing, prespecified tolerance schedule. Sample the first population from the prior. Each subsequent proposal resamples an ancestor by normalized weight and perturbs it using a diagonal Gaussian with scale proportional to the prior range. Reject proposals outside the prior or with discrepancy above the current tolerance. Normalize weights using the reciprocal of the weighted Gaussian mixture density. Report attempts and effective sample size for each generation and fail explicitly on a depleted proposal budget or insufficient effective sample size. [Beaumont et al. (2009)](https://arxiv.org/abs/0805.2256)

For two Bernoulli observations and a uniform prior on their success probability, observed successes are a sufficient statistic. For two observed successes, absolute discrepancy tolerances 2, 1 and 0 give posterior means 1/2, 5/8 and 3/4 and a final predictive probability of 3/5 for two future successes. Verify convergence across fixed seeds, compare later weights with a direct mixture calculation, and use simulated prior/observation pairs to check posterior rank calibration at zero tolerance. The old batch operation remains available as a distance scorer.

## Held-out count-model oracle

The held-out case changes the model, not just the seed: one Poisson count
of two with a uniform prior on rate in `[0, 1]`. At zero discrepancy the
posterior density is proportional to `rate² exp(-rate)`. Integrating directly,
the posterior mean is `(6 - 16/e) / (2 - 5/e)`, and the predictive probability
of zero for an independent future count is `(1 - 5/e²) / (4(2 - 5/e))`.

Use 384 particles, the existing 2/1/0 tolerance schedule, and twelve fixed
seeds `8100..8112`. Both mean absolute errors must be at most 0.025. Check
that all final particles match the observed count and that substituting the
unupdated prior would fail both bounds. This is a finite synthetic method
oracle, not an empirical model or downstream acceptance test.

On macOS ARM64 with Rust and Cargo 1.99.1, all five
`maturity_abc_sequential` tests passed. The held-out mean absolute errors
were 0.011176 (posterior mean) and 0.006029 (predictive probability).
Independent methods review confirmed the exact references, 0.709383 and
0.503297, and that the unupdated-prior control misses both fixed bounds.

The test was originally reviewed on `6e908df` and reran on `56c74b83` plus
the unchanged test. From the workspace root, `cargo test --locked -j 4 -p
kairo-ecs-cli --test maturity_abc_sequential -- --nocapture` exited zero.
The local rerun log and source hashes are in `target/reconcile-20261126/science`.
This is bounded method evidence, not acceptance of an application model.
