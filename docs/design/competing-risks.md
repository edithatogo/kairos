# Stochastic Competing Risks & Non-Markovian Hazard Engine

Track 69 provides native discrete-event simulation (DES) and random number generation (RNG) support for multi-state competing risks and non-Markovian survival clocks (`crates/kairo-ecs-rng::competing_risks` and `crates/kairo-ecs-des::hazard_clock`).

## Core Concepts

1. **Competing Risks Formulation**:
   - Multiple causes with cause-specific hazard functions $h_k(t)$ compete for event occurrence.
   - Overall cumulative hazard $H(t) = \sum_k H_k(t)$.
   - Each cause samples an exponential standard threshold $W_k \sim \text{Exp}(1)$ from an isolated pseudo-random number stream derived specifically for that entity or process (`derive_entity_seed`).
   - The first cause to satisfy $H_k(t_k) - H_k(t_{\text{start}}) = W_k$ realizes the event, with deterministic tie-breaking (`LowestCauseId`, `HighestCauseId`, or `ExplicitPriority`).

2. **Parametric & Non-Markovian Hazard Families**:
   - `Constant { rate }`: Exponential distribution (Markovian, memoryless).
   - `Weibull { shape, scale }`: Non-Markovian age-dependent hazard $h(t) = (k/\lambda)(t/\lambda)^{k-1}$.
   - `Gompertz { eta, b }`: Non-Markovian exponential hazard acceleration $h(t) = \eta \exp(b t)$.
   - `PiecewiseConstant { intervals, trailing_rate }`: Time-varying hazard rates across arbitrary continuous duration intervals.
   - `Zero`: Degenerate zero hazard for absorbing or latent states.

3. **Covariate Updates & Retained Residual Hazard**:
   - When environmental covariates or resource availability changes, active survival clocks support `HazardInterruptionPolicy::RetainResidual` or `Restart`.
   - Under `RetainResidual`, accumulated integrated hazard $H_k(t_{\text{elapsed}})$ is preserved, and only the remaining threshold $W_{\text{rem}} = W_k - H_k(t_{\text{elapsed}})$ is solved under the new hazard rate without redrawing unrelated random numbers.
   - Stale scheduled events are deterministically cancelled from the discrete-event `Scheduler`.

4. **Right Censoring as Observation Boundary**:
   - Right censoring at $T_{\text{censor}}$ does not corrupt the latent stochastic process: if the earliest event time $T > T_{\text{censor}}$, a censored observation event is emitted with `is_censored: true` and `winning_cause = CauseId(u32::MAX)`.
   - Latent unobserved future draws remain completely isolated from downstream agents and decision policies.

5. **Interruption & Resumption (Pause/Resume)**:
   - During transport holds or service preemption, `SurvivalClock::pause` freezes cumulative hazard progress.
   - Upon `SurvivalClock::resume`, the remaining hazard threshold resumes without burning progress during frozen intervals.
