# Flow checkpoint V4 direct wire contract

Status: reviewed native source stage; proposed bounded wire implementation contract, runtime acceptance pending. Root is Track03 scoped owner under full delivery authority; independent Sol review supplied framing/source analysis. Native final result d973b9d50bec8f6177f42abcea89671d20ba33f781caa949270fbdffac4eb6ad:403passed/0failed/1ignored,10nativeproofs. Accepted native source commit a178a46b2399c48b00755c13e8c209ad63dd5685. Durable parent archive conductor/evidence/e2.2-flow-v4-native-completion-20261012 is committed at parent61786408b7c85bc8336a15244eb1b4b56424f820, acceptanceSHA e07bc557dcae5157801e6325e91250befb3c6696b4bc0bd460825fcc20c4c181. Root reviewed this owner contract; wire implementation and runtime acceptance remain pending. Bind the commit containing this contract before source dispatch.

## Public surface and ownership

Add only FlowCheckpointV4::encode_wire_v4(&self,FlowCheckpointWireLimits)->Result<Vec<u8>,FlowCheckpointWireError> and decode_wire_v4(&[u8],FlowCheckpointWireLimits)->Result<Self,FlowCheckpointWireError>. They mirror current hidden experimental V3 direct APIs. No new limits/error type, C ABI/Arrow/root rename, native DTO alteration, Image/Ref variant, automatic version selection, runtime callback export or legacy conversion. Existing callers require no edits. Source ownership exactly crates/kairo-ecs-des/src/flow/checkpoint/wire.rs and new crates/kairo-ecs-des/tests/flow_checkpoint_wire_v4.rs. Inline private scanner allocation instrumentation belongs in owned wire.rs. Existing native helpers remain read-only.

## Bytes and canonical order

Eight-byte magic KFLOWV4 followed by NUL; u16 little-endian wire schema1; u32 native version4. Preserve all V1/V2/V3 framing and bytes unchanged. After header, write the exact existing V3 body field order and representations, from config/callback/batch/budget through scheduler/world, identities/registrations/stores/commands/notifications/pending operations/counters/grant flags/frontier/allocation_modes. Use existing V3 command tags. Append queue_order_modes then request_queue_orders.

Each vector has checked u64 count. Mode row: full ResourceId(u64index,u32generation), u8 mode0AdmissionOrder/1StableKey. Request row: full RequestId, u8 order0AdmissionOrder/1StableKey; tag1 has u64 length plus opaque bytes1..128. Rows strictly ascend by full ID; duplicate key VALUES are valid. Exactly one mode per live resource and one order per retained request; removed-resource terminal groups preserve full generation and consistent mode. No derived sidecars serialized. Native version/structural join validation remains authoritative; direct wire owns no history authenticity.

## Limits, validation and allocation boundary

Use existing FlowCheckpointWireLimits(flow,max_wire_bytes,max_total_records). New mode/request rows count toward existing resource/request bounds and aggregate total records. Stable byte totals share max_key_bytes with serialized registration/component strings using checked arithmetic. max_payload_bytes remains caller codec payload, unchanged. Existing capture and restore key-accounting minima differ (runtime registry versus repeated DTO strings); wire checks its serialized DTO strings, without claiming equal capture/restore minima.

Encoder: check native version; count/key/payload bounds; checked full byte measurement; canonical order/structural coverage validation; one fallible exact output allocation; write. Decoder: original whole-wire cap/header/schema; complete borrowed scanner checks all counts, per-key1..128, aggregate records/keys/payload, primitive tags/bools/options and checked integer conversions, canonical row ordering and trailing bytes; only then owned DTO allocation/decode; validate complete DTO canonical/structural joins. Full native restore then validates runtime cross-joins before caller codecs under accepted native policy. No caller codec is called by direct wire encode/decode. Bounded temporary join maps after count caps are allowed; do not claim all joins before any bounded allocation. Invalid final metadata or trailing data must reject before owned DTO collections.

Error taxonomy follows existing wire: UnsupportedSchema(u16), InvalidTag, InvalidBoolean, InvalidUtf8, IntegerOverflow, Truncated, TrailingBytes, LimitExceeded(label), AllocationFailed, InvalidValue(label). Freeze exact new labels/first-error ordering in source packet before execution; no silent fallback to older format. Bad magic uses established InvalidValue convention; wrong native version established Flow checkpoint version error. Structural validation cannot recover fully erased removed-resource history.

## Exact required proof names and oracles

1. v4_wire_actual_mixed_runtime_roundtrip_continues: actual native capture with both resource modes, Pending/Queued/Active/Suspended and duplicate keys; direct encode/decode DTO equality, real restored continuation dispatch/progress/context/fullimage equality.
2. v4_wire_removed_generation_terminal_rows_roundtrip: actual removed resource terminalkeys, later reused slot generation isolated; exactmetadata/accessor roundtrip and disagreement rejection.
3. v4_wire_literal_golden_and_legacy_bytes_unchanged: new literal V4 expected bytes/hash from independently reviewed field inventory; unchanged existing V1/V2/V3 literal sourcegoldens remain passing. Candidate encoder output alone is not expected golden.
4. v4_wire_exact_byte_record_key_limits: exact totalwire/records/resource/request/key cap success and onebelow failure; aggregate strings+128key, payloadcaps unchanged; no guessedmultipliers.
5. v4_wire_checked_lengths_and_integer_overflow: checked measurement/count/key sums and u64→usize boundary controls, no oversized allocation.
6. v4_wire_header_tags_and_truncation_matrix: unknownmagic/schema/nativeversion; every new mode/order tag; zero/129key; bool/option and primitive truncation, each isolated positive baseline.
7. v4_wire_duplicate_missing_unsorted_metadata_rejected: duplicate/unknown/missing mode/request IDs, swapped rows, live mode/tag mismatch and inconsistent removedgroups; duplicate keyvalues accepted.
8. v4_wire_bad_final_row_and_trailing_precede_owned_decode: borrowed scanner proves malformed last row/trailing byte zero owned DTO decode allocations, preserving instrumentation counters and positivevalidbaseline.
9. v4_wire_measurement_precedes_output_allocation: exactlimit/truncated/count/measurement failure zero outputallocation, valid oneallocation; no count-driven copies beforecaps.
10. v4_wire_direct_restore_has_no_legacy_fallback: keyed image decodes V4 and native restores actualeffectiveorder; olddirectcodec rejects V4 magic, V4 limit/error remains visible; no Image/Ref routing added.

## Gate budget and acceptance

Format first. Three Rust/Cargo1.99 gate commands: strict DES all-target Clippy, full DESpackage tests with all ten exactnames and unchanged legacy tests, two-owned-path format. Maximum three failed DEVELOPMENT executions; stop eachfailure for root review, retain all started/result/rawlogs and no counterreset. Freeze sourcehashes/pairedreceipt chronology/currenthead and source result before independent acceptance/commit. Source-stage success is directwire only. Clean joined four-package qualification, automatic routing, version-aware consumers/26processcases/4V3traces/19outergoldens, parentpin and ED remain separate gates.

## Compatibility and red-team response

Additive hidden Rust alpha/experimental method at crates/kairo-ecs-des; no C ABI include/kairo_ecs.h or Arrow schemas/arrow/event_log_v1.schema.json change. Copied DTO/result bytes; no dispatch/replay or scheduler change. ReturnVec allocation is explicit bounded/fallible. Malicious counts/keys reject before ownership; canonicalmetadata preserves authoritative native semantics without exporting EDpolicy. Shared fixturecatalog/beta/stable/release approvals are separate and not claimed.
