# Ensembles, Sobol sampling and calibration

The [Rust ensemble module](../../crates/kairo-ecs-cli/src/ensemble.rs) contains
experimental sampling, parallel evaluation and particle-scoring APIs. Their
scientific qualification is recorded separately in the [capability register](../../conductor/capabilities.md).

## Current API scope

| API | Implemented scope and limits |
| --- | --- |
| `ParallelEnsembleRunner::run_sweep_with_pool` | Evaluates supplied parameter points using an existing Rayon pool. This is independent ensemble execution, not intra-model PDES scaling. |
| `LatinHypercubeSampler`, `CommonRandomNumbersPlan` | Seeded stratification and per-entity seed helpers; distributional uniformity and policy-pair coupling still require qualification. |
| `SobolSequence::next_point` | Legacy multidimensional vectors are nonstandard; index 2^32 exceeds the direction table. Do not use this as a qualified Sobol generator. |
| `SobolSensitivityAnalyzer` | Computes first-order and total-effect estimates. Its `ci_lower_first`/`ci_upper_first` fields are fixed 0.9/1.1 multiples, not statistical confidence intervals. |
| `AbcSmcCalibrator::evaluate_particles` and parallel variants | Scores a supplied batch against a reference series and normalizes distance-based weights. Sequential populations, resampling and proposal corrections are absent; these outputs are not a validated ABC-SMC posterior. |

## Evidence required before scientific use

- S05: compare versioned direction numbers with independent SciPy/Joe–Kuo vectors, test bounds, and recover analytical Ishigami/Sobol-G sensitivity indices. Sequence parity alone does not validate an estimator.
- S06: test sampling distributions, repeated-seed interval coverage and CRN estimator bias/variance under scenario branching; preserve seed/algorithm versions.
- S07: define the calibration model, discrepancy, priors, proposal weights and tolerance schedule; validate posterior calibration and held-out predictions against analytical or reference posteriors.
- S08: qualify survival and competing-risk models independently; parallel ensemble agreement does not establish hazard correctness.

These are open [completion items S05–S08](../../conductor/scientific-completion.md),
with implementation in the [Track 110 plan](../../conductor/tracks/110-scientific-method-completion/plan.md).
Use the [dataset/oracle catalogue](../../conductor/validation-datasets.md) and
[performance programme](../../conductor/performance-programme.md) to separate
accuracy, uncertainty and runtime evidence. Existing deterministic tests do not establish empirical model validity.
