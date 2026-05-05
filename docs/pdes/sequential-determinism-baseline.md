# Sequential Determinism Baseline

The single-threaded `kairo-ecs-core` scheduler is the correctness oracle for PDES.
For the same seed, event set, priorities, and cancellation sequence, sequential
execution must dispatch events by:

1. ascending `SimTime`;
2. ascending priority value for equal timestamps;
3. insertion sequence for equal timestamp and priority.

PDES acceptance is based on final state parity with this oracle. Per-tick
interleaving may differ across logical processes, but a partitioned workload must
settle to the same observable component state as a sequential run.

Validation command:

```powershell
cargo test --manifest-path crates/kairo-ecs-core/Cargo.toml
```
