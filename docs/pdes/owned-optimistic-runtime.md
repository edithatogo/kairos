# Owned optimistic runtime alpha preview

The `time-warp` feature provides an owned constructor and process-local root routing. Each runtime holds models, snapshots, queues and emitters only for its owned LPs. Its immutable descriptor retains the complete global partition/topology and configured authorities. Disjoint runtimes must register actual peer objects and seal complete coverage before routing.

This page describes the joined process-local owned runtime. Its [acceptance record](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/owned-native-acceptance-20261004.md) binds source, independent fixtures and actual compiler checks; the [behavioral contract](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/owned-handler-retirement-leaf.md) and [API appendix](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/owned-handler-retirement-api.md) define its limits. Track 48 remains In Progress pending live distributed acceptance.

## Route and acknowledge a root

1. Construct each owner with `OptimisticRuntime::new_owned`. Register the actual peer authorities and seal the complete disjoint ownership set. A newly constructed lookalike is a different issuer even when every configured field matches.
2. Call `schedule_initial` on the actual source owner. A local destination gets exactly one local queue entry. A remote destination gets no local queue or mirrored process; the source retains an outbox obligation. Initial native roots start at incarnation zero. The configured emission epoch is authority metadata, not an incarnation seed.
3. Obtain remote tickets from `ready_native_sends`; `outbound_pending` is inspection. A returned/copied `OptimisticMessage`, `as_anti` conversion or observer copy cannot mint a ticket.
4. The actual destination owner calls `admit_native` with the opaque ticket. Admission queues/accountably retains the root and returns a capability bound to both actual owners. It does not prove handler execution or retirement.
5. Return that capability to the source's `acknowledge_native_admission`. The source closes only its exact outbox obligation and keeps completion readback plus the root-cohort reservation. Exact retries validate the actual actors and retained facts before returning without additional accounting capacity or revision.

`NativeAccountingAuthority` has a weak lifecycle witness. Cloning it does not keep its runtime alive. Runtime Drop invalidates its gate before model destructors; routing/admission publication holds canonical deduplicated actual-peer gates through validation/publication. Its `is_live` getter is a point-in-time observation.

## Identity, bounds and observations

Storage identity includes source, complete scoped authority, full logical ancestry and exact incarnation. Logical execution order excludes authority, incarnation and message kind. Native ticks/namespaces retain all u128 bits; counters retain their full native width. Public authority has no ordering/hash interface.

Every accepted local or remote root consumes one lifetime cohort reservation from the shared transition bound. ACK cannot permit reusing its caller root sequence. While inputs remain open and ordinary envelope validation passes, repeated initial scheduling returns the existing cohort-conflict error; closed inputs reject with `InitialSchedulingClosed` first. Remote roots reserve an outbox entry and source completion slot; admission reserves a receiver receipt and pending slot. Sender and receiver roles share the receipt bound. Exact retries require no extra capacity when those bounds are full. Local roots require no transport receipt.

`accounting_snapshot` reports actual owned frontiers, local/outbound minima and unique reserved/retained receipt counts. It is an observation, not a global cut. `close_initial_inputs` prevents new roots while retained admission/ACK remains possible. Raw owned `receive` cannot authenticate scoped tickets; raw owned fossil collection currently rejects with `NativeGroupCutRequired`. Use `fossil_collect_native_group` with the complete actual ownership group for an atomic common cut after inputs are closed and execution/channel obligations permit it. Cuts collect strictly before GVT and preserve equality and unresolved retirement dependencies. A failed preflight advances no participant floor; failures identify the selected failing runtime and all already-poisoned participants. Post-publication destructor failures are reported separately in the committed cut report.

## Execute, replay and retire

`run_owned_until_with_budget` executes staged owned handlers. A positive round validates and publishes its complete output batch; suffix rollback and executed cancellation each consume one control budget unit. Snapshots restore model state, including deterministic RNG state, before surviving input incarnations replay. A handler failure with successful compensation leaves no partial batch; failed restoration poisons the owned runtime. Cleanup after publication has a distinct committed outcome.

Retained outputs produce real anti-message and retirement obligations. Exchange requests through `pending_native_retirement_requests` and `receive_native_retirement`; after the destination applies cancellation, return `applied_native_retirements` to `acknowledge_native_retirement`. Applied proofs, source admission/completion facts and predecessor/ancestor barriers determine when replacements become eligible. An admission acknowledgement alone proves no handler effect or retirement. Blocked local replacements reserve queue pressure and appear in public pending-event and pending-positive totals even before admission to the runnable queue.

Native tickets, acknowledgements, retirement proofs and cut participants bind actual process-local actors. They are not wire-authenticated records and cannot be serialized into remote proof. Durable restart, authenticated external accounting, MPI/gRPC acceptance, migration and distributed GVT remain separate implementation and evidence gates. No release or publication claim follows from this preview.
