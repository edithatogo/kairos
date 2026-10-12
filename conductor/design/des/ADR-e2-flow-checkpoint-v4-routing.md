# Flow checkpoint V4 image/ref routing

Status: root-reviewed owner contract; implementation and runtime acceptance pending. Direct wire accepted at de8afe50322cd708e868543758231359b0926cd3, result SHA f6117b533876fc2ec7f0532c743d8bde17cd370a0a8903a58a0db411130a1190: 417 passed, 0 failed, 1 ignored, all ten direct wire proofs. Durable parent archive conductor/evidence/e2.2-flow-v4-direct-wire-completion-20261012 is committed at e84770f4f3d29f7e35b143658012af8ed94d7b40; acceptance SHA af5b23ced99420b4309a7aee632bf961edff86525eadf84f63ae2969536c5264. Root and independent GPT-6.1-sol reviewed actual API lifetimes, exhaustive matches, exact legacy selection and proof scope. Bind the commit containing this contract before source dispatch; this contract does not itself dispatch a worker.

## Scope and signatures

Exactly four source paths: crates/kairo-ecs-des/src/flow/checkpoint/v2.rs; crates/kairo-ecs-des/src/flow/checkpoint.rs; crates/kairo-ecs-des/src/flow/checkpoint/wire.rs; new crates/kairo-ecs-des/tests/flow_checkpoint_routing_v4.rs. Existing exhaustive production Image/Ref matches are confined these files; other consumers use generic accessors/let-else. Existing V3 routing test match includes wildcard.

Add enum variants FlowCheckpointImage::V4(FlowCheckpointV4) and FlowCheckpointRef::V4(&FlowCheckpointV4). Keep signatures unchanged:
- Image::as_ref(&self)->FlowCheckpointRef<'_>;
- Image::restore_with_rebind(self,&FlowCheckpointCodecs,FlowCheckpointLimits)->Result<(FlowRuntime,FlowCheckpointRebindV1),FlowCheckpointError>;
- FlowCheckpointRef<'a>::scheduler(self)->&'a SchedulerCheckpointV1;
- FlowCheckpointRef<'a>::work_registrations(self)->&'a [FlowWorkRegistrationV1];
- FlowCheckpointRef<'a>::context_stores(self)->&'a [FlowContextStoreV1];
- FlowCheckpointRef<'a>::commands(self)->impl Iterator<Item=(EventId,FlowCommandV3)> + 'a;
- FlowRuntime::capture_checkpoint_image(&self,&FlowCheckpointCodecs,FlowCheckpointLimits)->Result<FlowCheckpointImage,FlowCheckpointError>;
- Image::encode_wire(&self,FlowCheckpointWireLimits)->Result<Vec<u8>,FlowCheckpointWireError>;
- Image::decode_wire(&[u8],FlowCheckpointWireLimits)->Result<Self,FlowCheckpointWireError>.

V4 Ref borrows existing fields and commands reuse V3 command iterator; no command version4, copied temporary DTO or downconversion. Image restore delegates accepted native restore_checkpoint_v4_with_rebind. Wire routing delegates accepted encode_wire_v4/decode_wire_v4 on KFLOWV4 NUL. Direct old formats remain unchanged/failclosed.

## Deterministic source selection

Select representability from authoritative source inventory BEFORE capture: any live StableKey resource mode OR any retained request key requiresV4. Empty StableKey resource stillV4. Terminal key after full-resource removal stillV4; reused slot generation cannot downgrade old keys. Pending/Queued/Active/Suspended keyed requests stayV4. Missing/corrupt live mode/cache/key/unknown keys cannot be hidden by fallback: chosen strict capture validates source metadata/parity and fails. Invalid retainedkey belonging unknownrequest also triggersV4 validation. Use native accepted map inventory, not catch errors to infer version.

Only automatic capture entrypoint bypasses its old unconditional legacy ordering guard when it selectsV4, by directly calling accepted native capture_checkpoint_v4. Legacy direct V1/V2/V3 APIs retain original guard. All AdmissionOrder/no-key source states preserve existing minimal selection: Deferred or pending/allocation Flush command requiresV3; otherwise convert the captured V3 base to V2 using existing validation, then chooseV1 exactly when all resource_grant_flags are true AND commands contain no GrantEnabled; chooseV2 otherwise. Existing V2-to-V1 conversion validation remains unchanged. A started frontier alone is not a source selection predicate. Existing source-inventory validation and error policy unchanged. No failedV4capture retriesV3/V2/V1; no failedV3capture downgrades.

## Exact required proof names

