# Kairos DES/ABM and Flow API inventory

Date: 2026-09-29  
Inspected source pin: `764048a89872eae74e35bd92c5dff8a1527bd5b1`
Scope: current Rust public/source surfaces relevant to the proposed CareOps
first-class resource queue and preemption work. This is an inventory, not API
approval or implementation evidence.

## Current surfaces

| Surface | Current behavior | Source |
|---|---|---|
| DES `Resource` | Standalone FIFO helper. Holds `capacity`, `available`, and `VecDeque<EntityId>`; a request either consumes one unit or appends to the queue. Release hands off to the first queued entity or increments availability. It has no request identity, queue priority, deadline, cancellation, or preemption contract. | `crates/kairo-ecs-des/src/lib.rs:103-157` |
| DES `DESContext` | Publicly owns `Scheduler`, `World`, and `Vec<Resource>`; `new(_seed)` ignores the seed. `schedule_at` sends requests directly to the scheduler. It has no `ComponentRegistry` and is not a shared DES/ABM runtime. | `crates/kairo-ecs-des/src/lib.rs:160-202` |
| ABM `ABMContext` | Separately owns `Scheduler`, `World`, and `ComponentRegistry`; `new(_seed)` also ignores the seed. Component access is generic attach/get. | `crates/kairo-ecs-abm/src/lib.rs:55-109` |
| ABM behavior | `BehaviorContext` exposes mutable `World`, an event, and a mutable deterministic stream, but not component reads or a buffered command sink. `BehaviorSimulation` owns its own `ABMContext`; its loop processes every dispatched event that has a live entity, without checking `BEHAVIOR_UPDATE_EVENT_KIND`. | `crates/kairo-ecs-abm/src/lib.rs:9-16`, `111-190` |
| Shared ECS state | `World::despawn` increments slot generation with wrapping arithmetic. `ComponentRegistry` stores type-erased component stores and supports typed `insert`/`get` plus removal from an already registered component type; it has no operation that removes every component for an entity. | `crates/kairo-ecs-state/src/lib.rs:117-160`, `341-390` |
| Scheduler and event types | Scheduler order is owned by Kairos core. `schedule` increments event index, sequence, scheduled counter, and event generation without checked overflow; the generation uses wrapping addition. `EventKind` currently only contains `Custom(u32)`. `ScheduleRequest` exposes time, scheduler priority, optional entity, and kind. | `crates/kairo-ecs-core/src/lib.rs:37-84,117-127`; `crates/kairo-ecs-types/src/lib.rs:168-220` |
| RNG use | Existing ABM behavior creates a `DeterministicStream` from the run seed and entity handle. No RNG algorithm or stream-derivation change is proposed by this queue work. | `crates/kairo-ecs-abm/src/lib.rs:111-141`; source hash recorded below |

## Capability status

| Capability | Status at inspected pin |
|---|---|
| ECS resource definition, priority claim queue, active lease accounting | Missing from the inspected DES/ABM source. |
| Waiting timeout, explicit claim cancellation/reprioritization, stale lease revision | Missing from the inspected DES/ABM source. |
| Suspend/Abort/Restart work lifecycle and owned continuation context | Missing from the inspected DES/ABM source. |
| One authoritative DES/ABM scheduler, world, and component registry | Not present. DES and ABM contexts each own independent state. |
| Read-only shared-state behavior adapter with checked, buffered commands | Not present in current ABM behavior API. |
| Entity-wide component cleanup and checked scheduler/handle generations | Not provided by current core/state APIs. Track 01 implications need an explicit supported-use limit or additive core/state contract; a DES wrapper cannot claim these existing APIs are overflow-safe. |
| Protected API roots for DES and ABM | Both `crates/kairo-ecs-des` and `crates/kairo-ecs-abm` exist in the workspace/package matrix but are absent from `docs/design/protected-surface-inventory.json`. The proposed adapter introduces public symbols in both crates; Track 25 review therefore covers both exact roots. |

The negative symbol search in the packet checks the DES and ABM source trees
for `FlowRuntime`, `ResourceCapacity`, `ClaimQueue`, `ActiveAllocations`,
`ResourceRequest`, and `PreemptionStrategy`. It is a bounded absence check, not
a claim that no differently named helper exists anywhere in the repository.

## Additive proposal direction (owner-approved for design; not implemented)

The CareOps proposal is to add an experimental Rust `FlowRuntime` in
`kairo-ecs-des` that owns one scheduler, world, and component registry, while
keeping the existing `DESContext`, `Resource`, and standalone ABM APIs
source-compatible. A Flow-specific ABM adapter would read the shared runtime
through restricted queries and submit typed commands for validation at
deterministic dispatch boundaries. Queue priority remains separate from
scheduler event priority. The proposal does not change core event ordering,
RNG derivation, Arrow schemas, FFI, or host bindings.

The Kairos owner approved the four Q0.1 recommended dispositions on
2026-09-29, directing the design to proceed while retaining the listed
implementation and release gates. The Track 03 adapter adds public symbols in
`kairo-ecs-abm` as well as `kairo-ecs-des`; both exact roots are therefore in
the compatibility review. Independent Track 01/03/25 technical-review findings
and the owner disposition are recorded in the parent. The complete proposal and
its remaining Q0.2/Q0.3 contract questions are in the parent
repository at
`conductor/design/queue/ADR-0003-flow-runtime-contract-proposed.md`.

## Source hashes

SHA-256 values below were computed from the inspected checkout at the recorded
Kairos pin. They bind the source reviewed by this inventory; they are not claims
about later upstream revisions.

| Source | SHA-256 |
|---|---|
| `crates/kairo-ecs-des/src/lib.rs` | `15699dd5982dd83d206ae84f0e81eaa1910ccf0be0ec5424dea3a4c40efda72a` |
| `crates/kairo-ecs-abm/src/lib.rs` | `41d1e7f20860d260c6c02e8afb98aab94ac14cf8e1689acc11357216c32df7f9` |
| `crates/kairo-ecs-state/src/lib.rs` | `88cf420d5cb2b083bd76e8882a83fcc049001e905e197bba2785f72c1ec5aead` |
| `crates/kairo-ecs-core/src/lib.rs` | `8c7d09d7f55837a5d1557f49b97cbfc7014ba6a51dca5fb93ddd1695f80da7e3` |
| `crates/kairo-ecs-types/src/lib.rs` | `9613c19c885183844ab48c157c34e9df718fc3c01363e982a847fb3e8240542b` |
| `crates/kairo-ecs-rng/src/lib.rs` | `68397e53959221c17a9705da1dd6e23a8221df9825e879222cdd9b154029a991` |
| `docs/design/protected-surface-inventory.json` | `1f8acfc40665fe08230a71b2f656be64048811bbd548f0c76ae5c69efce62694` |
