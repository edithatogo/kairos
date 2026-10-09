# C2 sealed replay demo

This example runs one fixed synthetic Flow scenario through the experimental
`kairo_ecs_calibration::experimental_c2_replay` API. It demonstrates a paused
checkpoint at tick 1, a two tick pause, five route ticks, and a twenty tick
service completion at tick 27. The journal reconstructs the closed scenario
and replays its prefix; it does not serialize a Flow runtime or accept scenario
configuration from the checkpoint.

Build the example once, then save and restore in separate processes using that
same built executable:

```sh
mkdir -p .artifacts/c2-replay-demo-cli
cargo run --locked -p kairo-ecs-calibration --features flow --example c2_replay_demo -- save .artifacts/c2-replay-demo-cli/checkpoint.json
cargo run --locked -p kairo-ecs-calibration --features flow --example c2_replay_demo -- restore .artifacts/c2-replay-demo-cli/checkpoint.json
```

The parent directory must already exist and pass the API's path checks. Save
refuses to replace an existing checkpoint. Restore prints the canonical JSON
trace for the fixed suffix. Save and restore must use the same executable
binary: the journal binds to its executable fingerprint, so a rebuild or a
different binary is rejected.

The example is a doc-hidden experimental API demonstration for one synthetic
recipe. It is not a general checkpoint facility and does not establish C2,
C5, ED, clinical, hosted, or release acceptance.
