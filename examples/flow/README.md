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

## Q4 staged staff, bed, and cleaning example (experimental)

The runnable [synthetic public-API staff/bed/cleaning example](../../crates/kairo-ecs-des/examples/flow_staff_bed_cleaning.rs) stages the capacity-one staff claim before a separate bed claim. Urgent work suspends and resumes typed in-memory context. Patient-A's manual Bed-A lease remains held during a separate timed cleaning claim; Patient-B queues while cleaning and receives the bed only after the caller releases Patient-A's lease. The example uses no clinical rules or atomic multi-resource grant. The integration fixture compares continuous execution with pause/continue in the same live runtime; this is not portable checkpoint/restore or cross-process parity.

Q4 development source S: `b6671d75b77e2e98f4cd63dd6a73d7472c00ceb7`. Accepted source receipts: `.artifacts/q4-phase/source-qualification.json` (SHA-256 `0706c828d5b0a1b37c8cd77916c40681afbc701718915d9e11c85995cc3266f2`); exact owner run: https://github.com/edithatogo/kairos/actions/runs/37190690669. Parent pin integration is pending. A governance successor G requires fresh phase, strict clean-tree and exact-head native owner gates before parent acceptance.

`LifecycleRecord.snapshot` is experimental and source-breaking for exhaustive struct literals; migration and Track 25/release holds remain. `resource-lifecycle` and `resource-lifecycle-io` are optional; Arrow 60 feature tests use Rust 1.88 while default telemetry remains Rust 1.76. Full C1/C2, Q5, Track 22 portable checkpoint and release qualification are not claimed.

### Run and migration notes

```sh
cargo run --locked -p kairo-ecs-des --example flow_staff_bed_cleaning
cargo test --locked -p kairo-ecs-arrow --features resource-lifecycle-io
```

Use the pinned native developer toolchain for the example; the optional Arrow 60 path requires Rust 1.88 or later. Resource priorities belong to each claim and do not change scheduler priorities. Strict priority provides no starvation guarantee; the example makes staged one-resource claims and does not provide a deadlock-free multi-resource acquisition API.

Consumers of runtime-produced lifecycle records can read the new immutable `snapshot` field. Downstream exhaustive `LifecycleRecord` literals must supply that field and use the captured transition values; reconstructing them from the final World would erase intermediate queue, allocation and progress states. The `resource_lifecycle.v1` encoder validates contiguous per-event ordinals and uniqueness within the supplied batch, preserves input order, and does not provide a whole-run uniqueness writer. These changes remain experimental until Track 25/Q5 compatibility and release gates close.

## Q4 canonical Rust 1.99 source qualification

The bounded experimental source S 1123ad4bd0c9121a4a8f5f0be1229fbafc9861f6 is qualified by local Rust 1.99.0 checks, separate Rust 1.88.0 optional lifecycle/Arrow IO and Rust 1.76.0 default Arrow checks, plus exact-head native owner run https://github.com/edithatogo/kairos/actions/runs/37192093779. See `conductor/evidence/q4-development-source-qualification-20261004.json` for commands, source hashes and receipts. This is Q4 development qualification only; parent pin integration remains pending, and Track 03/04 historical Done scopes, Q5, Track 22 portable checkpoint, Track 25 compatibility, full C1/C2, and release gates remain open.
