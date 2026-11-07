---
title: 'KairoECS: A Rust-first multi-method simulation engine for DES and ABM'
tags:
  - simulation
  - discrete-event-simulation
  - agent-based-modeling
  - rust
  - apache-arrow
  - high-performance-computing
authors:
  - name: Dylan A Mordaunt
    affiliation: 1
  - name: KairoECS contributors
    affiliation: 1
affiliations:
  - name: KairoECS Project
    index: 1
date: 2026-05-05
bibliography: paper.bib
---

# Draft status and evidence boundary

This manuscript is a work in progress, updated on 7 November 2026. It describes
implemented source and bounded software qualification, not a completed
comparative performance study or independent empirical validation of application
models. The [current-state review](https://github.com/edithatogo/kairos/blob/main/docs/current-state.md)
and [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json)
identify known limitations and the evidence needed before stronger claims.

# Summary

KairoECS is an alpha Rust simulation engine combining discrete-event simulation
(DES) and agent-based modelling (ABM). Its design brings ordered event scheduling,
entity/component state, resource queues, spatial operations and versioned random
streams into a modular workspace. DES and ABM provide complementary ways to
represent systems in which individual behaviour interacts with shared queues and
capacity constraints [@zeigler2000theory; @north2013complex; @masad2015mesa].

The [v0.4.0-alpha.1 prerelease](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1)
was published on 18 October 2026. Later default-branch work remains experimental.
The repository, issue tracker and contribution workflow are hosted at
<https://github.com/edithatogo/kairos>. Release of a source snapshot does not imply
publication or native qualification of every language package.

# Statement of Need

Models of healthcare operations, logistics and population behaviour often need
both operational resources and individual decisions. Researchers also need to
record input data, seed policy, numerical assumptions and model version so that
results can be inspected and reproduced. A reusable simulation engine can reduce
the amount of scheduling, state and data-exchange code each application maintains.
It cannot replace domain-specific model design or validation against observations.

KairoECS explores a common Rust execution core with explicit language and data
boundaries. Host languages can prepare inputs, orchestrate experiments and analyse
outputs; supported simulation execution must call native Rust or Rust-compiled
WebAssembly. Independent host-language preview cores are unsupported under
[ADR 0010](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md).
This policy reduces duplicated engine semantics, but it does not make an
unimplemented adapter complete. Missing native capabilities must fail explicitly.

# State of the Field

Established tools address a broad range of simulation needs. SimPy provides
process-based DES in Python [@simpy2020]. Mesa supports Python ABM
[@masad2015mesa], NetLogo provides an agent-modelling environment
[@wilensky1999netlogo], and Repast Simphony provides infrastructure for complex
adaptive systems modelling [@north2013complex]. Parallel DES has a substantial
algorithmic literature, including conservative and optimistic synchronization
[@fujimoto1990parallel]. Apache Arrow provides standardized columnar data
interfaces [@arrow2021].

KairoECS should be evaluated as an additional implementation with different
engineering choices. Language choice alone does not establish speed, model
expressiveness, reproducibility or lower maintenance cost. This draft makes no
measured superiority claim over these frameworks. A useful comparison requires
equivalent model semantics, independently checked outputs, representative
workloads, exact versions and repeated raw timings. Existing libraries also offer
algorithms, interoperability tests and maintenance practices that can be reused
where their contracts and dependency costs fit.

# Software Design and Architecture

The scheduling kernel orders events by fixed-tick simulation time, priority and
insertion sequence. `SimTime` uses an unsigned 128-bit tick representation; the
model defines the unit. Generational entity and event identities support lifetime
checks. Entity/component storage and DES/ABM primitives provide reusable building
blocks for model authors. The core crate forbids unsafe Rust, while FFI and some
optional native integrations have separately reviewed unsafe boundaries. This is
not a workspace-wide claim that every integration contains no unsafe code.

The current RNG implementation uses versioned ChaCha8 streams and explicit
checkpoint/replay contracts. Reproduction depends on recording the RNG version
and model execution semantics, not only a seed. Exact discrete event order,
floating-point agreement within a declared tolerance and statistical agreement
are different contracts; this paper does not promise universal bit-for-bit
cross-platform model replay.

Optional data features use maintained Arrow/Polars implementations and expose
columnar interchange paths. Each consumer, format and ownership boundary needs
qualification. Converting host-language DTOs into an Arrow table does not prove
zero-copy transfer from the Rust engine. Functions and structures named for a
format must also be checked against their actual byte representation.

Conservative and optimistic PDES, MPI/gRPC transports, GPU kernels and FMI
integration extend the execution surface. Bounded native evidence exists for
some paths. A deployment template, a backend trait or a synthetic fixture is not
proof of real hardware execution, interoperability or scaling. Applications such
as `careops-sim` keep their domain models and policy outside Kairos and consume a
versioned engine dependency.

# Verification and Evaluation

At reviewed source `d56863505d938feb069af88a737eb7668113d14b`, the
[hosted Rust verification job](https://github.com/edithatogo/kairos/actions/runs/41658006983/job/126030245516)
reported 994 passing tests, 1,161/1,239 core lines covered (93.70%), and native MPI
smoke runs at two and four ranks. These results belong to that revision and job
matrix. Platform/feature exclusions still limit what executed. Coverage and test
counts are software-verification evidence, not mathematical proof or empirical
model validity.

A [retained conservative-PDES benchmark](https://github.com/edithatogo/kairos/blob/main/docs/pdes/benchmark-results.md)
measured five samples per configuration on an Apple M1 Max at source
`45f01c49be1d15cefb64ad48e59ee0a7e4146b3e`. Its small ring workloads were
substantially slower than the equivalent sequential Rust scheduler. Repeated
worker creation and synchronization are identified costs to investigate. Those
measurements establish execution and state parity within their workload; they
do not establish general parallel speedup or a ranking against other engines.
The [scaling-report correction](https://github.com/edithatogo/kairos/blob/main/docs/benchmarks/scaling-certification-report.md)
explicitly excludes generated placeholder efficiency and speedup figures from
performance evidence.

# Limitations and Completion Work

Scientific extensions have uneven maturity. The numerical audit identifies a
routine named RK4 that currently performs an Euler-style update, and the named
Lemke-Howson path is limited to a 2×2 calculation rather than a general pivoting
solver. Demonstration inference and vectorized environments do not establish
actual ONNX inference or simulation-backed reinforcement learning. These are
correctness and scope issues to fix or exclude from supported claims, not merely
documentation polish.

The [scientific completion programme](https://github.com/edithatogo/kairos/blob/main/conductor/tracks/110-scientific-method-completion/plan.md)
requires independent analytical and differential oracles, convergence studies,
edge cases and honest method naming. The
[dataset plan](https://github.com/edithatogo/kairos/blob/main/conductor/validation-datasets.md)
separates verification fixtures from empirical validation, including held-out
evaluation and uncertainty. Research applications in healthcare, economics and
digital twins remain intended uses until their models pass those checks.

Beta and stable promotion depend on a defined supported subset, installed-native
consumer acceptance, compatibility, measured performance, recovery, documentation
and supply-chain gates. Frontier experiments require explicit hypotheses and
comparison methods before claims of novelty or performance leadership.

# AI Usage Disclosure

This draft has been edited with AI assistance. Before submission, maintainers
must reconcile the disclosure with actual project history and journal policy,
verify citations and results, and take responsibility for the final manuscript.
No blanket claim is made that all algorithms, proofs or implementations have
already received independent human verification or validation.

# Acknowledgements

The authors acknowledge the open-source communities behind Rust, Apache Arrow
and the simulation literature and software on which this work builds.

# References
