# Phase 0 Evidence & Value Gate Qualification: Tract Inference Qualification

**Track:** 89-tract-robustness  
**Date:** 2026-10-25  
**Lead:** `ml-integration-agent`  
**Status:** Phase 0 Complete — Evaluation Decision: **REJECT (NO-GO)**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `ml-integration-agent`
- **Owned Path:** `conductor/tracks/89-tract-robustness/`
- **Prospective Target:**
  Candidate replacement of machine learning model execution and inference scaffolding in `crates/kairo-ecs-ml/` with `tract-onnx` (`tract = "0.23.9"`).
- **Prerequisites Status:**
  - Track 78 (`Rayon data parallelism robustness`) completed and qualified.
- **Current Baseline Capabilities in `kairo-ecs-ml`:**
  1. **Strict Safety Invariant:**
     - `crates/kairo-ecs-ml` enforces `#![forbid(unsafe_code)]`.
  2. **Dependency-Free Mock Inference Scaffold:**
     - Provides `OrtSession`, `NeuralSystem`, `InferenceTickHook`, and `Tensor` abstractions.
     - Strict shape validation: dimension products match value buffer sizes.
     - Strict numerical validation: float buffers reject `NaN` and `+/-Infinity`.
     - Explicit tick execution lifecycle (`TickPhase::BeforeSystems`, `TickPhase::AfterSystems`).
     - Resilient fallback policies: `HoldLastOutput`, `UseOriginalSystem`, and `FailTick`.
     - Multi-world parallel rollout vector environments verified on Rayon threadpools (1024 worlds).
  3. **Zero Host Friction & Minimal Workspace Footprint:**
     - Compiles with zero external dependencies in milliseconds across Linux, macOS, and Windows.

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `tract-onnx` / `tract-core` (Sonos Tract)
- **Version:** `0.23.9`
- **Repository:** `https://github.com/sonos/tract`
- **License:** Apache-2.0 OR MIT
- **Architecture & Ecosystem Cost:**
  - Pure-Rust neural network inference engine supporting ONNX operator subgraphs.
  - Pulls over 50 transitive crates into the dependency graph (`tract-core`, `tract-hir`, `tract-linalg`, `tract-data`, `tract-pulse`, etc.).
  - Introduces substantial binary bloat and prolonged workspace compilation times.
- **Stop Criterion & Value Policy Conflict:**
  1. **Triggered Stop Criterion:**
     > *"Stop criterion: No adoption without a required model or if supported operators/targets fail. Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity."*
  2. **Absent Production Consumer & Model Artifact:**
     - Zero `.onnx` model files exist across Kairos and CareOps-Sim.
     - Neither repository has an active production ML model consumer or clinical inference pipeline requirement.
     - Adopting `tract` merely to replace a placeholder scaffold violates the core tenet: *"Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity."*
  3. **Adoption Policy:**
     > *"If there is no production target/accepted consumer, no net benefit, unsupported contracts or excessive migration burden, record no-go/deferred with evidence. Keep Phase 1/2 unchecked, adoption_decision=deferred/rejected, and do not install a dependency merely to close a track."*

---

## 3. Value Gate Evaluation (Task 0.3)

| Dimension | Existing Native (`kairo-ecs-ml`) | `tract-onnx 0.23.9` | Decision Impact |
| :--- | :--- | :--- | :--- |
| **Active Model Consumer** | None (scaffold only) | None | Stop criterion triggered (NO-GO) |
| **External Dependencies** | 0 direct dependencies | 50+ transitive dependencies | Significant bloat / negative net value |
| **Safety** | `#![forbid(unsafe_code)]` | Contains SIMD/raw pointer unsafe | Preserves `#![forbid(unsafe_code)]` |
| **Shape & Float Checks** | Fully implemented & verified | Handled by operator graph | Native checks already 100% covered |
| **Tick Hooks & Fallback** | Fully implemented & verified | Requires custom adapter wrapper | Retaining native requires zero wrappers |
| **Compilation Overhead** | < 1 second | Multi-minute graph build | Faster CI and developer iteration |

---

## 4. Evaluation Outcome & Disposition (Task 0.4)

- **Phase 0 Value Gate Decision:** **REJECT (NO-GO)**
- **Adoption Decision:** `rejected`
- **Disposition:**
  - Do NOT install `tract-onnx` / `tract`.
  - Retain the dependency-free `#![forbid(unsafe_code)]` `kairo-ecs-ml` implementation.
  - Leave Phase 1 (Bounded replacement) and Phase 2 (Acceptance & handoff) unselected, keeping this track cleanly closed at Phase 0 evaluation per policy.
