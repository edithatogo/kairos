# Track70 native file metadata followup

Base d835c525. Existing sole Arrow/FFI integration chat, standard Git worktree and
separate parent build-target reservation remain unchanged. No shared manifests,
locks, runtime, scheduler, bindings or global registry writes. Prior phases and
C Data receipts stay scoped to their actual immutable sources.

CareOps B7 uses RunOutput's protected durable Parquet publisher plus IPC. Current
writers accept no schema metadata. Rewriting emitted bytes inside a sink would
invalidate the upstream part receipt; replacing the publisher would duplicate
qualified durability. Add metadata to the original encoding before hashing.

## Bounded packet

1. Actual native baseline control requires metadata in existing plain Parquet/IPC
   outputs and demonstrates its absence; preserve exact red/positive controls.
2. Add write_parquet_zstd_with_metadata and write_ipc_stream_with_metadata with
   deterministic BTreeMap<String,String> input. Preserve plain helpers via empty
   metadata. Use pinned Polars metadata APIs before finish/start. Reject the
   reserved ARROW:schema key before output; caller keys cannot replace the
   library's physical Arrow schema. Duplicate input keys cannot exist in a map.
3. Add RunOutput::write_batch_with_metadata and write_batch_to_with_metadata;
   original methods delegate with empty metadata. Existing loop/order/partition,
   guarded synced staging, watermark/ack and receipt paths stay authoritative.
   Shared encode_part receives metadata before writing and hashing actual bytes.
   Sink errors record no receipt, preserving first-failure behavior.
4. Custom metadata is caller-supplied declaration, not certified domain meaning.
   Generic RunManifest is unchanged; consumers bind metadata to their authoritative
   domain manifest and actual columns/hashes. Per-part metadata may differ; no
   new generic domain schema or compatibility engine is introduced.
5. Real Parquet/Zstd and IPC typed values/nulls/custom metadata roundtrip; actual
   durable part receipts hash metadata-bearing bytes. Empty metadata is byte-equal
   to old helpers. Reserved-key and failed sink controls refuse explicitly.
   Independent PyArrow reads explicitly emitted owned disposable fixtures.
6. Run affected native durability/publication targets, warnings-denied Clippy/
   docs and review. Commit only this packet, then obtain actual hosted/package
   and downstream receipts before parent adoption. No final acceptance inferred.

Native build uses the separately claimed terminal parent target, explicit
CARGO_TARGET_DIR; source and both lockfiles stay separate. Python advisory helper
uses the already recorded user-selected3.15.1 environment; upstream canonical
runtime policy remains untouched and unqualified by this local helper use.

## Local engineering qualification

Actual baseline3controls:1plain native value pass and2method failures for absent
Parquet/IPC metadata. Preserved baseline source/log is not an API compile-failure
substitute. Prototype then passes9focused native controls. Final complete Arrow
polars-native package100pass,0failed/ignored, including unchanged durable staging,
symlink, canonical ordering, checkpoints and recovery controls. DataFrame/FFI
code is unchanged from the separately reviewed d835c525 source.

The optional metadata contract has a checked64KiB aggregate key/value+eight-byte
entry framing budget; it is not an encoded-part/RSS bound. Empty keys and reserved
ARROW:schema refuse before output/state mutation, including zero-row calls.
Existing upstream RunOutput bounds rows, not encoded bytes; no new claim of a
byte cap is made. Tests derive a sink limit from actual plain encoded output:
metadata-bearing Parquet/IPC exceeds that same limit and propagates a real write
error. Actual partial StorageFull sink emits128bytes and records no receipt or
watermark; a later explicit test operation succeeds with matching receipt.
This is an injected writer failure, not physical disk-exhaustion proof.

Real file receipt checks use an independent standard FNV-1a64 oracle with the
known hello vector, rather than the private implementation function. The first
new test attempted that private function and failed compilation; it is retained
as a compile error, not method-red evidence. An overbroad import edit produced
unused imports; these were removed without allowances. Plain empty-metadata
helpers produce byte-identical output to direct original pinned Polars writers.
Typed zero-row Parquet/IPC schemas and actual file metadata remain readable.

Independent Python3.15.1/PyArrow26 consumes the explicitly emitted fresh disposable
fixture:2populated Parquet rows,2IPC rows, fullUInt64/null/Float64 and Unicode/
quoted multiline strings, real Zstd, declared metadata and2durable event-log
parts whose receipt hashes attest metadata-bearing bytes. Parquet provenance is
in its file footer; IPC provenance is in schema metadata. The first draft reader
wrongly expected Parquet table schema metadata; the second draft expected
lowercase complete instead of existing serialized RunStatus Completed. Both
failed verifier logs are retained; corrected format/contract-specific reads pass,
with no production or acceptance-gate change. File SHA256 receipts are retained
privately alongside explicitly labeled upstream noncryptographic FNV receipts.

All-target warnings-denied Arrow Clippy, warnings-denied rustdoc, fmt/diff pass.
Native commands use Rust1.99.1/Homebrew LLVM23.1.3, locked source, one build job,
no incremental/debug and configured Polars budget1. Independent source review
has no findings. Prepared/source/log SHA receipts are private polars-metadata-*.
No manifest/lock/runtime/schema/C header or dependency change. Accept this local
alpha engineering slice only; actual hosted, full-workspace/policy/clean-package
and exact parent metadata/consumer acceptance remain open. Do not promote pins
or relabel final migration/release/clinical gates as complete.
