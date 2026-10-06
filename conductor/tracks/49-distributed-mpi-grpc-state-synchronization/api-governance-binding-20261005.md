# Track49 prospective API governance binding — 2026-10-05

Disposition: preparation evidence; exact API/type freeze and release HOLD.
Base5379e73e0d3e70cc54fdbe29cfbfe975d1fb1f46, isolated Track49 worktree.
No public API, protected inventory, policy, manifest, runtime, status or pin edits.
Accepted design a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5
is consumed unchanged. Pending material wire amendment is not adopted.

## Concrete governance mismatch and owner handoff

The module preparation places the new boundary in `crates/kairo-ecs-pdes`, avoiding
a cyclic new crate. That root exists in package catalog/matrix but is absent from
`docs/design/protected-surface-inventory.json`. The API review template requires
an exact inventoried affected root. Substituting `crates/kairo-ecs-core` would be
false. Therefore this is a prospective review, not a template-compliant acceptance.
Track25 governance must reserve/review an exact PDES inventory entry and alignment
with conductor/api-design-review.md, versioning compatibility, compatibility matrix
and release/package records as applicable. Runtime ownership stays Track48/49;
governance acceptance cannot confer implementation or release authority.
This concrete handoff does not mutate shared governance or approve a broader promise.

The earlier module-preparation phrase “all12 questions” is imprecise. The current
conductor/api-design-review.md lists12 required review fields; the review template
contains7 compatibility questions. This record binds both, plus the affected surface
assessment, rather than treating twelve unrelated surface categories as those fields.

## Twelve required review fields

| Field | Prospective answer |
| --- | --- |
| affected_root | crates/kairo-ecs-pdes; not currently inventory-covered, blocking complete intake |
| surface_family | rust_api; planned canonical conformance fixtures separately use conformance |
| compatibility_level | release-hold; proposed additive experimental surface, pending exact classification |
| breaking_change? | No replacement/removal of existing APIs proposed; final public semantics and exact exports remain unclassified until binding |
| migration_note_required? | No current consumer migration proposed; required before beta-or-later if final API alters consumer behavior |
| adr_required? | yes, new public semantics; external-accounting-v1 is reviewed design baseline, full freeze HOLD |
| release_hold? | yes: inventory gap, exact types/features and independent fixture/backend/dependency proof incomplete |
| consumer_impact | opt-in external runtime with fixed verified driver and portable codec; native witnesses cannot cross wire/restart |
| deprecation_or_transition_plan | retain native runtime; no automatic mode switch or native witness serialization; no existing API deprecation proposed |
| fixture_or_schema_impact | all22 versioned canonical record fixtures and distributed parity/recovery evidence required; existing deterministic expectations preserved |
| red_team_objections | inventory mismatch; incomplete signatures/types; proof opacity; unstable support/MSRV claims; schema amendment pending |
| decision | release hold / preparation only; Track25 owner disposition and joint exact freeze still required |

## Seven template compatibility questions

| Question | Answer at this proposal |
| --- | --- |
| Additive only? | intended yes for PDES exports; exact inventory not yet accepted |
| Alters public semantics? | yes, introduces authenticated durable external runtime; no existing native semantic change authorized |
| Renames/splits/merges/removes protected root? | no; same existing PDES crate proposed |
| Alters deterministic ordering/replay/fixture output? | new versioned external fixtures required; existing ordering/replay/output changes not proposed |
| Alters C ABI ownership/allocation/status/symbols? | no proposed change |
| Removes/retypes/renames/changes Arrow field meaning? | no proposed change |
| Downstream source edits/adapters/version pins? | new users must opt into new runtime/codec/provisioning; existing consumers no migration proposed; dependency/support promises pending |

## Prospective exports and exact unresolved bindings

The following is an inventory of existing design text, not implemented Rust API.
All final visibility, receivers, bounds, field definitions and constructors must be
bound before serial scaffold dispatch. Existing source has forbid(unsafe_code),
default=[], pdes=[], time-warp=[] and rust-version1.76. No external feature exists;
feature/export graph and dependency compatibility are not approved by this record.

| Group | Names / members | Remaining exact binding |
| --- | --- | --- |
| Runtime/codec | ExternalOptimisticRuntime<P,C>; PortableModelCodec<P>; encode_process, decode_process, encode_snapshot, decode_snapshot | associated types, receivers, schema/RNG/limit descriptors, CodecError and immutable-limit contract |
| Constructors | new_external, resume_external; ExternalAccountingDriver::open | free/associated placement, exact ownership and error fields; standby/recovery transitions |
| Model operations | seed_external_roots, close_external_initial_inputs, admit_external, acknowledge_external_admission, receive_external_retirement, run_external_until_with_budget, acknowledge_external_retirement | complete typed arguments/results; root batch atomicity, committed budget semantics, handle validation/consumption rules |
| Observations/cuts | pending_external_records, applied_external_retirements, prepare_external_cut, apply_external_cut | bound output sizes, deterministic ordering, polling errors and cut result fields |
| Driver verification | verify_delivery, verify_admission, verify_cancellation, verify_retirement, verify_cut_plan, verify_cut_decision, verify_readback_request | receivers/mutability, precise verified-handle lifecycle and verification error fields |
| Delegated/runtime controls | verify_external_record, readback_external, export_external_checkpoint, prepare_external_manifest, activate_external | prose-only exact signatures, closed verified enum, coordinator private initial-prepare mapping |
| Driver observations | readback, export_checkpoint | receivers, complete request/response/checkpoint definitions; no raw journal/driver/signing accessor |
| Opaque handle families | Delivery, Admission, Cancellation, Retirement, CutPlan, CutDecision, ReadbackRequest, ManifestProposal, ManifestDecision, RecoveryOpen (VerifiedExternal variants) | private construction in fixed driver, permitted getters/traits/reuse, restart/session/journal/revision binding; no Deserialize/public constructors |
| Identity | SimulationId, ParticipantId, SessionId; ConfigDigest, EnvelopeDigest, FactDigest; RecoveryGeneration, FenceTerm, EmissionEpoch, ChannelSequence, JournalPosition, Revision | fixed widths from design, exact visibility/constructors/getters/traits and validation; LPu32/ticku128 preserved |
| Inputs/provisioning | ExternalRootInput, ExternalProvisioning, JournalDirectory, DriverLimits | complete fields, trust/key ownership, input-schema amendment disposition, negotiated limits and concrete fixed backend selection |
| Errors/progress | CodecError, ExternalOpenError, ExternalRecoveryError, ExternalVerificationError, ExternalOperationFailure, ExternalRunProgress, ExternalCutReport | complete enum/struct fields, visibility/traits, authoritative committed/unknown result semantics |
| Portable output | signed *Record types, ExternalOutboundRecord, ExternalReadbackResponse, ExternalCheckpoint; all22 wire kinds | exact public-versus-internal type mapping, bounded complete canonical schema and golden digest/signature evidence |

