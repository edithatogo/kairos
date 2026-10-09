# ADR-0011: C20 paired-flow runner compatibility

**Status:** bounded compatibility repair, reviewed internally; no C2 phase,
release, or acceptance gate is closed.

## Decision

The private `BoundIntrinsicWork` bridge exposes a crate-private read-only
`sampled_duration()` accessor returning the already sampled service duration.
This preserves the typestate boundary and lets the frozen paired-flow fixture
assert its empirical provider result without consuming or resampling a stream.

When the paired-flow runner detects the production modules and Flow feature, it
builds only the named frozen paired fixture with `flow` and
`kairo-ecs-abm/test-support`, selecting that exact test with `-- --exact`.
Green requires that test to be the sole selected result: exactly one passed,
zero failed, zero ignored, and zero measured; the test harness may report other
tests filtered by the exact selection. The expected missing-module red path
remains a compile-time missing-file check and does not enable the feature or
alter its diagnostic oracle. The fixture source and its hash remain unchanged.

The runner keeps using its established C20 artifact directory and disposable
output cleanup behavior. This compatibility repair does not introduce an
alternate artifact path.

## Boundaries

- The runner uses Rust 1.99.0 and the locked disposable archive it already
  creates. It does not change package manifests or the lockfile.
- Selecting one exact test keeps the runner's green meaning narrow; the full
  calibration suite remains a separate check.
- The C20 fixture remains frozen; this commit changes only the private bridge
  accessor, the runner's exact test invocation, and this ADR.

## Verification

The named green result is recorded under
`.artifacts/mvp/C2.0.red-tests.paired-flow`. The result records the exact
fixture SHA, Cargo argv, toolchain, exit status, and output hash. The expected
missing-module red oracle remains in the runner for the baseline source state;
this compatibility commit does not claim a red result from the repaired tree.
Local results do not establish hosted checks or external maintainer
acceptance.
