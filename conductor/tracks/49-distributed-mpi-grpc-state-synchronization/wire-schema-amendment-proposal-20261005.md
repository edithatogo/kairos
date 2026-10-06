# Decision proposal: complete the external v1 signed wire schema

Status: prepared for joint review and fresh human disposition. Not adopted. Full freeze, all22 golden generation and executable production dispatch remain HOLD.

Base bdfc2638837c93826bfe800cc44686d64cd873a2. Accepted semantic design SHA a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5 remains historical, unchanged. The wire fields below are proposed amendments, not assertions that the old layout already contains them. No production source, manifest/lock, parent pin, global status or parallel calibration path changes.

## Concrete problem and counterexample

Original Positive under sessionA/generation4/configA uses channel sequence11. Its Cancellation must be captured as sequence12. The reserved successor Positive uses13. The old Cancellation layout embeds the predecessor envelope including11 but has no separate anti position. Relabelling11 as12 changes the signed preimage/digest and no longer preserves the original record. Structural event equality excluding routing does not remove routing from authenticated bytes.

A reserved successor descriptor and TransitionId also do not uniquely bind its complete Positive preimage: common prefix, channel sequence and full ancestor list are needed. Deriving those from a later cancellation after restart to sessionB/generation5/configB changes the original digest. Treating original generation4 bytes as ordinary current traffic weakens fencing. Independent distributed/security and PDES schema audits identified these as generator blockers, not failing implementation tests.

Local counterexample.py executes SHA-256 over diagnostic prefix/sequence fragments and confirms changed channel position or activation changes the digest. These fragments are explicitly not complete canonical records, signatures or runtime evidence. It supports the concrete binding example; the contract/source audits establish the missing fields.

## Recommended amendment

Retain exact predecessor bytes/digest. Give the Cancellation its own current authenticated traffic position. Reserve a complete successor Positive preimage rather than guessing it from a descriptor. Preserve historical reservations after restart; existing RecoveryAuthorization remains the only publication route for retained original traffic under current fenced ownership.

Proposed precise reusable productions:

- RecordDigest32 is SHA-256 of literal KAIROS-EXT-V1, record-kind u8 and complete canonical prefix/body, excluding the trailing signature. It is distinct from FactDigest and any checkpoint/state digest.
- SignedRecordRef = signed byte length u32, complete original signed record bytes, RecordDigest32. The nested record includes its final64-byte signature. Validate expected kind, exact digest, signature, original trusted manifest and historical issuer authorization. Length bounds apply to the aggregate outer record.
- ChannelPosition = source participant Id16, destination participant Id16, sequence u64. Sequence is nonzero and uniquely reserved in its channel namespace. Source session/generation are bound by the current record prefix; destination activation and source/destination Owner tuples are bound by its exact trusted manifest. Channel cut namespaces must explicitly retain those activation tuples rather than conflate restarted sequence counters.
- PlannedPositive = preimage byte length u32, complete original canonical Positive prefix/body, RecordDigest32. It excludes only the signature, includes original activation/config/key ID, full EnvelopeFields, reserved channel position, TransitionId and sorted complete ancestor closure. Validate kind1 and digest before reservation. It is data, never a VerifiedDelivery or an issued live proof.
- MaterializedPositive = the exact PlannedPositive preimage plus a valid64-byte signature from its original authorized key. Signing follows the durable reservation commit. Signing/key unavailability leaves receipt pending/recovery required; it cannot rewrite prefix, move to a new key or release an unproven successor.

Proposed Cancellation body replaces the ambiguous predecessor/successor descriptor production with: SignedRecordRef(expected Positive), current source Owner, current destination Owner, own ChannelPosition, cancellation TransitionId, successor tag0 Absent or tag1 Present followed by PlannedPositive, sorted ancestor TransitionId[]. The original predecessor descriptor is decoded from its retained Positive. The current issuer/traffic owners are not obtained by rewriting historical event authority. No new Positive body layout is proposed by this amendment.

Present successor ancestry includes this cancellation TransitionId and the exact inherited ancestor closure. It does not include this Cancellation's record digest, avoiding a new hash cycle. Current transition and future channel/transition counters are reserved durably, checked for overflow and never allocated again on identical retry. No precommit signing, publication or model effect is authorized.

