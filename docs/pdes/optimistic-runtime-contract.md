# Event-owned optimistic runtime contract

Track 48 implementation packet, reviewed at main fc2f7b7 after Track 47 PR #193 merged. This is execution authority for the selected local implementation only. It does not waive distributed evidence, dependency, hosted, security or release gates.

## Model and ownership

Add `OptimisticRuntime<P>` beside the compatible local `TimeWarpRuntime` scaffold. Keep engine time and `RemoteEvent` unchanged; no core, ECS, debug or transport crate changes.

```rust
pub trait OptimisticProcess {
    type Snapshot: Clone;
    fn snapshot(&self) -> Self::Snapshot;
    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError>;
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent>;
}
```

A snapshot must independently own all handler-visible logical state, component membership, deterministic RNG state and reversible output state. Shared interior mutations and external side effects are unsupported. Capture initial snapshots before execution and pre-event snapshots before each handler. Restore errors and snapshot/restore/handler panics poison the runtime; no staged outputs publish. Expose immutable process/report access and runtime-bound LP validity tokens. Epochs and runtime identity never roll back; every mutation or restore invalidates that LP's previous tokens. Checked exhaustion returns an error.

The local driver owns LP pending queues, histories, snapshots, send logs and tombstones. It processes deterministic LP rounds, one minimum input per participating LP, without conservative incoming safe-time waits. This permits genuine local stragglers; it is not MPI/gRPC execution or distributed GVT proof. `new(partition, topology, processes, limits)`, `schedule_initial`, `receive`, `run_until_with_budget`, `fossil_collect`, `report`, `pending_events`, `state_token` and `process_at` are the intended surface. Initial scheduling closes on execution. Validate LP sets, routes, duplicate/conflicting delivery identities, horizon and capacities before external mutation.

## Identity and ordering

`EventOrderKey = (tick, source_lp, LogicalEventId)`. A private immutable logical node is either Root(source LP, stable caller sequence), or Output(parent full order key, vector ordinal). Define Root < Output; outputs compare parent full key then ordinal. Use immutable Arc ancestry; roots have depth 0, constructors reject depth 129 (maximum 128). No unchecked recursive enum or deserializer is public. Roots are fixed by scenario/source, never transport arrival. `RemoteEvent` has no sequence field. A stable caller root sequence is passed explicitly to `schedule_initial(stable_sequence, event)` and travels in the envelope identity; RemoteEvent remains unchanged.

Each output derives its logical ID from the current input's full key and ordinal. Delivery incarnation is a checked nonrollback counter, excluded from logical order. Replays preserve output-slot identity and mint fresh incarnations. First-execution sequence allocation is rejected: parents at ticks 10/20 both emitting at 30 must order their children 10 then 20 even when 20 executed first. Cancellation targets exact actual emitting LP plus opaque logical ID plus incarnation. Envelope source must match event.source_lp before mutation. Every tombstone, duplicate index, pending delivery and send record uses this actual-source namespace; do not obtain it from an ancestral Root source. Pending storage accommodates multiple incarnations under one logical key; a replacement arriving before its old anti is never overwritten. Retain actual output metadata for cancellation.

## Rollback, cancellation and progress

Compare complete ordering keys. A straggler restores the pre-event state of the earliest later input, retracts every recorded downstream output of the invalidated suffix and requeues its inputs. An executed anti restores before its exact target, permanently cancels that delivery, retracts its and later outputs, and requeues only surviving later inputs. Pending anti removes its matching positive; anti-before-positive creates an exact tombstone; repeated anti is idempotent. Never send an anti merely for an invalidated input. Route antis and replacements through owned delivery queues; locally drain antis before replacements.

Require output.tick > input.tick and declared output source == executing LP; validate the complete batch before publishing any item. Bounded limits cover LPs, pending/history/tombstone counts, output batch size and causal depth. Budget counts handler executions, including replay, and queued anti/cascade steps; one rollback operation may traverse the bounded retained suffix. At exhaustion leave the next unit pending and resume without duplicate work. Invalid post-handler output, identity exhaustion or restore failure poisons execution. Externally rejected malformed input need not poison an unchanged runtime.

GVT cannot exceed locally pending positives, antis, staged sends or replay work. Only history strictly before a validated floor may be collected; preserve the state required by the first retained event. Reject pre-GVT positive/anti arrivals without mutation; work exactly at GVT stays reversible. Caller-supplied proven GVT remains an external proof contract, not a distributed measurement. Report rollback attempts/depth, replay executions, canceled sends, checkpoint/history/pending counts and GVT lag. Define GVT lag as the maximum executed/fossil LP frontier minus GVT, clamped at zero; idle LPs must not hide speculative lead. Generic snapshots do not justify a total-memory-byte claim.