1. routing_v4_empty_stable_resource_selects_before_capture: actual emptyStableKeyresource returnsV4, directcapture equality, controls validbaseline.
2. routing_v4_all_keyed_lifetimes_and_removed_generation: actual Pending/Queued/Active/Suspended/terminalremoved keys all chooseV4; slotreuse separate generations; all metadata preserved.
3. routing_v4_ref_borrowed_accessors_and_commands_complete: actual imageRef scheduler/registrations/contextstores and commands exact field/iterator equality for all V3commandtags where actual acceptedfixtures support them; no missingFlush/GrantEnabled/DomainControl/Completion.
4. routing_v4_image_direct_native_restore_suffix_equal: real native keyed runtime, direct vsImage restored event/state/progress/context/fullimage suffix equality; actual rebind identity scoped to returned runtime.
5. routing_v4_image_wire_magic_roundtrip: actualImage encode/decodeV4 bytes equal directacceptedwire and actualrestored suffix; oldV1/V2/V3magics still route correctly; unknown/truncated/errorpropagation exact.
6. routing_v4_corrupt_source_and_limits_never_fallback: independently mutated cache/mode/key source or ownedfixture controls, chosenV4 fails before caller codecs where native authority specifies; exactlimit/excess failures remainV4 error, no oldcodec invocation or version fallback.
7. routing_v4_legacy_minimal_selection_unchanged: actual AdmissionOrder/no-key Eager/disabled/Deferred/currenttick/halted/pendingFlush examples route sameV1/V2/V3 as old source and exactdirectcapture; existinglegacytests and literalbytes unchanged.
8. routing_v4_old_direct_capture_remains_failclosed: V1/V2/V3 direct captures reject liveStableKey and retained terminalkeys before codecs; automaticV4capture succeeds positivecontrol without weakeningoldguards.

Malformed V4 wire/schema/order errors belong accepted directwire proofs; routing preserves them exactly, no new framing or alterederrorprecedence. In-source unit controls may access private metadata for corruption only inside ownedcheckpoint.rs; no fixture edits outside scope without reviewed newpacket.

## Commands and acceptance

Format first. Three pinned Rust/Cargo1.99 gates: strict DES all-target Clippy, full DESpackage tests with all8exactnames/oldtests, four-sourceformat. Maximum3failed DEVELOPMENT executions; everyfailure root-review STOP, retainimmutable started/result/rawlogs. Fresh root authority binds packet/driver/worktree/base and exact complete failedreceiptlist for reviewed correction. Ready only latest ordered terminal gates exit0 at identicalcurrent4hashes with actualrawnames/counts. Frozen independent source review before commit.

Routing acceptance is additive hidden Rust alpha/experimental enum/API support, no C ABI include/kairo_ecs.h or Arrow schemas/arrow/event_log_v1.schema.json change, no scheduler/dispatch/replay. Adding enumvariant is an exhaustive-match source compatibility change; bounded inventory found required production matches in scope, future callers must account for newvariant. No release reviewer/beta/stable approval. Downstream preview/coherent/portable consumers wait acceptedcommonDESrouting+ABMwire join for fullnewversionqualification; inherited26childcases/4V3traces/19literaloutergoldens remain and newHandoff childproofs precede parentpin. ParentEDadapter/88fixtures/clinicalrelease separate.


## Coordinator compatibility follow-up (12 October 2026)

The original routing packet and rebound are closed without acceptance after three failed development executions, all preserved. The initial exhaustive-match inventory was incomplete: the old V3 iterator test and preview helper required explicit V4 arms (commit 2177b148132d054a8822c0341cce7db1dcea6930). The new routing fixtures then required hook registration before context creation and an actual transition-budget halt (snapshot 8d098fd86ca7a0024003c0d71aea6c864bf0219a).

The remaining legacy queue-order helper incorrectly includes automatic Image capture among V1/V2/V3 rejection paths. A separate coordinator-owned correction is limited to crates/kairo-ecs-des/tests/flow_queue_order_v1.rs and this addendum: retain all direct V1/V2/V3 fail-before-codec checks for every existing lifetime, and assert actual routed V4 equals direct V4 at the existing complete-codec empty-resource and removed-resource cuts. Do not add a routed-success assertion to the intentionally unregistered aborted-context fixture. Preserve every other oracle and all production source.

After independent source review, commit this correction and bind a fresh artifact-only qualification to that exact clean HEAD, protecting all seven routing/compatibility source paths. Root performs one fresh ordered strict DES Clippy, full DES tests and seven-file format sequence. Historical three failed executions remain immutable and counted; the explicit coordinator escalation permits at most one further failed execution (cumulative ceiling four), with immediate stop and owner redesign if it fails. This is not a reset or a Luna retry. Final raw receipts and all eight routing names plus the retained queue-order lifetime proof are required before acceptance.
