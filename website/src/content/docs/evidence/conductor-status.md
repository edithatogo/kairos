---
title: Conductor Status
description: Source-linked capability, validation and release status without stale completion counters.
---

Conductor records work ownership, acceptance criteria and retained evidence.
Its [track registry](https://github.com/edithatogo/kairos/blob/main/conductor/tracks.yaml)
and [narrative status](https://github.com/edithatogo/kairos/blob/main/conductor/status.md)
are the current navigation points. This page does not duplicate a static track
count that can drift from the registry.

## Read completion in context

A `Done` track means its recorded scope and acceptance were completed. It does
not automatically establish native engine execution in every language, scientific
correctness, hardware scaling or release readiness. Earlier preview work may
have completed DTOs, contract tests or local fixtures while broader capabilities
remain incomplete.

The [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json)
and [current-state review](https://github.com/edithatogo/kairos/blob/main/docs/current-state.md)
separate:

- implemented source and API scope;
- actual native execution and installed-consumer acceptance;
- analytical, differential and empirical validation;
- measured performance and qualified hardware;
- beta/stable release acceptance.

## Work toward release

| Programme area | Acceptance focus |
| --- | --- |
| Rust-only runtime | One core; missing native capabilities fail explicitly; every supported adapter executes the actual engine. |
| Scientific-method completion | Independent oracles, correct method identities, numerical convergence and documented limits. |
| Measured performance | Real workload timings, profiles, fair baselines and reproducible artefacts. |
| Evidence automation | Claims tied to exact source, input and artefact hashes; synthetic fixtures remain labelled. |
| Beta and stable release | A coherent, supported subset with native consumer, compatibility, security and recovery gates. |
| Dependencies and consumers | Maintained implementations where qualified; applications consume Kairos through versioned contracts. |
| Frontier experiments | Preregistered questions and comparison methods before new superiority claims. |

Read the [delivery programme](https://github.com/edithatogo/kairos/blob/main/conductor/frontier-programme-20261107.md)
and [release gates](https://github.com/edithatogo/kairos/blob/main/conductor/release-gates.md).
Historical evidence remains useful at its recorded source revision; current
changes need their own relevant verification.