## Generational component membership

The real `GenerationBitset` stores fixed-capacity Vec<u64> active bits, maximum 65,536 slots (8 KiB bits). Private handles bind instance, nonrollback epoch and slot. Every successful insert/remove/restore invalidates all old handles; invalid operations leave the object unchanged. Snapshots contain logical bits/capacity, not authority. Same-capacity foreign snapshots are allowed; destination identity remains its own. Checked identity/epoch exhaustion and fallible construction/restore allocation return typed errors. Snapshot cloning is bounded but uses ordinary nonfallible Vec cloning.

Model snapshots save values, RNG and bitset.snapshot(); restoration preflights bitset restore before publishing staged model values and obtains fresh handles. Production token claims apply to this primitive and new runtime, not to unchanged scaffold behavior. A separate bounded legacy repair restores initialization and uses fresh validity stamps while preserving diagnostic generation metrics; it does not add production replay or downstream anti semantics to the helper.

## Independent acceptance oracles

Use noncommutative/state-dependent models: equal-tick reverse arrival; parents 10/20 emitting digits at 30 yield 12 under every arrival/replay permutation; canceled original child cannot survive replacement; anti-after-replacement and anti-before-positive; earlier input cancellation recomputes dependent sends; initial/RNG restoration; component/runtime/LP token isolation; GVT equality vs pre-floor rejection; repeated one-step budgets equal an uninterrupted run; complete-batch invalid output and panic/restore failure publish nothing and poison. Compare final state and committed logical trace to an independent sequential reference using actual core Scheduler where applicable. Attempt counts/incarnations are speculative metadata and may differ.

Local implementation tests and benchmarks are required before a phase PR. Track 48 remains In Progress until live distributed rollback evidence involving Track 49 exists. Current scheduling dependencies form a cycle: an explicitly accepted phase/interface handoff is required before Track 49 production dispatch. Drafting its wire tests is already permitted; no dependency bypass or Done claim is introduced here. EXC-193 covers only PR #193 and cannot authorize this branch's future npm gate.


## Checked codec bridge — alpha preview

The [codec bridge ADR](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/adr-codec-bridge.md) adds three pure methods under `time-warp`:

```rust
LogicalEventId::output_parts(&self)
    -> Option<(&OptimisticEventOrderKey, u32)>;
OptimisticEventOrderKey::try_from_parts(tick, source_lp, logical_id)
    -> Result<Self, OptimisticError>;
OptimisticMessage::try_from_parts(event, logical_id, incarnation, kind)
    -> Result<Self, OptimisticError>;
```

Together with `root_parts`, `root` and checked `child`, a codec can inspect and rebuild every ancestry node bottom-up, then reconstruct an exact positive or anti envelope. Checked reconstruction validates the entire ancestry iteratively: root source matches its ordering-key emitter; every output tick is strictly greater than its parent; depth128 is accepted and129 rejected. Output emitter may differ from parent emitter and root origin. All payload/destination fields, u32 ordinals, u64 sequences/incarnations and full native u128 ticks are preserved; incarnation and kind do not alter logical ordering. Native Tick must never be silently narrowed to the draft wire fixture's u64 range.

The native [integration fixture](../../crates/kairo-ecs-pdes/tests/optimistic_codec_bridge.rs) decomposes actual emitted multi-generation envelopes and reconstructs exact messages, including antis, with boundary and rejection tests. Reproduce its four cases with matching explicitly bound Cargo/rustc/rustdoc:

```sh
cargo test -p kairo-ecs-pdes --features pdes,time-warp --locked --test optimistic_codec_bridge
```

Expected output: four passing tests. The [source-bound evidence](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/codec-bridge-evidence/README.md) records98 crate tests plus Clippy/format under explicit1.98.1 and98 crate tests under matching1.76.0. Independent held-out acceptance and hosted/new-PR checks remain separate.

This bridge is a representation API, not a wire decoder or accounted transport admission. `receive` retains topology, route, configured limits, GVT and duplicate/conflict checks. Raw-byte/allocation bounds, duplicate-field parsing, source authorization, durable persistence and ACK semantics belong to the reviewed transport. Native delivery identity currently omits authority epoch, and published_messages copies already-local deliveries: the [authority decision candidates](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/authority-identity-design-candidates.md) and [owned/outbox prerequisite](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/owned-outbox-design-prerequisite.md) must be resolved before production distributed admission. No transport-only epoch, socket response or observer copy discharges those requirements.
