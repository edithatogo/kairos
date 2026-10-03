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

## FIFO migration to the experimental Flow runtime

The runnable [FIFO migration example](../../crates/kairo-ecs-des/examples/flow_fifo_migration.rs)
compares three labeled claimants in the retained legacy `Resource` helper with
three actors created in one `FlowRuntime`. Both admit equal-priority claims in
FIFO order. Entity IDs belong to their own runtime; the example compares label
order, not IDs across worlds.

The legacy helper grants synchronously and returns the next queued entity on
release. Flow acquisition returns a pending request; `step()` commits the grant.
Release requires the actual active lease and changes state at dispatch. A released
lease cannot be reused. The example asserts those differences and exits on any
unexpected dispatch error; it does not imply that the return values or event
counts of the two APIs are identical.

From the Kairos checkout root, after selecting the matching installed compiler:

```sh
cargo +1.98.1 run --locked -p kairo-ecs-des --example flow_fifo_migration
```

Expected stdout:

```text
Legacy FIFO: first -> second -> third
Flow FIFO: first -> second -> third
Flow commands commit at dispatch; released leases cannot be reused.
```

`flow_builder_migration_v1` separately checks timed Suspend work, priority versus
scheduler priority, deadlines, rejected admissions and pausing at a dispatch
boundary in the same runtime. This example uses manual leases only. It does not
implement a shared ABM adapter, lifecycle telemetry or portable checkpoints.
The public Flow API is experimental and retains its compatibility/release holds.
