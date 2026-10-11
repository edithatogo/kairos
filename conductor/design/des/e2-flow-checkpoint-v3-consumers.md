# Flow checkpoint V3 consumer qualification

## Authority and dependency

Coordinator preparation under the full CareOps delivery goal. Routing source is
`c9091ce049a9d156c260bbc9f670f0e9bd4073e8`. Its reviewed lint-only repair is `5efb3710a2cb1b81256e948f45c27328a2de4e1d`.
Final clean-head qualification of that repair is separately required. This document
does not assert acceptance. Dispatch requires routing acceptance, independent
contract review, exact base/input hashes and a reviewed bounded packet per lane.
Rust/Cargo 1.99.0 only. No publication, parent pin or ED acceptance here.

Existing consumer seams use complete routed Image/Ref/command views. Start with
tests. Do not rewrite working generic production code to introduce version-specific
branches. Any necessary production change requires root review and explicit packet
scope; shared routing/validator changes require their own owner and rebinding.

## Three parallel lanes

All three workers start from the same accepted routing source plus reviewed
consumer documentation. One isolated worktree and disjoint claim per lane.

| Lane | Owned Rust files |
| --- | --- |
| Preview | `crates/kairo-ecs-calibration/src/shadow_native.rs`, `crates/kairo-ecs-calibration/src/c5_native.rs` |
| Coherent | `crates/kairo-ecs-calibration/src/flow_bridge.rs` |
| Portable | `crates/kairo-ecs-calibration/src/c2_portable_checkpoint.rs`, `crates/kairo-ecs-calibration/src/c2_portable_checkpoint/process_tests.rs` |

Each packet separately owns its result and log prefix. Protect every other source,
contracts, manifests and toolchain. A discovered shared dependency stops only the
affected work for review/rebinding; no assumed serialization or cross-lane edits.

## Preview lane

Required families (each adapter and the C5 helper must have its own actual witness;
use namespaced module tests or explicit helper invocations to prove both):

- `preview_v3_deferred_pending_flush_without_advancement`
- `preview_v3_disabled_queue_has_no_invented_completion`
- `preview_v3_halted_live_tick_and_repeat_step_error`
- `preview_v3_capture_limit_error_without_legacy_fallback`

Construct real Deferred resources and logical work, never detached images alone.
Duration5 Submit0 followed by Flush4 previews4 without stepping, actual grant4
then previews9, completes9. Preview preserves complete image, scheduler, request,
resource flags, counters and stored work state. For a disabled queue, distinguish
two real cuts: before Submit the request is Pending/no lease; after the bounded
same-tick Disable then Submit then Flush closure it is Queued/no lease with empty
active set and preview None, not an invented completion.

Budget1 with Submit0 consumed and pending Flush0 previews Some(0). Repeated step
returns exact SameTickBudgetExceeded{at_ticks:0,limit:1}, preserving pending image
and budget. A valid baseline plus a narrowed capture limit proves the actual error
propagates without legacy fallback. Preserve actual LIVE stale Completion11 as
next event before useful completion14; dispatch11 has no callback/lifecycle/raw
stored-progress change, then preview14. Clock-inspected progress can advance.
Do not modify C5 study configuration or claim new configuration support.

## Coherent lane

Required families:

- `coherent_v3_transit_control_and_owned_field_mutations`
- `coherent_v3_arrived_pending_submit_join`
- `coherent_v3_observed_frontier_required`

Use real transit pause/resume, arrived work with pending Submit, Deferred resource
and pending calendar Enable/Flush. Assert a valid coherent cut first, including
observed frontier and round-trip restoration. Independently mutate only fields the
bridge actually owns: Domain tick/priority/kind, Submit tick/priority, DomainControl
entity. Require exact existing diagnostics. Native mode/Flush/scheduler-command
joins remain native-validator responsibilities; no invented bridge checks to make
an otherwise unsupported mutation fail. Shared checkpoint_wire.rs is protected.

