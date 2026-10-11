# ADR: bounded public Flow event preview

Status: Reviewed design for bounded implementation; runtime acceptance pending.

Authority: The root coordinator serves as the Track03 scoped owner under the user's full CareOps delivery authority. This agent provided independent source-based design review; root reviewed and accepted the public contract. The reviewed design authorizes preparation of a bound source packet. Dispatch still requires exact packet review and a fresh claim.
Decision source: Kairos 7d3917833b7fa27b5ea0727d00381ec5a6915d60, consumer qualification accepted in parent commit 01a476bafdb1b30fa5b7998c5815d2ad654f104f. Rust 1.99 only.

Parent ED execution requires inspecting the next event before deciding whether it lies within the observation horizon. Flow owns its scheduler privately. Checkpoint image inspection currently supplies this information but requires registered codecs and bounded full-image construction. Core already supplies the exact projection without dispatch.

Decision: add FlowRuntime::peek_next(&mut self)->Option<kairo_ecs_core::ScheduledEventPreview>, delegating only self.scheduler.peek_next(). Keep core, private calibration bridges and checkpoint layouts unchanged. Return pending head while halted, and include stale LIVE completions. Heap tombstone pruning is permitted; logical time, counters, live pending membership and domain state stay unchanged. No public stats accessor is added.

Ownership: Track03 des-api-agent; actor/transit proof review coordinated with abm-api-agent. Source packet owns flow.rs, flow_preview_v1.rs and e2-flow-preview-v1.md. ADR and handoff are coordinator preparation artifacts, frozen before dispatch. Consumers may subsequently adopt this public API under separate packets.

Consequences: immutable logical preview uses mutable borrow because Core prunes dead heap heads. Returned value is copied; later scheduling can change the next event. Preview does not grant resources, deliver callbacks, consume transition budget, clear budget halt or resume execution. Caller owns horizon policy; <=H admits equality and >H stops without dispatch.

The proof requirements and stop/qualification rules are specified below. External tests compare public checkpoint/budget/domain images. Inline private tombstone test uses private scheduler stats/live-entry snapshots; it introduces no public test API.

Acceptance: all named actual-runtime proofs and strict source-stage gates at frozen source; independent review/scoped commit/release; separate exact clean combined full four-package tests, strict Clippy and format; owner acceptance before parent pin. Maximum three failed development executions with all receipts preserved; final stops on any failure.

No ED fixture/runtime, clinical, empirical or release completion is claimed.

## Required actual-runtime proofs

Use names prefixed flow_preview_ and fresh actual FlowRuntime construction. All signatures used already exist: spawn_actor, create_resource_with_allocation_mode, create_work, acquire(...).owner(...).timed_work(...).at(...).scheduler_priority(...).submit, schedule_resource_grant_enabled, schedule_allocation_flush, step, now, budget_snapshot, request/resource/work_progress/work_context. For rich complete-state non-mutation proof, use existing public V3 capture/codecs; preview itself requires no codecs.

1. flow_preview_empty: new runtime returns None twice; now/stats remain identical.
2. flow_preview_ordered_live_head: real scheduled commands at distinct ticks, priorities and tied priorities; preview includes exact id/time/priority/sequence/kind/entity and matches each subsequent real dispatch. Equal-priority ties follow actual insertion sequence. Repeated previews do not consume events or change stats.
3. flow_preview_cancelled_head_preserves_logical_state: INLINE private test in flow.rs creates a real Flow runtime and scheduled commands, cancels the current scheduler head using its existing private scheduler, removes the matching command test-side to maintain owner-map consistency, then previews the next live head. Snapshot now/stats/live membership plus request/resource/work state before preview; unchanged after, including repeated preview. Only dead heap entries may disappear. Public Flow cancel(request) schedules semantic cancellation and does not expose event cancellation, so an external public test cannot directly manufacture this tombstone. This fixture is test-local; no production API expansion. Do not replace this with a detached DTO.
4. flow_preview_deferred_disabled_flush: duration5 Submit0 then Flush4; preview4 before dispatch, real grant4 then preview9/completion9. Separately disabled queue before Submit previews pending0; after actual Disable/Submit/Flush closure previewNone while queued/no lease remains. A pending disabled Flush is previewed despite no possible grant. Assert complete routed image and budget/domain state unchanged by each preview.
5. flow_preview_live_stale_completion: real Suspend preemption with low work10 starting1, urgent work2 from4 to6, resume7, leaving LIVE stale completion11 and useful completion14. Preview11; actual stale dispatch11 has empty lifecycle/callback receipts, unchanged raw stored progress and active callback context; inspected elapsed7/remaining3 is permitted. Next preview14 and actual logical completion14. A genuine completion callback baseline must be exercised.
6. flow_preview_budget_halted: budget1, actual Submit0 consumes budget and Flush0 remains pending; previewSome0, step exact SameTickBudgetExceeded{at_ticks:0,limit:1}; repeat preview and repeat step preserve complete pending image, budget and domain state. It must not become RunHalted or None.
7. flow_preview_horizon: caller checks at<=H; event exactlyH is dispatched, futureH+1 remains pending and previewable without dispatch or clock advance. Do not add a horizon policy to Flow. Assert identity of future event and unchanged state when caller stops.

