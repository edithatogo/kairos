---
title: Python
description: Python 3.10–3.14 bindings, zero-copy Arrow telemetry, and Gymnasium/PettingZoo MARL wrappers.
---

The Python binding (`bindings/python/`) exposes the KairoECS runtime to Python 3.10–3.14 with zero-copy Apache Arrow table export and a native multi-agent reinforcement learning (`kairo_gym`) interface.

## Python Surface Overview

| Module / Class | Purpose | Underlying Rust Crate |
| --- | --- | --- |
| `kairo_ecs` | Core simulation lifecycle (`create`, `step`, `snapshot`, `restore`, `export_arrow`) | `kairo-ecs-ffi`, `kairo-ecs-core`, `kairo-ecs-arrow` |
| `kairo_gym.ParallelMarlVecEnv` | Batched multi-agent RL environment compatible with Gymnasium & PettingZoo Parallel API | `kairo-ecs-ml::ParallelVecEnv` |
| `kairo_gym.OptunaObjective` | Deterministic ensemble objective wrapper for Optuna hyperparameter & SBI calibration | `kairo-ecs-cli::ensemble` |

## Quickstart Example

```python
from kairo_gym import ParallelMarlVecEnv

# Vectorized multi-agent simulation stepping across 64 parallel worlds
env = ParallelMarlVecEnv(num_envs=64, obs_dim=4, action_dim=2, seed=42)
obs, info = env.reset()
actions = [i % 2 for i in range(64)]
next_obs, rewards, terminated, truncated, step_info = env.step(actions)
assert len(rewards) == 64
```

Canonical sources:

- `bindings/python/README.md`
- `docs/tutorials/python-getting-started.md`
