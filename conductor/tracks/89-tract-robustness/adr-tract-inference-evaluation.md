# ADR: Evaluation and Rejection of `tract` Neural Network Inference Dependency

**Status:** Decided — **REJECT (NO-GO)**  
**Date:** 2026-10-25  
**Track:** 89-tract-robustness  
**Deciders:** `ml-integration-agent`, Principal Architecture Reviewer  

---

## Context

Track 89 was chartered to evaluate Sonos `tract` (`tract-onnx = "0.23.9"`) as a candidate inference engine for executing ONNX models within Kairos's agent-based simulation loop (`crates/kairo-ecs-ml`).

The repository's [dependency-value-policy.md](../../dependency-value-policy.md) mandates:
1. An active accepted production consumer and real model requirement.
2. Code-reduction or maintainability improvements, rather than speculative capability addition.
3. Strict enforcement of Stop Criteria defined in [spec.md](./spec.md).

## Decision

We **REJECT** the adoption of `tract` and retain the native pure-Rust `#![forbid(unsafe_code)]` architecture in `crates/kairo-ecs-ml`.

## Rationale

1. **Stop Criterion Triggered:** Track 89 [spec.md](./spec.md) defines an explicit stop condition:
   *"Stop criterion: No adoption without a required model or if supported operators/targets fail. Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity."*
   There are zero `.onnx` models and zero active ML inference consumers in Kairos or CareOps-Sim.
2. **Dependency Bloat Without Justified Consumer:** Adding `tract-onnx` pulls in over 50 transitive crates, increasing compile time, binary size, and dependency review burden without a production workload to evaluate.
3. **Safety & Architecture Invariant Preservation:** `crates/kairo-ecs-ml` strictly enforces `#![forbid(unsafe_code)]` and provides complete tensor validation, shape verification, tick hooks, and robust fallback policies.
4. **Value Policy Rule:** The value policy explicitly instructs: *"Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity."* and *"Do not install a dependency merely to close a track."*

## Requalification Triggers

This decision may only be revisited if:
1. An explicit clinical agent model (e.g. neural surrogate for patient physiological state or clinical triage) is committed with an actual `.onnx` model artifact and verified accuracy tolerances.
2. A dedicated evaluation confirms `tract` executes that specific model's operator graph without performance regressions.
