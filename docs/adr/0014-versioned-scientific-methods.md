# ADR 0014 — Versioned scientific methods

## Decision

Use Joe–Kuo D(6) directions for the checked 32-bit, 16-dimensional Sobol stream. Version the original multiplier sequence for replay. Estimate Jansen effects using independent paired Monte Carlo samples and percentile bootstrap intervals for first-order and total effects.

Use weighted sequential ABC for a bounded uniform prior, a decreasing tolerance schedule and diagonal Gaussian perturbations. Keep the fixed-batch scoring API under its own name. Pair stochastic scenarios by replicate, generational entity and purpose. Survival clocks keep active age and first crossing time through interruptions and hazard replacement.

The optional Rust affine surrogate evaluates coefficients within a calibrated domain. `OrtSession::run` reports that the backend is unavailable. The queue model owns its state and observations in Rust. An opt-in CLI command drives the experimental Gymnasium and PettingZoo adapters over bounded local stdio on Linux and macOS.

## Compatibility

Callers of `SobolSequence::new` and `next_point` handle checked results. Use `LegacySobolSequence` for the previous points. Sensitivity callers handle undefined variance and read the new total-effect intervals. Fixed-batch ABC callers use `BatchDistanceScorer`. Rust queue callers pass real worlds and use checked step and reset operations. The host accepts an explicitly selected feature-enabled binary and uses the `queue-env-v1` contract. The stable C ABI remains at version 1.

Coefficient or seed compatibility depends on the reported algorithm version. The scientific register lists the supported inputs and independent conformance cases. [Scientific register](../../conductor/scientific-completion.md)

## Review concerns

The sensitivity bootstrap keeps A, B and their hybrid rows together and reports approximate marginal uncertainty. ABC exposes proposal exhaustion and low effective sample size. The queue transport rejects malformed frames and reaps the child on incomplete replies. Only run an executable selected from a trusted build. The installed Python adapters require POSIX process pipes. [Queue adapter contract](../ml/vectorized-vecenv-and-marl.md)
