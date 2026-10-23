---
title: Python
description: Python 3.15.1 bindings, zero-copy Arrow telemetry, and Gymnasium/PettingZoo MARL wrappers.
---

The Python binding (`bindings/python/`) provides a pure-Python scheduler facade for Python 3.15.1; native wheels and zero-copy Arrow export are planned.

## Current status

The shipped package is `kairo_ecs` (pure-Python scheduler facade). `kairo_ecs.ParallelVecEnv` is a
**placeholder**: `step` returns zeroed observations and rewards and does not call the Rust
`kairo-ecs-ml::ParallelVecEnv`. Native library loading (`_native.py`) is only a ctypes probe; PyO3/maturin
wheels are not yet built. There is no `kairo_gym` module in this package.

```python
from kairo_ecs import ParallelVecEnv

env = ParallelVecEnv(num_envs=4)
obs, infos = env.reset()
obs, rewards, terminated, truncated, infos = env.step([0, 1, 0, 1])
assert len(rewards) == 4  # placeholder values
```

Canonical sources:

- `bindings/python/README.md`
- `docs/tutorials/python-getting-started.md`
