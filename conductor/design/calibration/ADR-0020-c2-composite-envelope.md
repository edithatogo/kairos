# ADR-0020 — Complete C2 composite and durable envelope

Date2026-10-10. Experimental coordinator design under continuing C2 authority.
Extends ADR0017–19. efb3bac joins complete Flow/dispatch, transit, provider, fidelity
and seed/key/stream bytes. Its full Rust1.99 join is622/0/5. Complete bridge bytes,
coherent composite, durable file and fresh-process matrix remain required.

## Owner byte completion

Bound and Submitted bridge DTO bytes include every native field, exact sample and
stream state, graph configuration/identity, metadata/receipt, all ordered event and
control history, full rejected dispatch, arrival fields and acquisition. Keep
history order. Nested stream image encoding uses encode_stream_image; never
temporarily reset/replay the stream. DES owns full dispatch bytes. Route receipt
owner supplies complete bounded metadata/receipt bytes and validates canonical
geometry against the actual restored context; digests are not signatures.
Reject all lengths, aggregate counts/identifier/payload/wire limits, schemas,
tags, malformed UTF-8 and trailing data before owned decode allocations. Preserve
native semantic validation; no new public engine error/dependency/serde changes.

## Coherent composite and transaction

Capture Flow, fidelity, complete provider and seed registry, and every Bound and
Submitted item at one immutable coordinator cut. Multiple records and multiple
caller-approved graphs are supported. Trusted current model code supplies context
and template codecs, immutable configuration and per-work expected Service keys
and graph bindings; unknown registrations, owners or missing bindings reject.
Never let an artifact authorize its own expected key, graph or executable code.

The cut must be checked against actual Flow scheduler/command inventory and the
actual transit context, not a caller flag. A dispatch applied by Flow but not
observed by its bridge must reject capture. Correlate pending events, expected
event/due, acquisition, carrier owner/registration/kind, controls, retry dispatch
and arrival requests. Preserve legal paused stale events, popped rejected retry
events and arrived pending admissions through exact context/history evidence.
Pending control events still retained by the bridge must be live; after observation
they leave its controls list. Source generation/issued-ID checks bound historical
references but do not attest provenance. No simulated-prefix replay.

Decode a complete bounded section inventory into detached owner data. Compare the
trusted model binding before any owner decoder. Restore Flow privately once, use
its exact read-only view to restore fidelity and bridge records, restore provider
and full registry, then validate all cross-owner links/coherent frontier. Expose
one complete owned result only after every step succeeds. Failure drops the staged
new runtime and cannot mutate an existing source/destination. Rust permit-bearing
Prepared/Created stages remain within a coordinator operation; every completed
Bound/Submitted stage and all legal C2 mode/progress states remain supported.

## Envelope and local file publication

A versioned fixed envelope binds three caller-trusted 32-byte identities: model
code, immutable configuration (including graph/key bindings), and owner schema
inventory. Those identities are opaque compatibility contracts supplied by current
model code, not authority from the file. Include exact body length and SHA-256;
verify expected identities, schema, total size and whole-body digest before owner
decode. Hashes detect corruption; they are not authentication or clinical proof.

Read metadata/header under caps, require exact file length and no trailing bytes,
reserve fallibly only after complete length checks, then read/verify the body.
Local writing uses a same-directory exclusive temporary file, write/sync, and
atomic no-clobber publication. A check-then-rename race is not acceptable because
rename may overwrite another writer's target. Same-filesystem hard-link publication
is one bounded option; unsupported filesystem capability returns an explicit error,
never a partial-file fallback. Clean up the task's own temporary file, sync the
directory where supported, preserve existing targets and report unresolved durability.

## Required evidence

Independent owner reviews, exact command exits/log hashes and Rust1.99 only. Test
unobserved accepted/rejected/control cuts versus observed positives, all legal
Macro/Micro/zero/transit/policy/active/Suspend/Restart states, arbitrary registered
owned context/template payloads, identity/geometry/schema/limit corruption and
transactional rejection. Transport real owner bytes; no test-only payload global
or closure sidechannel may support complete-byte/process qualification.

Fresh-process restore must use this complete image and trusted model configuration,
then compare suffix events/IDs/resources/cursors/seed draws/receipts/results with
uninterrupted controls over the complete C2 matrix. A fixed sealed recipe cannot
replace the generic surface or wider required cases. File tests cover collision and
concurrent no-overwrite publication, partial/truncated/corrupt/wrong-binding artifacts,
caps before decode and real reload. New-head hosted checks remain separate proof;
C2.4, clinical, release and C5 operational gates are not cleared by this design.
