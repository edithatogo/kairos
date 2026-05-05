# Flow Examples

This directory contains Track 03 examples that combine the DES trajectory API and
the ABM behavior API over the shared scheduler/entity contracts.

Current R2 smoke slice:

- `kairo-ecs-des::Trajectory` schedules fixed-tick DES steps and returns a
  deterministic dispatch trace.
- `kairo-ecs-abm::BehaviorSimulation` schedules behavior-update events for
  entities and runs `AgentBehavior` implementations in scheduler order.

Publication-ready examples still need scenario files and conformance fixture
exports before they should be promoted into the model zoo.