## Bounded verification proposal

Bind exact commands at accepted current HEAD, Rust/Cargo1.99 only. Three source-stage command types: targeted strict cargo clippy --locked -p kairo-ecs-des --all-targets -- -D warnings; focused cargo test --locked -p kairo-ecs-des flow_preview_ -- --test-threads=1 (both inline and external tests); owned rustfmt --edition2021 --check --config skip_children=true on flow.rs and flow_preview_v1.rs. Format before captured gates. Capture argv/cwd/HEAD/toolchain/source before-after hashes/exits/log hashes and actual required named successes. Allow at most THREE failed DEVELOPMENT executions total within the reviewed source scope. Preserve every started/terminal receipt and failed log; bounded correction of compile/test issues is permitted within that ceiling. Stop at the ceiling or whenever repair requires an out-of-scope change, new semantics or weaker oracle. Final qualification stops at any failed gate and requires separate reviewed repair. Source commit follows independent frozen review; release immediately. Combined four-package tests/strict Clippy/format at exact clean integration HEAD remain a separate final packet before API acceptance.


## Source-stage limits

Stop for HEAD, input hash or ownership drift, unavailable Rust 1.99, missing prerequisite, or a required change outside the reviewed scope. Preserve all command receipts. Three failed development executions is the total ceiling for this packet, not three retries per command. The final joined qualification stops on any failed gate. Source acceptance requires actual evidence; this design closes no runtime gate.

## Public API governance review

This compact review covers `docs/api/api-review-template.md` and `docs/design/api-review-template.md`. The exact protected root is `crates/kairo-ecs-des` (inventory ID `rust-des-api`), surface family `rust_api`; scoped conformance proofs accompany the addition. Proposed by Track03/root coordinator; release stage alpha, experimental Rust API. Compatibility classification: compatible, additive only. Design accepted for bounded implementation; runtime acceptance pending.

### Compatibility matrix

| Surface | Current root | Change | Breaking? | Status and consumer action |
|---|---|---|---|---|
| Rust API | `crates/kairo-ecs-des` | Add `FlowRuntime::peek_next` | No | Alpha/experimental; existing consumers need no source edits |
| C ABI | `include/kairo_ecs.h` | None | No | No symbols, statuses, allocation or ownership change |
| Arrow schema | `schemas/arrow/event_log_v1.schema.json` | None | No | No fields, types or meanings changed |
| Host APIs | Existing binding roots | None | No | No binding or migration change |

No root is renamed, split, merged or removed. Existing event ordering, `SimTime`, replay and deterministic fixture outputs remain unchanged. No adapter or pin change is required merely to compile existing consumers. Later parent adoption and dependency pin updates are separately owned integration work. A migration note is unnecessary for this additive alpha method; beta, stable and release planning retain their own review gates. This ADR is the compatibility note; method rustdoc and `conductor/design/des/e2-flow-preview-v1.md` provide user documentation. Release notes and catalog promotion remain future release-owner work.

### Ownership, error and concurrency model

The caller holds an exclusive `&mut FlowRuntime` borrow during preview. The returned `ScheduledEventPreview` is copied and contains no borrowed runtime state. The delegation adds no allocation, fallible result or new error variant. `Option::None` indicates no live scheduled event. Existing Flow concurrency properties are unchanged; this method does not add shared mutable access, locks or a thread-safety promise.

### Conformance and alternatives

The seven exact named proofs above update the scoped source conformance evidence. Shared fixture catalog, beta/stable promotion and release acceptance remain separate. Rejected alternatives are exporting private calibration bridges, adding a public stats API, scanning complete checkpoint images in production solely for preview, or modifying Core scheduler semantics. No existing consumer is required to migrate its preview implementation in this packet.

### Red-team objections and responses

- Mutable preview may look like a simulation step: the mutable borrow permits physical tombstone pruning only. The proof compares logical time, counters and live membership; heap representation equality is intentionally not promised.
- Preview might skip stale completion and mislead horizon handling: stale LIVE Completion11 remains the next event. It is dispatched normally; no semantic liveness filter is added. The stale-event fixture verifies callback and raw progress invariants before useful Completion14.
- Halted preview might silently resume execution: the method bypasses neither budget enforcement nor the halt. It returns the pending head while repeated `step` returns the exact existing budget error and complete state remains unchanged.
- A copied event might become stale after another operation: rustdoc states later scheduling/cancellation can change the head. It is an observation, not a reservation or dispatch ticket.

### Signoff boundary

API governance/design review: root coordinator acting as Track03 scoped owner, with independent source-based review by this agent. Red-team objections above are addressed by the bounded contract and required oracles; implementation proof remains pending. Release reviewer: not requested and not claimed. Final decision: accept this additive alpha design for bounded implementation; no beta, stable or release reviewer approval follows.
