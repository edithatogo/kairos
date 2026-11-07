# Verification, benchmark and validation catalogue

Source review: 7 November 2026. Owners: Tracks 110/111/112; domain evaluation stays with the consumer. These resources are proposed inputs, not new completed experiments.

## Separate questions

- **Verification:** does the implementation satisfy its algorithm/engine contract?
- **Benchmarking:** how much time, memory and communication does equivalent work require?
- **Empirical validation:** does a specified model reproduce held-out observations with useful uncertainty?
- **Decision evaluation:** does the model support the intended comparison under explicit assumptions?

Passing one category does not establish another. Production cores remain Rust; independent reference programs are test-only external oracles.

| Capability | Dataset / oracle | What to check | Access / interpretation |
|---|---|---|---|
| Scheduler, resources and DES | Generated exact M/M/1, M/M/c, finite queues; [SimPy bank reneging](https://simpy.readthedocs.io/en/latest/examples/bank_renege.html) | Exact adversarial event traces; conservation; warm-up and steady-state moments/interval coverage; cancellation/preemption/reneging | Share fixed input tapes between engines. Equal seeds across different RNGs do not imply equal inputs. Analytical/generated fixtures avoid personal data. |
| PDES, Time Warp and scaling | [ROSS PHOLD](https://ross-org.github.io/setup/running-sim.html) | Useful events/s, serial parity, remote-event fraction, lookahead, LP count, rollback cost, GVT and memory | Synthetic controlled workload; does not validate a hospital model. Pin source and source licence. |
| CPU ABM / spatial | [krABMaga models](https://github.com/krABMaga/krABMaga) plus small exhaustive local-rule cases | Matched flocking/Schelling/network transitions and aggregate distributions; geometry/routing degeneracies | Match actual rules, precision, update order and neighbours, not just model names. |
| GPU ABM | [FLAME GPU circles benchmark](https://github.com/FLAMEGPU/FLAMEGPU2-circles-benchmark) | Equal populations, density/radius, steps and precision; transfer/setup and steady-state time | Benchmark repository MIT; engine licence is separate. A particle result does not certify arbitrary DES acceleration. |
| Independent-world batching | [Madrona](https://madrona-engine.github.io/) | Matched complete environments, state layout, tensor export and simulation budgets | Distinguish parallel independent worlds from accelerating one coupled simulation. |
| Continuous/hybrid/FMI | [Modelica Reference FMUs](https://github.com/modelica/Reference-FMUs), [fmusim](https://github.com/modelica/fmusim) | Dahlquist error order; BouncingBall state events; Stair time events; Feedthrough types; VanDerPol dynamics; save/restore for supported capabilities | Reference-FMUs and fmusim are BSD-2-Clause at review. FMUs include native binaries: pin/check each build and supported FMI variant. DAE cases only when actually supported. |
| Game theory | [OpenSpiel](https://github.com/google-deepmind/open_spiel), [Gambit](https://www.gambit-project.org/) | Dominance, degeneracy, matching pennies/RPS, rectangular games, Kuhn/Leduc; simplex, payoffs, incentive residuals and exploitability | OpenSpiel Apache-2.0; Gambit GPL. Use external oracle execution and preserve input/output rights; do not copy a solver into the permissive core without licence assessment. |
| QMC / sensitivity | [SciPy Sobol](https://docs.scipy.org/doc/scipy/reference/generated/scipy.stats.qmc.Sobol.html), fixed direction-number vectors; Ishigami/Sobol-G | Multidimensional vectors, bounds, reproducibility, convergence/discrepancy and known sensitivity indices | Fix exact scrambling, dimension, precision and licence; include the existing nonstandard-vector and 2^32 boundary regressions. |
| Survival / competing risks | Generated constant, Weibull and piecewise hazards; [BMT example](https://scikit-survival.readthedocs.io/en/stable/api/generated/sksurv.datasets.load_bmt.html) | For constant causes, S(t)=exp(-sum(lambda)*t) and F_k(t)=lambda_k*(1-S(t))/sum(lambda); censoring/restart and estimator coverage | The small BMT example is an estimator fixture, not clinical generalization. Check original data terms before redistribution. |
| SBI / calibration | [sbibm](https://github.com/sbi-benchmark/sbibm) | Two-moons and applicable SIR/Lotka–Volterra references; posterior calibration, C2ST/reference discrepancy, predictive checks and simulation budget | MIT benchmark framework; pin task versions and note task corrections. Keep external Python/Julia oracles outside the production core. |
| ML execution | [ONNX reference semantics](https://onnx.ai/onnx/intro/concepts.html) and a chosen real model | Nonidentity input/output cases, shapes/operators, unsupported errors, independently computed output, then held-out model accuracy | Runtime/model/training-data licences differ. A tensor shape or identity output is not inference. |
| Empirical event sequences | [BPI2011 hospital log](https://research.tue.nl/en/datasets/real-life-event-logs-hospital-log/), DOI 10.4121/uuid:d9769f3d-0ab0-4fb8-803b-0d1120ffcf54 | Ingestion, sequence replay, duration fitting and held-out pathways | Confirm original licence before redistribution. No inference that this contains ED staffing/capacity ground truth. |
| Patient trajectories | [MIMIC-IV-ED 2.2](https://physionet.org/content/mimic-iv-ed/2.2/) and [MIMIC-IV timestamp rules](https://www.physionet.org/content/mimiciv/3.1/) | Within-patient durations, pathway variation and LOS with patient-level holdouts | Credentialed access/training/DUA. Subject-specific date shifts prevent reconstructing actual cross-patient arrivals/concurrent occupancy. Never publish credentialed rows or patient-derived fixtures in public GitHub/Hugging Face. |
| Synthetic interchange | [Synthea downloads](https://synthetichealth.github.io/downloads.html) | FHIR/CSV mapping, population volume, schemas and deterministic pipeline checks | Downloads permit unrestricted reuse at review. Synthetic records demonstrate interoperability, not realistic operational flow or clinical validity. |
| Australian aggregate plausibility | [AIHW ED statistics](https://www.aihw.gov.au/hospitals/topics/emergency-departments) | Presentation mix, waiting and LOS summary comparison | Check each table's licence/version. Aggregates do not recover individual pathways or identify policy causal effects. |

## First releasable public fixture pack

Prioritize analytical queue/event tapes, PHOLD variants, independently built Reference FMUs, small game matrices/trees, exact survival cases and versioned QMC vectors. Keep the small deterministic cases in the repository; large redistributable inputs and raw results can use versioned data artefacts. Record URLs, licences, source revisions and SHA-256 for every input. Reuse existing dataset registries where possible; creating a new repository is not required.

For restricted resources retain fetch/transform code, schemas and permissible aggregate metadata only. Access or data-use agreements are not granted by this plan. Domain owners obtain the lawful dataset and execute locally; the public engine receives no credentials.

## Acceptance design

Freeze model semantics, source versions, transformations, split strategy, tolerances and endpoints before evaluating. Use disjoint tuning and held-out scenarios; patient/hospital/time holdouts when those dimensions are valid. Include parameter recovery, identifiability limits, posterior/predictive calibration and uncertainty coverage as appropriate. Test deliberate incorrect algorithms/backends to ensure the oracle rejects them.

Store command, working directory, source and lock hashes, seed/input tapes, actual backend, machine/compiler/driver, timing scope, raw samples, exit status and reviewer disposition. Native execution, scientific validity, comparator equivalence and licence acceptance have separate fields. CI passes only the checks actually executed.

## Sources and refresh

The linked project owners/documentation are primary sources reviewed for this programme. Recheck licences, access conditions and pinned versions during ingestion. New datasets need a provenance card and independent oracle plan before they are added as acceptance evidence.

