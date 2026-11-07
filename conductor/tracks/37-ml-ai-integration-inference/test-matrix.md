# Test Matrix: Track 37 ML/AI Integration & Inference

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| `kairo-ecs-ml` compiles with `--no-default-features` | yes | yes | yes |
| `kairo-ecs-ml` compiles with `--features onnx` | yes | yes | yes |
| `kairo-ecs-ml` compiles with `--features tensorrt` | no | yes | yes |
| `kairo-ecs-ml` compiles with `--features burn` | no | yes | yes |
| `kairo-ecs-ml` compiles with `--all-features` | yes | yes | yes |
| ONNX-facing scaffold validates metadata, tensor shapes, and backend-not-configured status without runtime dependencies | yes | yes | yes |
| Real ONNX model loads and infers inside simulation tick | no | yes | yes |
| Real ONNX inference does not deadlock under 1000 consecutive ticks | no | yes | yes |
| Consecutive inference runs without memory leak | no | yes | yes |
| Neural surrogate accuracy within 5% of original system | no | yes | yes |
| `kairo_gym` rejects execution without a qualified Rust environment | yes | yes | yes |
| Installed Rust environment passes Gymnasium API check (`check_env`) before advertising Gym support | capability gate | capability gate | capability gate |
| Installed native reset/step/close contract compliance before advertising Gym support | capability gate | capability gate | capability gate |
| RL training loop (PPO + Stable-Baselines3) completes without crash | no | yes | yes |
| TensorRT inference matches ONNX output within numerical tolerance | no | no | yes |
| Burn backend produces identical output to ONNX for same model | no | no | yes |
| Feature flag isolation: each feature compiles independently (`cargo hack`) | yes | yes | yes |
| Docs: ML architecture overview page exists and renders | no | yes | yes |
| Docs: surrogate authoring guide includes runnable example | no | no | yes |
| `cargo test` passes without GPU hardware (CPU-only CI) | yes | yes | yes |
| `cargo test --features onnx` passes on CI with ONNX Runtime installed | no | yes | yes |

**Current policy (7 November 2026):** [capability-scoped release gates](../../release-gates.md) supersede the blanket beta/1.0 feature inclusion above. All positive ML/Gym checks remain mandatory before that capability can be advertised. The Python host environment is retired immediately; its current tests prove explicit unavailability only. Real native Gym support remains open in Tracks 109/110.

Alpha scope note: current `OrtSession` is a dependency-free contract double. It
must not be reported as evidence that ONNX Runtime, TensorRT, Burn, or surrogate
accuracy validation has been integrated.

Local aggregate validator:

```bash
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/36-streaming-real-time-processing/validate-track36-40.ps1 -SkipCargoTests
```

This validator checks the dependency-free ML crate, the `kairo_gym` contract
tests with `PYTHONPATH` set to the package source tree, and bounded-claim docs.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.