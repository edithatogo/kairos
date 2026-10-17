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

From the Kairos checkout root, use the canonical compiler:

```sh
cargo +1.99.0 run --locked -p kairo-ecs-des --example flow_fifo_migration
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

The legacy helper grants synchronously; Flow acquisition first creates a pending
request and dispatch commits its grant. Flow release is also scheduled: the
active lease remains in force until dispatch commits the release. See the
[migration guide](../../docs/flow/queue-migration.md) for the method mapping,
deadline and cancellation boundaries, and continuation limits.

## Q4 staged staff, bed, and cleaning example (experimental)

The runnable [synthetic public-API staff/bed/cleaning example](../../crates/kairo-ecs-des/examples/flow_staff_bed_cleaning.rs) stages the capacity-one staff claim before a separate bed claim. Urgent work suspends and resumes typed in-memory context. Patient-A's manual Bed-A lease remains held during a separate timed cleaning claim; Patient-B queues while cleaning and receives the bed only after the caller releases Patient-A's lease. The example uses no clinical rules or atomic multi-resource grant. The integration fixture compares continuous execution with pause/continue in the same live runtime; this is not portable checkpoint/restore or cross-process parity.

Historical Q4 source S: `b6671d75b77e2e98f4cd63dd6a73d7472c00ceb7`. Its accepted receipt was `.artifacts/q4-phase/source-qualification.json` (SHA-256 `0706c828d5b0a1b37c8cd77916c40681afbc701718915d9e11c85995cc3266f2`), with the then-current owner run [37190690669](https://github.com/edithatogo/kairos/actions/runs/37190690669). That Q4 receipt reported parent-pin integration pending at the time; this is historical status, not a current pin readback. It required fresh phase, strict clean-tree and exact-head native-owner gates before parent acceptance.

`LifecycleRecord.snapshot` is experimental and source-breaking for exhaustive struct literals; migration and Track 25/release holds remain. `resource-lifecycle` and `resource-lifecycle-io` are optional; Arrow 60 feature tests use Rust 1.88 while default telemetry remains Rust 1.76. Full C1/C2, Q5, Track 22 portable checkpoint and release qualification are not claimed.

### Run and migration notes

```sh
cargo +1.99.0 run --locked -p kairo-ecs-des --example flow_staff_bed_cleaning
cargo +1.99.0 test --locked -p kairo-ecs-arrow --features resource-lifecycle-io
```

The staff example prints ordered lifecycle rows in the form
`t=<ticks> <label> <transition> priority=<n> queue=<n> active=<n>`. Its source
asserts that continuous and pause-at-boundaries runs have identical records,
terminal states and resource conservation. The exact source-level stdout oracle
is the ordered `continuous.records` loop at
[flow_staff_bed_cleaning.rs:488](../../crates/kairo-ecs-des/examples/flow_staff_bed_cleaning.rs#L488),
after equality is checked at line 487. The coordinator records the current-source
stdout; this page does not claim an execution receipt.

Use the pinned native developer toolchain for the example; the optional Arrow 60 path requires Rust 1.88 or later. Resource priorities belong to each claim and do not change scheduler priorities. Strict priority provides no starvation guarantee; the example makes staged one-resource claims and does not provide a deadlock-free multi-resource acquisition API.

Consumers of runtime-produced lifecycle records can read the new immutable `snapshot` field. Downstream exhaustive `LifecycleRecord` literals must supply that field and use the captured transition values; reconstructing them from the final World would erase intermediate queue, allocation and progress states. The `resource_lifecycle.v1` encoder validates contiguous per-event ordinals and uniqueness within the supplied batch, preserves input order, and does not provide a whole-run uniqueness writer. These changes remain experimental until Track 25/Q5 compatibility and release gates close.

## Historical Q4 source qualification (not current Q5.3 evidence)

The bounded experimental source S `1123ad4bd0c9121a4a8f5f0be1229fbafc9861f6`
was qualified by local Rust 1.99.0 checks, separate Rust 1.88.0 optional
lifecycle/Arrow IO and Rust 1.76.0 default Arrow checks, plus exact-head native
owner run [37192093779](https://github.com/edithatogo/kairos/actions/runs/37192093779).
See [`q4-development-source-qualification-20261004.json`](../../conductor/evidence/q4-development-source-qualification-20261004.json)
for that historical source's commands, hashes and receipts. Those receipts do
not qualify the Q5.3 source. Parent pin integration, Q5.3 compatibility, Track 22
portable checkpoint, full C1/C2, and release gates remain separate.

For the current experimental migration scope, see the
[queue backend boundaries](../../docs/flow/queue-backend-boundaries.md). No
stable API promotion or release approval follows from these examples.
