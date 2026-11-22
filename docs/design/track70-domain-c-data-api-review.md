# API Review: trusted declared-domain C Data

## Affected Surface

- Affected roots: crates/kairo-ecs-arrow; crates/kairo-ecs-ffi.
- Surface family: rust_api; standard Arrow C Data interchange.
- Current status/release stage: additive alpha preview prototype.
- Related track: Track70 bounded domain followup; CareOps R13/R14 dependency.

## Proposed Change

Arrow arrow_c_data::domain provides export_batch/export_stream taking native
DataFrame, explicit Arrow Struct field and optional requested exact schema.
Private owned holders transfer raw ABI only by consuming into_raw. FFI
native_tables provides safe trusted-batch import and trusted-stream iterator.
Only pinned Boolean/integer/float/String physical types are supported initially.
No C symbol, existing field, host API, root rename or simulation behavior changes.

## Compatibility Classification

| Question | Answer |
|---|---|
| Additive only | yes |
| Existing public semantics altered | no |
| Protected root renamed/split/removed | no |
| Event ordering/replay/fixture changed | no |
| Existing C ABI symbols/ownership changed | no |
| Existing Arrow fields retyped/removed | no |
| Downstream source/pin adoption needed | yes, opt-in preview API |

## Required Evidence

ADR: docs/adr/0007-trusted-domain-c-data.md. Compatibility/migration: existing
consumers keep event-log exports; arbitrary domain consumers opt into the typed
preview and retain native-data feature isolation. No new package/catalog root.
Test fixtures: concrete non14-column native frame with nulls, exact full-width
limbs and generation fields; exact negotiation/required-null/type refusal;
source mutation/drop, empty/multibatch/partial-consumption and release behavior.
No release note claiming availability before immutable qualification.

## Memory, errors and concurrency

Only validated safe constructors create opaque holders. Raw external imports
stay unsafe; a safe consumer cannot accept or reconstruct an arbitrary ABI
pointer. Import owns the original release callback through the pinned library;
failed validation drops owned values normally, without custom callback mutation.
Unsupported schema/type/nulls reject typed ArrowError before export. No automatic
schema coercion. Thread safety follows owned Arrow holders and pinned types;
no global environment changes or runtime initialization in these APIs.

## Red-team objections and response

- Forged raw pointers through safe import: private holders, no from_raw, safe
  import accepts only original opaque owned values.
- Schema-only/fake capsules: require populated decoded values in direct tests.
- Released/dangling engine buffers: own exports, mutate/drop source before import.
- Overclaim of ABI/zero-copy compatibility: actual pinned Arrow implementation,
  explicit format/type scope and copy notes, unchanged external C obligations.
- Unbounded queued stream: lazy windows; no collection hidden inside the API.
- Domain null/ID semantics inferred: caller schema owns requiredness, domain
  validation remains in CareOps; no fabricated IDs, ticks or missing records.

## Release Decision

Needs independent prototype/safety review and actual native receipts. Historical
Track70 Done remains unchanged. No stable/beta, hosted, package publication or
parent acceptance is inferred from this review proposal.

## Prototype disposition — 3 November 2026

Independent review of the exact claimed source found no actionable safety or
compatibility issue. Private opaque holders, trusted-only imports, lazy windows,
error/early-drop ownership and explicit consuming ABI transfers are sound as
written. The qualified pinned DataFrame and BooleanArray type reexports are
additive, without a new package, generic engine or dependency. Local133native
package tests plus29default FFI tests pass; exact10domain and3schema controls,
all-target warnings-denied Clippy, warnings-denied docs and formatting pass.
Accept the local alpha prototype; stable/beta/release and parent adoption remain
held for actual hosted, package and downstream evidence. Historical phase
receipts remain scoped to their original event-log implementation.
