# Q4 shared Flow/ABM contract v1

Status: draft durable contract awaiting final internal root/01/03/25 review. No runtime or full Q4 acceptance. Child base 60d5f4e9d6755f6b2731c8a87540ec1a14b1b85b. This supersedes the preserved predecessor's per-key carrier and omitted callback-despawn choices; its source analysis and safe borrow constraints remain applicable.

## One authoritative runtime and safe borrowed view

Flow owns the sole World, Registry and Scheduler; the adapter owns no competing instance. The existing ABMContext/BehaviorSimulation/AgentBehavior APIs remain byte-compatible and retain their independent legacy semantics. A new trait is required because old AgentBehavior can mutate World directly. Never adapt its Despawn result into a no-op or expose raw mutable shared World/Scheduler.

```rust
pub struct FlowWorldView<'a> { /* private &'a World and committed SimTime */ }
impl FlowWorldView<'_> {
    pub fn now(&self) -> SimTime;
    pub fn is_alive(&self, entity: EntityId) -> bool;
}
pub fn FlowRuntime::register_domain_view_hook<C: 'static>(
    &mut self, key: &str, kind: EventKind,
    callback: for<'a> fn(&'a mut C, &'a FlowCallbackSnapshot,
                         FlowWorldView<'a>, &'a mut FlowCommandSink),
) -> Result<(), FlowError>;
```

The view has no public constructor, Clone/Default or mutable access. Runtime delivery safely splits its separate World and Registry fields. Typed C is mutably borrowed from its actual ComponentStore row in the shared Registry. No immutable reference to the whole Registry aliases that mutable row. Arbitrary component queries are excluded until a separate safe store-splitting contract. Event snapshot and view time equal committed dispatch time. Old and view-domain callbacks share one descriptor family: one per key/kind, same existing duplicate/type checks. No extra scheduler or clock advancement is introduced.

## Dedicated carrier role and atomic creation

One behavior carrier and one behavior stream per actor generation, independent of behavior key. A second live association rejects atomically, even with a different key. A private explicit role distinguishes actor-domain carriers from task work; every public builder, manual for_work, timed_work and callback-batch Acquire rejects a carrier with InvalidWork before association/entity/scheduling mutation. Zero duration is representation only, never the eligibility rule. Existing normal work, including zero-duration timed work, keeps its accepted semantics.

```rust
pub fn FlowRuntime::create_actor_domain_context<C: 'static>(
    &mut self, actor: EntityId, registration: &str, kind: EventKind, context: C,
) -> Result<WorkId, FlowError>;
pub fn FlowRuntime::actor_domain_context(
    &self, actor: EntityId,
) -> Result<WorkId, FlowError>;
```

Creation requires a live Flow actor, valid nonblank registration and matching already-registered view-domain descriptor for exactly the requested kind; it cannot silently create an unregistered callback target. Preflight current run/halt state, actor membership/generation, descriptor/type/key consistency, absence of actor association, World operation-cap creation and registry role/row invariants before spawn/insert/index writes. Reuse checked existing allocation bounds; no prospective ID or mutable World exposure. After preflight commit creates one actual WorkId, typed context/role/membership and actor->carrier association together. Duplicate association returns a new explicit FlowError::DuplicateActorDomainContext; no carrier exists for invalid input. Missing carrier inspection uses existing InvalidWork, invalid actor uses InvalidEntity. A caller-supplied context is owned input; rejection can drop that value, but does not mutate previously stored context or draw streams. User Drop/panic/OOM effects are not transaction guarantees.

Carrier WorkSpec records the real owner/key, no request and zero original duration; WorkProgress remains Pending and cannot be acquired. Domain delivery targets the same opaque WorkId and checks role, owner generation and registry context. Cleanup removes carrier typed state, role and actor index alongside other owned work. Terminal task entities remain according to existing retention; carriers are despawned with their actor. A recycled actor gets a new full handle and no old association.

## ABM API and deterministic ownership

