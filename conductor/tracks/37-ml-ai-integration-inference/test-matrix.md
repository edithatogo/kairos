# Test Matrix: Track 37 ML/AI Integration & Inference

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| `kairo-ecs-ml` compiles with `--no-default-features` | yes | yes | yes |
| `kairo-ecs-ml` compiles with `--features onnx` | yes | yes | yes |
| `kairo-ecs-ml` compiles with `--features tensorrt` | no | yes | yes |
| `kairo-ecs-ml` compiles with `--features burn` | no | yes | yes |
| `kairo-ecs-ml` compiles with `--all-features` | yes | yes | yes |
| ONNX model loads and infers inside simulation tick | yes | yes | yes |
| ONNX inference does not deadlock under 1000 consecutive ticks | yes | yes | yes |
| Consecutive inference runs without memory leak | no | yes | yes |
| Neural surrogate accuracy within 5% of original system | no | yes | yes |
| `kairo_gym` Python package imports and passes Gymnasium API check (`check_env`) | yes | yes | yes |
| `kairo_gym` reset/step/close contract compliance | yes | yes | yes |
| RL training loop (PPO + Stable-Baselines3) completes without crash | no | yes | yes |
| TensorRT inference matches ONNX output within numerical tolerance | no | no | yes |
| Burn backend produces identical output to ONNX for same model | no | no | yes |
| Feature flag isolation: each feature compiles independently (`cargo hack`) | yes | yes | yes |
| Docs: ML architecture overview page exists and renders | no | yes | yes |
| Docs: surrogate authoring guide includes runnable example | no | no | yes |
| `cargo test` passes without GPU hardware (CPU-only CI) | yes | yes | yes |
| `cargo test --features onnx` passes on CI with ONNX Runtime installed | no | yes | yes |
