# Handoff: Track 37 ML/AI Integration & Inference

## Summary

Track 37 now has the first concrete ML scaffold. `kairo-ecs-ml` defines shape-checked tensors, dependency-free model metadata and tick-boundary input/output-shape validation, finite tensor-value validation, an ONNX-facing session wrapper surface, explicit backend-not-configured status, `NeuralSystem`, phase-filtered `InferenceTickHook`, feature-gated backend modules, a DE surrogate example, and a Gymnasium-compatible Python wrapper that can import without optional Gymnasium installed.

## Files created

- `crates/kairo-ecs-ml/Cargo.toml`
- `crates/kairo-ecs-ml/src/lib.rs`
- `crates/kairo-ecs-ml/tests/feature_matrix.rs`
- `docs/ml/architecture.md`
- `docs/ml/model-versioning.md`
- `docs/ml/surrogate-authoring.md`
- `examples/ml-surrogate/de-surrogate/Cargo.toml`
- `examples/ml-surrogate/de-surrogate/src/main.rs`
- `python/kairo_gym/pyproject.toml`
- `python/kairo_gym/setup.cfg`
- `python/kairo_gym/src/kairo_gym/__init__.py`
- `python/kairo_gym/src/kairo_gym/env.py`
- `python/kairo_gym/tests/test_env_contract.py`

## Contracts consumed

- Track 01: ECS tick boundary concepts from `conductor/contracts/core-contract.md`.
- Track 06: Python binding track requirement for a separate Gymnasium wrapper.
- Track 23: Domain model examples for surrogate validation.
- Track 32: GPU device/memory contract, still required before TensorRT implementation can move beyond a blocked stub.

## Release gates affected

- ML features are non-blocking for v1.0 (gated behind Cargo feature flags).
- `kairo_gym` Python package is distributed separately from the Rust crate.
- Default build path (`cargo build`) must exclude all ML dependencies.

## Tests added

- `crates/kairo-ecs-ml/tests/feature_matrix.rs`
- Unit tests in `crates/kairo-ecs-ml/src/lib.rs`
- `python/kairo_gym/tests/test_env_contract.py`
- Contract checks now reject blank model names/versions, empty or zero-dimensional tensor/model shapes, non-finite tensor values, tick-hook input/output shape mismatches, invalid Gym space sizes, string actions, non-iterable actions, and non-finite action values before runtime integration is attempted.
- Backend status checks now report that ONNX Runtime, TensorRT, and Burn are not configured in the dependency-free scaffold; the scaffold must not be used as evidence of real backend execution.

Validated:

- `cargo check --manifest-path crates/kairo-ecs-ml/Cargo.toml --no-default-features`
- `cargo check --manifest-path crates/kairo-ecs-ml/Cargo.toml --all-features`
- `cargo check --manifest-path crates/kairo-ecs-ml/Cargo.toml --tests --no-default-features`
- `cargo check --manifest-path crates/kairo-ecs-ml/Cargo.toml --tests --all-features`
- `cargo check --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml --features onnx`
- `rustfmt --check crates/kairo-ecs-ml/src/lib.rs crates/kairo-ecs-ml/tests/feature_matrix.rs examples/ml-surrogate/de-surrogate/src/main.rs`
- `$env:PYTHONPATH='python/kairo_gym/src'; python -m unittest discover -s python/kairo_gym/tests`
- `$env:PYTHONPATH='python/kairo_gym/src'; python -m compileall -q python/kairo_gym/src python/kairo_gym/tests`

Blocked validation:

- `cargo test --manifest-path crates/kairo-ecs-ml/Cargo.toml --no-default-features`
- `cargo test --manifest-path crates/kairo-ecs-ml/Cargo.toml --all-features`
- `cargo run --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml --features onnx`

The two `cargo test` commands compiled code but failed at Windows link time because this shell resolves `link.exe` to Git's `usr\bin\link.exe`, which failed with `couldn't create signal pipe, Win32 error 5`. Earlier example execution was also blocked by the same Windows linker setup; `cargo check` for the example passes.

## Risks and unresolved questions

- ONNX Runtime version compatibility across Linux, macOS, and Windows: CI matrix will need per-platform ONNX Runtime installation.
- TensorRT requires NVIDIA GPU, CUDA toolkit, and TensorRT SDK; testing will need self-hosted GPU runners.
- Neural surrogate accuracy threshold (5%) may be too strict for stochastic domains; threshold should be domain-configurable.
- Gymnasium API stability: the envelope pattern isolates `kairo_gym` from upstream changes, but breaking releases may force patches.
- `crates/kairo-ecs-ml/` is now included in the root workspace; optional runtime backends still need dependency-policy review before real ONNX/TensorRT/Burn adapters are added. The current ONNX-facing session is a contract double and does not load or execute ONNX Runtime.

## Worker 6 hardening evidence — 2026-05-06

- Added duplicate `(model name, model version)` rejection to `InferenceTickHook::try_register` so audit and stale-model checks are not ambiguous.
- Added Rust tests for duplicate model registration and documented the rule in `docs/ml/model-versioning.md`.
- Added Track 37 coverage to the Track 36-40 aggregate offline validator, including `kairo_gym` tests with `PYTHONPATH` set to the package source tree.

## Files changed

No additional file list was recorded by this Conductor hygiene update. Use the track plan, spec, and git history for implementation-specific file evidence.


## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.

## GNU toolchain rerun

The Windows `link.exe` failure is not a Track 37 implementation defect. The same owned surface passes under the installed GNU Rust toolchain on this host:

- `rustup run stable-x86_64-pc-windows-gnu cargo test --manifest-path crates/kairo-ecs-ml/Cargo.toml --no-default-features`
- `rustup run stable-x86_64-pc-windows-gnu cargo test --manifest-path crates/kairo-ecs-ml/Cargo.toml --all-features`
- `rustup run stable-x86_64-pc-windows-gnu cargo check --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml --features onnx`
- `$env:PYTHONPATH='python/kairo_gym/src'; python -m unittest discover -s python/kairo_gym/tests`

Result: 11 crate tests passed with `--no-default-features`, 8 feature-matrix tests passed with `--all-features`, the ONNX example checked successfully, and all 6 Python contract tests passed.

Current Track 37 status for the owned surface: implementation scaffold is validated and the remaining blocker is only the shared Conductor closeout surfaces, which are out of scope for this turn.
