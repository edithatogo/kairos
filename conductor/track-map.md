# KairoECS Track Map

## Find a Track

- Use the [track index](tracks.md) to find the specification, plan and handoff.
- Read the [authoritative registry](tracks.yaml) for status, owner, dependencies,
  owned paths and required gates. Phase acceptance is recorded in the
  [closeout ledger](phase-closeout.yaml).
- Use the [wave policy](wave-policy.md) and the DAG check in the
  [Conductor workflow](workflow.md) for dependency and execution order.
- Check [parallel execution](parallel-execution.md) and the
  [single-maintainer harness](harness/single-maintainer.md) before writing to a
  track's paths. Read the track's own test matrix before running its gates.

## Release-Critical Path

Use the [release gates](release-gates.md) and the candidate's frozen scope for
release decisions. For [native-core-v1](release/beta-scope.md), unfinished
experimental PDES, MPI, GPU, FMI and production HPC work does not block the
bounded core candidate. Each excluded capability keeps its own evidence and
acceptance gates. [Capability register](capabilities.md) · [Current handoffs](status.md).

[Historical track map at 9d76ca7](https://github.com/edithatogo/kairos/blob/9d76ca7f4b7f4b5898f2ec77515a7a7639f1478b/conductor/track-map.md).