```rust
pub struct FlowAgentHandle { /* private actor + carrier WorkId + bound EventKind */ }
// Clone, Copy, Debug, Eq, PartialEq; no Default or public constructor.
pub struct FlowAgentContext<'a, C> {
    pub agent: EntityId,
    pub event: &'a FlowCallbackSnapshot,
    pub state: &'a mut C,
    pub rng: &'a mut DeterministicStream,
    pub view: FlowWorldView<'a>,
    pub commands: &'a mut FlowCommandSink,
}
pub trait FlowAgentBehavior<C> {
    fn update(&mut self, context: FlowAgentContext<'_, C>);
}
pub fn register_flow_agent_behavior<C: 'static, B: FlowAgentBehavior<C> + 'static>(
    flow: &mut FlowRuntime, registration: &str, kind: EventKind,
) -> Result<(), FlowError>;
pub fn create_flow_agent<C: 'static, B: FlowAgentBehavior<C> + 'static>(
    flow: &mut FlowRuntime, actor: EntityId, registration: &str,
    kind: EventKind, run_seed: u64, state: C, behavior: B,
) -> Result<FlowAgentHandle, FlowError>;
pub fn schedule_flow_agent_update(
    flow: &mut FlowRuntime, agent: FlowAgentHandle,
    at: SimTime, scheduler_priority: i32,
) -> Result<EventId, FlowError>;
```

The ABM carrier row owns state C, behavior B and a DeterministicStream; DES generic registration and role/index enforce uniqueness. Registration of generic bridge/key/kind happens explicitly before first work via the normal runtime registration gate; create_flow_agent never registers partially after spawn. The helper constructs the existing from_entity(run_seed, actor) stream without drawing, then atomically delegates carrier creation. Actor/registration/type/index/counter validation occurs before persistent state writes. Registering multiple purposes/streams per actor is excluded from this version.

Use unchanged existing entity derivation, including generation, with explicit run_seed. No ignored constructor seed or algorithm change. Advance stream only on actual live behavior delivery. Stale updates, admission rejection and preconsume budget/batch-ID overflow consume zero draws. A delivered callback whose batch is rejected retains its state and consumed draws and is never replayed. Same-runtime pause/continue retains draw position; task duration sampling is separate and cannot be redrawn by behavior resume. Direct registry/World mutation and legacy BehaviorDecision are unavailable in the new trait; stopping means no new Domain update, while despawn uses the buffered command below.


### Exact generic bridge and kind binding

ABM owns private `FlowAgentState<C, B> { actor: EntityId, state: C, behavior: B, rng: DeterministicStream }`. `register_flow_agent_behavior::<C,B>` invokes exactly one `register_domain_view_hook::<FlowAgentState<C,B>>(registration, kind, dispatch_flow_agent::<C,B>)`. It validates reserved kind, key/type and normal pre-first-work registration gates via that one existing-style descriptor operation; failure writes no descriptor. It creates no actor, carrier, stream or association. Duplicate same key/kind is rejected; compatible different kinds may be registered before any work. No compound helper registers after partially creating a carrier.

`create_flow_agent::<C,B>` takes an explicit kind and requires that exact key/kind view-domain descriptor with exact TypeId `FlowAgentState<C,B>`. The DES carrier creation helper performs this check along with all role/index/counter checks before writes. The resulting carrier role privately stores its kind binding; handle also carries that kind. One carrier per actor generation holds even when another registered key/kind exists. A second create fails rather than creating a second actor-derived stream. Role and handle kind must agree at inspection/scheduling/delivery; the handle cannot silently resolve an arbitrary kind from a key with several descriptors.

`schedule_flow_agent_update` uses the opaque handle's bound kind, validates real actor generation/carrier role/membership/key and bound-kind consistency, then delegates to the authoritative checked domain scheduling ingress. It cannot select another event kind, create a context, reseed a stream or bypass same-tick halt checks. Generic direct `schedule_domain` targeting an actor carrier must likewise require its bound kind; normal task domain contexts retain their existing multiple-kind behavior. A callback can emit a future Domain update with the captured real carrier WorkId and bound kind; another kind rejects the entire batch with InvalidWork. The view exposes no scheduler mutation.

