# Vectorized VecEnv and MARL

Native vector environments and multi-agent reinforcement-learning adapters remain
experimental or unavailable. [Rust-only policy](../../conductor/native-only-policy.md)
requires Rust to own each world's state, RNG, action effects, reward and termination;
host-language APIs may marshal data and call that implementation.

## Current surfaces

The [Rust ML crate](../../crates/kairo-ecs-ml/src/lib.rs) exposes `ParallelVecEnv`
dimensions, `reset` and `step_batch`. The current batch method constructs
observations/rewards from action values without executing independent model worlds;
`step_with_ort_session` is a no-op. Thread-count parity tests exercise this scaffold.
`OrtSession::run` copies/repeats input values; `runtime_status()` reports an
unconfigured runtime. The method does not execute an ONNX model.

The [Python vector/PettingZoo constructors](../../bindings/python/kairo_ecs/vecenv.py)
and [Gymnasium environment/registration](../../python/kairo_gym/src/kairo_gym/env.py)
now fail explicitly. Installing Gymnasium or finding a shared library does not
enable these environments. The [Python native scheduler](../../bindings/python/README.md)
is a separate, narrower API without a qualified RL model adapter.

## Required native qualification

- S09: execute a real licensed model against independent nonidentity outputs, with shape/operator errors and held-out surrogate accuracy checks.
- S10: compare actual batched and scalar Rust trajectories; verify seeded resets, action/reward/termination semantics, cross-world isolation and thread-count replay.
- Run real Gymnasium/PettingZoo API checks through installed native artifacts, including buffer ownership, close/error behavior and missing-backend rejection. Shape checks and synthetic rewards cannot satisfy this gate.

Follow [completion items S09–S10](../../conductor/scientific-completion.md) and the
[Track 110 plan](../../conductor/tracks/110-scientific-method-completion/plan.md).
Publish training or throughput claims only after the model contract and
[measurement scope](../../conductor/performance-programme.md) are independently qualified.
