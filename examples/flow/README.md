# Flow Examples

Maturity: preview.

This directory contains Track 03 examples that combine the DES trajectory API and
the ABM behavior API over the shared scheduler/entity contracts.

Current R2 smoke slice:

- `kairo-ecs-des::Trajectory` schedules fixed-tick DES steps and returns a
  deterministic dispatch trace.
- `kairo-ecs-abm::BehaviorSimulation` schedules behavior-update events for
  entities and runs `AgentBehavior` implementations in scheduler order.

Reproducibility commands:

```powershell
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1
```

Expected output: both commands complete with all named Track 03 fixture tests
passing and no failed tests.

Publication-ready examples still need scenario files and conformance fixture
exports before they should be promoted into the model zoo.