## Portable lane

Required families:

- `portable_v3_mixed_interior_cuts_fresh_process_continuation`
- `portable_v3_current_tick_flush_fresh_process_continuation`
- `portable_v3_halted_envelope_fresh_process_preserves_state`
- `portable_v3_malformed_inner_resealed_outer_rejects_precodec`

Extend both parent setup and compiled fresh-child case resolver with unique V3
case IDs, trusted context/domain/continuation codecs and identical graph, work,
resource/Flush setup. Outer C2PortableCheckpointV1 and existing section/provider/
fidelity/bridge/seed labels remain valid: they do not imply inner FlowV1-only.

Cover both actual mixed tick6 interior cuts before pending Notify/Flush. Prove
continuation context before and after notification (0/None/false to1/Some6/emitted),
all lifecycle intervals [1,4),[4,6),[6,7),[7,14), original sample lineage and useful+
remaining conservation. Select the second actual Completion6 by request and due
value, not first event ID (stale11 is earlier). Keep stale Completion11 LIVE: its
dispatch has no stored-progress/lifecycle/callback mutation; inspected progress
at11 is useful7/remaining3. Compare full source/fresh-child suffix and final state,
including decoded trace byte hashes, not only ASCII hexadecimal text hashes.

Current-tick Flush readmission must be actual release/grant on same tick. Halted
case preserves complete pending commands, counters, modes and frontier; repeated
step has the exact budget error in source and restored child. Bounded loops assert
the expected cut and terminal condition, never silently accept early exhaustion.

Malformed cases structurally decode V3, change one native join, successfully
re-encode it and reseal a valid outer envelope. Restoration fails native validation
before caller codecs, with a valid codec-positive baseline. Preserve all22existing
V1/V2 process cases and identities/provider/seed settings, and all19literal legacy
full-envelope goldens. Freeze four additive V3 process cases in the reviewed packet:
`portable-v3-mixed-tick6-trigger-first`, `portable-v3-mixed-tick6-urgent-first`,
`portable-v3-currenttick-flush`, `portable-v3-preflush-budget-halt`. The mixed pair
must genuinely differ actual completion insertion order, not only Flush order.
The current-tick case includes real release/grant readmission, not only an initial
Submit/Flush. Four new plus22preserved cases give26process cases after integration.

## Execution, review and integration

Each lane packet binds exact source API context, required names and cases, inputs,
three prospective commands (targeted strict lint-compile, named focused runtime proofs,
owned-file format), result/log ownership and maximum3failed DEVELOPMENT executions.
Keep immutable started/terminal receipts, actual argv/cwd/HEAD/toolchain and source
hashes before/after each command. The first command is targeted single-package
`cargo clippy --locked -p kairo-ecs-calibration --features flow --all-targets -- -D warnings`.
It typechecks tests and catches style issues before source commit. No full
four-package suite/Clippy in a source-stage lane; final joined strict Clippy stays
mandatory. This prospective improvement follows the preserved routing test-style
failure; no earlier failed command is reclassified or erased.

After independent frozen source/evidence review, one scoped SOURCE commit then
immediate release. A source commit is not consumer acceptance. Root joins the three
reviewed commits in deterministic order and binds a fresh artifact-only packet at
the exact clean combined HEAD. Run full four-package Flow tests, strict all-target
Clippy and owned-source format once, with a maximum2failed FINAL executions. Keep
all failed evidence; any repair needs a new bounded source claim/commit and new
qualification. Verify all new consumer families/process cases, inherited routing/
native/wire proofs,22oldprocess cases and19goldens. No source changes under gates.

All three lanes must pass joined qualification before parent Kairos pin updates.
Then one authoritative Flow clock/state ED adapter, followed by parallel staff32,
interruption24 and space32 fixture bindings; composition and remaining delivery
phases stay mandatory. No clinical/empirical/hardware/release completion follows
from these local synthetic checks.