AppliedRetirement's tombstone field becomes the complete predecessor EnvelopeFields decoded from its exact signed Positive reference, with historical routing bytes retained as conflict evidence. Structural identity is explicitly sourceLP, scoped namespace/ownership epoch, full LogicalEventId and incarnation. Tick, destination, payload/schema, emission/fences and other complete descriptor bytes remain bound for conflict rejection; smaller identity is not permission to ignore different envelope data.

Release binds the exact materialized successor SignedRecordRef, its original TransitionId and complete applied-retirement FactRef closure. It cannot authorize a rewritten successor or skip the historical/current-owner RecoveryAuthorization checks after restart.

## Clarifications accompanying the amendment

Unsigned initial RootInput is sourceLP u32, root sequence u64, destinationLP u32, tick u128, payload schemaId16/versionu32, payload lengthu32 and payload bytes. Initial input-set preimage is KAIROS-INPUTS-V1 plus a u32 count and ordered complete RootInput values, sorted by (sourceLP,root sequence), rejecting duplicate root keys. Exclude ConfigDigest, signed envelopes and allocated transport/activation metadata to prevent a manifest/input digest cycle. Incarnation/transition allocation remains a separate deterministic reservation policy to bind against native source behavior; this proposal does not change its starting values or authorize overlapping seed sets.

Define count-prefixed references consistently: descendants/ancestor transitions sort by complete TransitionId; FactRef arrays by FactId; Member/PrepareRef/ManifestPrepare references by participant; Owner arrays by LP; Seed refs by their complete durable FactId. Duplicates by those keys reject even when bytes/signatures differ. InputClosed binds a counted SeedRef array and exact nonoverlapping coverage of the immutable input set. Every reference fixes expected kind and whether its digest is RecordDigest or FactDigest.

Manifest arrays explicitly have u32 counts. EntityId is indexu64/generationu32. Membership sorts by participant, owner/segment/topology by LP, entities by (index,generation), neighbors by LP. Decoder rejects duplicate/inconsistent keys and count/length arithmetic overflow before allocation.

Own FactDigest omission means exactly the32-byte own top-level DurableRef slot, determined by the parsed schema. Retain all nested prior fact digests; do not locate the slot by searching for a byte pattern. Signature and RecordDigest use section8's exact domain, kind byte and prefix/body (kind occurs in both domain framing and common prefix).

These array/manifest/fact-slot bindings require a complete machine-readable22-kind grammar and independent review before a generator is authoritative. Channel-set digest domain/ordered preimage and activation namespace, checkpoint/suffix/after-state section schemas and nested-record resource limits also remain explicit holds. Wire fixtures with labelled opaque state digests cannot prove underlying checkpoint or state canonicalization.

## Authorization and unresolved validation gates

The renewal ADR states: “Material departure in the eventual joint freeze requires fresh human disposition; merely binding a new hash cannot broaden approval.” This changes signed Cancellation/reference fields and therefore requires that disposition. Approval of this proposal would authorize updating the draft schema and completing its joint freeze/golden work within this precise amendment scope; it would not approve production dependency adoption, release, global statuses or bypass unmet gates.

Cross-activation cancellation authorization must be explicitly reviewed before schema acceptance: a current issuer cannot treat an old predecessor fence as current authority. Require complete historical manifest/activation proof and current ownership/fencing; migration must bind its exact committed routing/ownership decision. Until the specific validator rules are accepted, such traffic stays rejected/recovery required. This proposal does not invent a cross-activation native-cohort bridge or weaken existing authority/equality/closure rules.

After fresh human disposition: bind final Channel/cancellation/current-versus-historical authority rules and all22 schemas; independently accept exact new contract hash; generate complete reference bytes/signatures with public fixture keys and an independent encoder; cross-check from Rust before production codec acceptance. Storage/commit-uncertainty/provider/platform/API gates continue independently. No old golden or accepted source is overwritten to make a test pass.

Alternative: retain the old draft unchanged and keep all22 golden/full freeze blocked. Relabelling a predecessor's channel sequence or deriving historical successor bytes from new activation fields is not an acceptable implementation shortcut.

## Local evidence

| Artifact | SHA-256 |
| --- | --- |
| counterexample.py | ff808acc99a478f57e48425e773585543ddeb7d19040d677ac1eb9248ea31384 |
| counterexample.json | ed646ecf98025f2c128d1f0fc282d4e9dcd11923f335d64509c5a7b0dc30ba18 |
| counterexample.log | ed646ecf98025f2c128d1f0fc282d4e9dcd11923f335d64509c5a7b0dc30ba18 |