The bridge has one higher-ranked delivery lifetime shared by mutable carrier row, immutable snapshot, immutable World view and mutable sink. Runtime first borrows the separate World field immutably and Registry field mutably, obtains the actual typed row, and reborrows both to the shorter callback scope. In ABM, destructure the carrier's separate state/behavior/rng fields, copy actor, and invoke `behavior.update(FlowAgentContext { agent, event, state, rng, view, commands })`. Rust can split those disjoint fields; no reference to the whole carrier/Registry is held beside mutable child fields. No callback may retain the borrowed view/sink/snapshot beyond update. This is a design binding, not an executed compilation claim: the initial fixture/bridge qualification must compile this real generic body and reject lifetime escapes before runtime acceptance.

Additional oracles: explicitly registered `<C,B>` succeeds and mismatched C or B fails before carrier allocation; creation without registration and registration after first work fail atomically; one key with two registered kinds creates only one carrier bound to the selected kind; direct/batch other-kind scheduling rejects without draws/IDs. Registration duplicate failures retain descriptor inventory and do not install a bridge for a wrong context type. Type-level compile fixture demonstrates field-split reborrowing; an attempt to retain a World view or obtain whole Registry while borrowing typed C must fail compilation rather than be supported through unsafe code.

## Buffered actor despawn as a fully admitted batch command

Extend the frozen experimental owned-command enum explicitly:

```rust
FlowOwnedCommand::DespawnActor {
    actor: EntityId,
    at: SimTime,
    scheduler_priority: i32,
}
pub fn FlowRuntime::despawn_actor_at_with_scheduler_priority(
    &mut self, actor: EntityId, at: SimTime, priority: i32,
) -> Result<(), FlowError>;
```

Existing despawn_actor(actor) delegates at current time/priority zero, preserving its signature and normal single-despawn behavior. The current implementation permits two ingress commands for the same live actor before either dispatches. New pending-despawn uniqueness rejects the second ingress while the actor remains live; this is an explicit experimental behavioral change under the compatibility/release hold, not an unchanged legacy guarantee. This public enum extension is experimental source-breaking for exhaustive matches, not blanket additive semver-safe. Require reviewed contract amendment/release hold before implementation; do not change the prior frozen five-command fixture to conceal the extension.

The sink issues an ordinary opaque ticket and applies the immutable callback cap exactly as for every command. Whole-batch pure validation checks live actor membership/generation, checked at>=committed now, one primary scheduler reservation, cumulative Flow/Scheduler counters and a cloned pending-despawn reservation set. Repeating the same actor in a pending or same batch returns the new explicit FlowError::DuplicateActorDespawn at the second issued ticket without any partial reservation/event IDs. InvalidEntity is reserved for an actually invalid/dead actor, not a live actor with pending despawn. Direct ingress returns DuplicateActorDespawn before scheduling, with existing pending reservation preserved. Forward/foreign/non-Acquire references continue to reject normally. Sink cap poison wins for the whole batch. Commit only after all commands pass, schedules the real existing Command::Despawn with the requested time/priority and returns its actual EventId (request/deadline fields None). Pending despawn does not immediately remove the actor or mutate resources/context; earlier/later commands remain ordinary scheduled operations.

Two-stage arithmetic is essential: callback-batch admission reserves the ingress event atomically; it cannot predict all future owned requests or cleanup notification counts at a later tick. At actual despawn dispatch, the existing pure preview-at Flow plan computes ALL then-live owner requests/work/carrier cleanup, all due boundaries, resource arbitration, lifecycle rows and legacy/new notifications before consuming the head. Aggregate destroyed counts, World generation bounds, every token reservation, registry context/role/index presence, revisions, checked time/accounting and persistent same-tick cost must pass together. Arithmetic or budget rejection retains the exact pending event and all actor/resource/carrier/context/stream/pending-release/notification state; no factory or callback runs. Ordinary semantic rejection follows existing due-boundary behavior and clears only its consumed despawn reservation. No fallible cleanup after commit and no per-request partial actor removal.

Commit follows existing canonical request-ID cancellation, then canonical resource arbitration. Cancelled work snapshots capture accounting at the dispatch time. Remove actor-owned work/carrier context and association only after aggregate preflight; release obsolete pending-release reservations and the consumed pending-despawn reservation. Future completion/deadline/notification/domain commands use existing logical liveness/revision checks and become visible stale outcomes without effects; queued notifications to removed context cost zero and never call it. Old completion tokens retain their special invalid-token empty/noerror contract. Existing request Command::Submit terminal-state errors are not relabeled empty just for this adapter. Do not eagerly cancel or remove unrelated scheduled events: that would alter actual EventId/time/order/stats behavior.

