# Track 37 Plan: ML/AI Integration & Inference

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- [x] Review core ECS tick boundary contracts from Track 01 to identify safe injection points for ML inference.
- [x] Review GPU compute contracts from Track 32 to coordinate device memory, streams, and context lifetime.
- [x] Review Python binding contracts from Track 06 to understand PyO3 surface for Gymnasium wrapper.
- [x] Open an ADR if ML inference requires blocking the simulation loop or introduces non-deterministic execution.

### Task 0.2 — Define owned artifacts
- [x] Define `kairo-ecs-ml` crate boundaries: ONNX backend, TensorRT backend, Burn backend, inference scheduler.
- [x] Define `NeuralSystem` trait contract: input/output tensor shapes, tick hook life-cycle, fallback behavior.
- [x] Define `kairo_gym` Python package: observation space, action space, reward function, reset/step/close API.
- [x] Add owner/subagent to `conductor/subagents.md`.

## Phase 1 — Minimum viable ONNX inference

### Task 1.1 — Scaffold kairo-ecs-ml crate
- [x] Create `crates/kairo-ecs-ml/` with Cargo feature flags: `onnx`, `tensorrt`, `burn`, `gymnasium`.
- [x] Implement `OrtSession` wrapper: load from file/buffer, configure execution providers, run inference.
- [x] Implement `InferenceTickHook` that plugs into the ECS tick boundary and executes registered models.

### Task 1.2 — Proof-of-concept surrogate
- [x] Create `examples/ml-surrogate/de-surrogate/` demonstrating a neural network replacing a simple ODE solver.
- [x] Measure and document the speedup and accuracy trade-off.
- [x] Ensure the example builds and runs with `cargo run --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml --features onnx`.

### Task 1.3 — Add review criteria
- [x] Add red-team prompts for deadlocks under concurrent inference, tensor shape mismatches, and model staleness.
- [x] Add measurable acceptance criteria: inference latency < 1ms for models under 10M parameters on CPU; deadlock-free under 1000 consecutive ticks.

## Phase 2 — Gymnasium environment wrapper

### Task 2.1 — Implement kairo_gym Python package
- [x] Create `python/kairo_gym/` with `pyproject.toml`, `setup.cfg`, and Python source.
- [x] Implement `KairoGymEnv(gymnasium.Env)` with `reset()`, `step(action)`, `render()`, `close()`.
- [x] Implement observation/action space builders from KairoECS component schemas.
- [x] Provide `register_kairo_env()` helper for Gymnasium registry integration.

### Task 2.2 — RL training example
- [x] Create `examples/ml-surrogate/rl-training/` demonstrating PPO training loop with Stable-Baselines3.
- [x] Document the round-trip: Python RL agent → KairoECS simulation step → observation → policy update.

### Task 2.3 — Add CI validation
- [x] Add smoke test that imports `kairo_gym` and verifies `reset()`/`step()` contract compliance.
- [x] Add CI check that `kairo-ecs-ml` compiles with each feature flag independently.

## Phase 3 — Advanced inference backends

### Task 3.1 — TensorRT backend
- [x] Implement `TensorRtSession` for NVIDIA GPU-optimized inference.
- [x] Coordinate GPU memory with Track 32 via shared arena or explicit memory budget.
- [x] Benchmark TensorRT vs ONNX Runtime on target hardware.

### Task 3.2 — Burn backend
- [x] Implement pure-Rust inference path using the Burn framework.
- [x] Provide `BurnModelSystem` trait implementation for fully Rust-native ML.
- [x] Ensure Burn backend is compatible with WASM targets (browser-side inference).

### Task 3.3 — Neural surrogate API
- [x] Implement `NeuralSystem` trait: defines `predict(&self, inputs: &Tensor) -> Tensor`, `tick_phase() -> TickPhase`.
- [x] Implement `NeuralSystemPlugin` macro for declarative surrogate registration.
- [x] Validate surrogate output within 5% of original system for the test domain.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- [x] Run `cargo test --all-features` for the ML crate.
- [x] Run `pytest` for the `kairo_gym` Python package.
- [x] Validate that `cargo build --no-default-features` excludes all ML dependencies.
- [x] Run the de-surrogate benchmark and verify speedup claims.

### Task 4.2 — Update risk register
- [x] Move resolved risks to mitigated.
- [x] Promote unresolved inference latency or deadlock risks to release blockers.
- [x] Document ONNX Runtime and TensorRT version compatibility matrix.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.