No caller-defined verifier or generic journal callback can issue a proof. Codec
outputs are unverified data; proof fields/constructors stay solely inside the fixed
driver boundary. Runtime must recheck activation/revision at every handle entry.
No implementation of Serialize/Deserialize/Clone/Copy is inferred by a type name.
Complete external checkpoint import cannot be implemented by reconstructing native
weak references or pointer witnesses. RecoveryOpen currently appears in the verifier
enum but lacks a complete public/private control-entry map; this must be resolved.

## Protected and consumer surface disposition

| Surface | Preparation disposition |
| --- | --- |
| crates/kairo-ecs-types, core, state, rng | consumed types/semantics; no edits or changed promises authorized; any later required helper/refactor has separate owner/API review |
| include/kairo_ecs.h | no external proof/runtime exposure proposed; ABI symbol/ownership/status changes require separate review |
| bindings/python, bindings/r, bindings/julia, bindings/typescript, bindings/csharp, bindings/go | no exposure proposed; each later host binding needs its owner/API review; no browser/Wasm support inferred |
| schemas/arrow/event_log_v1.schema.json | no schema change; signed accounting records are independent of Arrow telemetry; telemetry merge remains Track49/51 acceptance |
| conformance/fixtures | protected root; new versioned independent canonical and behavioral fixtures needed; existing fixture expectations preserved |
| batch/determinism | root batches and bounded polling/execution proposed; exact atomicity/order/limits and deterministic replay proof remain required |
| crates/kairo-ecs-mpi, crates/kairo-ecs-grpc | transport roots already catalogued draft-only; any new public transport API requires exact export inventory and governance coverage review at dispatch |

No alpha/beta/package maturity advancement is declared. PDES is version0.0.0 and
catalogued draft-only; target release stage is not accepted. Before beta-or-later,
reassess migration/release notes, package alignment and all promised support graphs.

## Evidence and next serial bindings

PDES reviewer inspected design sections1/2, actual crate seams, protected inventory,
Track25 owner contract and template; identified the exact-root mismatch and required
signature/type gaps. This assessment closes the ambiguity in preparation but not
Track25 acceptance or the interface freeze.

Next: separately reserve reviewed Track25 governance alignment; resolve complete
types/signatures/features and all22 visibility; obtain pending material schema
human disposition before adopting that amendment; independently bind schema/goldens,
backend/provider and dependency graph; joint exact freeze review; only then serial
scaffold followed by approved disjoint engine/codec leaves. Release and live runtime
acceptance follow implementation. No checklist/status/pin mutation follows here.

## Verified source bindings

- `docs/distributed/external-accounting-v1.md` SHA256 `a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5`.
- `docs/design/api-review-template.md` SHA256 `30471924c8d71da2e7ebec19fb8bfb2e1365ec31adecd6d590df3281e4e29f50`.
- `docs/design/protected-surface-inventory.json` SHA256 `1f8acfc40665fe08230a71b2f656be64048811bbd548f0c76ae5c69efce62694`.
- `docs/design/compatibility-matrix.md` SHA256 `8d669f95ef097ef5aee5de0309193491633f296169fd61bf060868b42b737b52`.
- `conductor/api-design-review.md` SHA256 `33d3263b469aab8867ddcd78959b88003f9ed198bc19e102797fc577787a64e4`.
- `conductor/tracks/25-api-design-review-compatibility-governance/agent-contract.md` SHA256 `8b0a2ee29d92db53676554d21f34c5072d1dc3735a4b74ef62709aff109ad607`.
- `conductor/package-catalog.md` SHA256 `d4b6f8a2d4f83c7f9eb8d75c8afdbdc4a45b310db55a5514e3dc75a85525f4e6`.
- `conductor/package-matrix.md` SHA256 `74c0bc501f1d8d4c4de40082fa0e0608317aaff320a24ae309a7c1d29268d500`.
- `crates/kairo-ecs-pdes/Cargo.toml` SHA256 `75acfa36b9deed08213aa8658648c3f87234a974b6aceb0a18bc0e2677174329`.
- `crates/kairo-ecs-pdes/src/lib.rs` SHA256 `b7ba58cf4844181c008175dc2371c0ce103d8ce5373fe6648acddf114394e68e`.