Future captured lifecycle telemetry must freeze each transition at its actual staged instant. Current completion/release clears request.lease before recording; preemption removes the old lease and records Preempted before Restart resets/requeues or Abort emits its terminal row. Therefore capture the causal prior lease explicitly before clear/removal, separately from any resulting active lease. Cancellation/despawn must likewise capture request/resource/work accounting and capacity/queue/active membership from that transition stage, not reconstruct records from final state after the entire actor cleanup. Track 04 owns the final schema and applicability; this contract requires source-backed snapshots but does not define or claim an implemented serializer. See the root retained call-site review `/private/tmp/kairos-q4-lifecycle-root-review-20261004/capture-callsite-findings.md` for source observations.

Same-tick user event priority and admission sequence determine order: behavior update before despawn can run once; despawn before update makes update stale zero-cost. Legacy/new cancellation notifications are scheduled legacy-first, but actor cleanup means their removed live contexts are not invoked. Preserve that current source-backed behavior rather than promising posthumous callbacks. Events on independent surviving actors/resources remain deterministic.

## Mandatory fixtures and private fault gates

- Public one actor carrier: different-key second creation rejects; invalid owner/registration/type and overflow (private injection) have no partial entity/association/context/stream writes.
- Both manual and timed Acquire carrier attempts (builder and sink) reject atomically; ordinary zero-duration task still completes exactly once.
- Shared view now/world identity matches DES interleaving; peer liveness changes only after committed despawn; no second ABM world or aliased registry access can compile.
- Existing from_entity golden draws, seed sensitivity, actor generation, independent actor scheduling, same-runtime pause stream retention and no duration redraw; do not claim statistical collision freedom.
- Document the existing double-ingress despawn behavior as the pre-extension baseline; test new direct and same-batch second ingress returns DuplicateActorDespawn while actor is live, with the first event/reservation unchanged. Preserve old signature and successful single cleanup.
- Behavior emits Acquire then Despawn in one batch and reverse ordering: all ingress IDs/events admitted together, actual dispatch order obeys priorities/sequence, cleanup is not synchronous callback mutation. Release followed by duplicate despawn/counter/time/cap failure rolls back ALL admission reservations.
- Private second-event schedule overflow, identity preconsume overflow, destroyed+carrier cleanup overflow, missing descriptor/context/role/index, aggregate cancellation legacy/new tokens and budget exceed all retain exact head/all state and no callback/factory/draw effects.
- Active/queued/Pending/suspended tasks of actor plus carrier are cleaned atomically; resource replacements/grants and cancellation rows follow canonical order. Old leases fail, future tokens stale as above, recycled actor handles cannot drive old carrier/RNG.
- Live callback invalid/over-cap batch retains state/draws once, no partial scheduler/event IDs. Despawn command counts toward default1024 cap and exactcap control; default/budget configuration not bypassed.
- Legacy ABM tests/APIs unchanged. New behavior does not invoke old mutableWorld trait; independent legacy context is not represented as a shared adapter.

## Ownership and sequencing

First durable doc proposed: conductor/design/flow/q4-shared-abm-v1.md plus explicit amendment reference to frozen typed-continuation contract for DespawnActor/role APIs, root-review before any writes. Future DES flow.rs owns view descriptor, role/index creation, both ingress and atomic planner/cleanup extensions; ABM flow_adapter.rs/lib.rs owns new trait/state/helper. Tests precede implementation. ABM adding a DES dependency requires coordinator-owned Cargo.toml/lock review (no DES->ABM cycle), not incidental worker edits. Owners01/03/25 review safe borrow/entity/RNG/source-compatibility;04 lifecycle snapshots remain separate. No core/state/RNG source, ABI, binding or clinical-policy change.

Serial freeze -> public failing fixture -> role/bridge implementation -> full batch/cleanup joins -> DES+ABM existing regression/host qualification; no partial acceptance or callbacks with stubbed despawn. FullQ4 synthetic workflow, captured lifecycle schema and phase review remain open; portablecheckpoint Track22 deferred.
