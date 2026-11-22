# Native queue environments

`QueueEnvironment` runs a finite shared queue with seeded arrivals. Actions idle or serve, and rewards depend on completed jobs and waiting work. `ParallelVecEnv` owns a batch of those worlds and returns results in input order. A failed validation leaves the batch untouched. Reset accepts one seed per world, and `reset_at` restarts an individual completed world.

The experimental `kairo-gym` package provides `KairoGymEnv` for one agent and `PettingZooQueueEnv` for ordered agents sharing a queue. Each adapter connects to one feature-enabled Rust process. Linux and macOS use the same bounded `queue-env-v1` stdio protocol. A missing or incompatible executable returns an error. Always close the adapter when finished.

```sh
cargo build --locked --release -p kairo-ecs-cli --features rl-queue
python -m pip install --require-hashes -r python/kairo_gym/ci-requirements.lock
python -m pip install --no-build-isolation --no-deps ./python/kairo_gym
KAIRO_NATIVE_QUEUE_BIN="$PWD/target/release/kairo-ecs-cli" \
  python -m unittest discover -s python/kairo_gym/tests -v
```

Use the repo-selected Python 3.15.2 interpreter. Supply the built executable explicitly to the adapter.

```python
from pathlib import Path
from kairo_gym import KairoGymEnv

with KairoGymEnv(executable=Path("target/release/kairo-ecs-cli").resolve()) as env:
    observation, info = env.reset(seed=42)
    observation, reward, terminated, truncated, info = env.step(1)
```

The real Gymnasium and PettingZoo checkers, direct native dispatch comparison, deterministic reset and pipe lifetime regressions cover this queue model. Broader simulation environments require their own adapter and conformance evidence.
