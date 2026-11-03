# ADR9: Shared trusted-domain C Data import boundary

## Status

Reviewed architecture with actual bounded local native/default, strict lint/docs
and policy qualification. Final frozen source review, hosted, clean Git consumer
and parent adoption acceptance remain pending.
ADR7 and its original source receipts remain immutable historical evidence. This
ADR changes only the owning module/package of the trusted import implementation.

## Context

The CareOps private populated ED, calibration and B7 consumer passes actual C Data
and C Stream checks. Adding the existing engine FFI crate solely for these safe
imports also acquires a distinct Kairos engine/core/RNG closure. The unchanged
parent dependency policy rejects duplicate core/RNG sources. An exemption or a
new runtime promotion would not address that coupling. Changing old no-default
engine behavior or Arrow's unsafe prohibition would break existing contracts.

## Decision

Move the existing trusted native_tables implementation once, byte-for-byte, into
kairo-ecs-arrow-ffi. The new crate is the narrow audited unsafe import boundary;
its native-data feature activates the same pinned Arrow producer and ABI types.
Default builds have no normal dependencies. The existing engine FFI crate
reexports the same import_batch, import_stream and TableStreamReader APIs under
its existing native-data feature. Neither its mandatory engine dependencies nor
its default/no-default behavior changes. Arrow and workspace unsafe prohibitions
remain unchanged. No new unsafe algorithm, raw constructor or callback shim is
introduced. The original four unsafe blocks and safety proofs remain canonical.

The two actual consumers are the current engine FFI interface and the CareOps
native-domain acceptance lane. This extracts one demonstrated shared operation;
it creates no query framework, engine abstraction or data-model wrapper.

## Ownership and security

Only original opaque OwnedBatch and OwnedStream values enter safe imports. They
have private fields, no Clone/from_raw/mutable holder API, and consuming ownership
transfer. Imported arrays retain the pinned library's original callback lifetime.
Arbitrary foreign pointers remain subject to ordinary unsafe C Data obligations.
The new crate denies unsafe code except in the one exact audited module. A closed
source/feature guard and actual normal-graph checks reject a widened boundary,
changed importer, removed Arrow ban or unconditional data/engine dependencies.

Track70's existing Arrow/FFI/conformance coordinator owns the exact bounded source
handoff, including new crate/workspace registration and existing FFI reexport.
The fixture-only kairo-ecs-types dev dependency constructs EventId values; it
adds no normal dependency or public domain-type exposure. Source retains existing
upstream licence terms and publish=false. No registry publication is qualified.

## Compatibility and consequences

Existing ffi::native_tables Rust source paths, function signatures, returned
schema/error types and safe ownership semantics remain available by direct
reexport. TableStreamReader's defining Rust module/type_name moves to the shared
crate. No stable Rust ABI or type-name-string promise is made for this alpha
preview. C symbols, headers, engine handles, schema versions and host bindings
are unchanged. Polars/native features remain absent from minimal core/browser
normal graphs; the new crate's default normal graph is dependency-free.

Parent adoption uses a separately reviewed default-off test feature and exact
accepted new data-source revision, then reruns actual dependency policy and final
26-case compatibility. No upstream implementation or old private pass alone
accepts the new parent closure. No zero-copy, every-column, clinical or release
claim follows. Roll back source/pin with a fresh output directory; preserve old
artifacts and their declared source/schema identities.
