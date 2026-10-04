# C1 physical calibration schema v1

Status: coordinator-frozen experimental schema for synthetic C1.1 fixtures after layout review and ten focused codec tests on CPython3.14.8/PyArrow25.0.1. Not stable-release, local-feed or C1 phase acceptance.

## Decision

Define three separate Arrow tables keyed by the C0 logical record families:
`trace_event.v1`, `trace_exclusion.v1`, and `outcome_observation.v1`. The complete C0 logical fields remain represented by named columns and nested structs. The transport contract is the caller-supplied-schema RecordBatch interface in `c1-recordbatch-io-v1.md`; this proposal adds no Rust API or dependency and does not establish a consumer, mapper, or clinical-feed gate.

The schema and field metadata identify `careops.calibration.physical`, version `1`, the C0 logical schema name/version and SHA-256, table record type, and each logical field path. The `source_fields_json` helper remains inside its `raw_event` struct; the row-level `presence_fields` list records absent optional paths (including nested paths), distinguishing absence from explicit null and empty values. Every fixed-size clock field carries explicit per-field encoding metadata: `signed_i128_le` plus `ns_since_unix_epoch` for UTC, and `unsigned_u128_le` plus `1ns` for ticks. Nullable Arrow fields represent logical null. Optional C0 properties that are non-null when present (such as `event_kind_rank`, `quality_flags`, and `tick_resolution`) use a nullable Arrow slot only to encode absence; an unmarked null is rejected. Required logical fields cannot be absent even when their value schema permits explicit null. Decoding rejects unknown paths, duplicates, and contradictions between the presence list and physical values.

## Physical encodings

- C0 `time_value` becomes a struct retaining `raw`, `utc` as exact `utc_text`, source offset/zone, relative ticks, tick resolution, source precision, and lineage. `utc_i128_le` is a non-null fixed-size 16-byte little-endian signed Unix-epoch nanosecond value computed from the offset-bearing UTC text. The original UTC text is authoritative for exact round-trip; the integer is a lossless range-preserving indexed representation. Invalid, naive, unknown `-00:00` offset, or sub-nanosecond values reject; zero digits beyond nanosecond precision are accepted and preserved in `utc_text`. The logical RFC3339 date-time range remains year 0001 through 9999, while the byte codec supports the complete signed i128 range. No Arrow timestamp type is used, avoiding narrower timestamp ranges and unit truncation.
- C0 `u128_decimal` fields become fixed-size 16-byte little-endian unsigned values, range checked to `[0, 2^128-1]`. `event_kind_rank` remains arbitrary-precision nonnegative decimal UTF-8. `source_order` and `occurrence` use uint64 and uint32 respectively.
- `raw_event.source_fields` alone uses canonical JSON UTF-8 (`ensure_ascii=False`, sorted keys, compact separators, finite numbers only); its exact logical object is reconstructed on decode. Named `raw_time_values` is a map from UTF-8 names to nullable UTF-8 values, never JSON. All remaining strings, enums, booleans, lists, structs, and nulls use corresponding Arrow logical types.
- Optional keys are omitted on decode when marked absent. `presence_fields` uses sorted unique dotted logical paths, with array/map contents treated as values rather than key-presence paths.

## Compatibility and migration

This is a new versioned physical format; it does not alter C0 JSON, the legacy event-log schema/fingerprint, or transport behavior. Readers must require exact schema equality and metadata under the transport contract. Writers must use the declared schemas, never infer them from data. List elements are non-null UTF-8; null/absent lists follow each logical field contract. A future incompatible field/type/encoding change requires a new physical schema version and explicit migration; readers must not silently coerce or drop fields. UTC text remains available to older consumers that cannot decode i128. The physical encoding is independent of Parquet/IPC choice.

## Red-team review and response

A plausible objection is that storing both UTC text and an integer permits disagreement, and that presence paths or canonical JSON can be corrupted to produce misleading reconstructions. The codec therefore derives the integer only from validated offset-bearing text, verifies it against the text on read, preserves the text byte-for-byte, and rejects mismatch. Presence metadata is row data, canonicalized in sorted order, and checked for duplicates, unknown paths, required-field absence, and absent/non-null contradictions; duplicate keys in dynamic raw-time maps reject. Canonical JSON is restricted to the intentionally open-ended source-fields object; named fields retain typed columns. Schema fingerprints alone do not replace checking the full schema and metadata. External reader/writer parity and hostile-file resource qualification remain independent gates.

## Review boundary

Logical C0 schema and dataset semantic validation remain required before admission and after readback; this codec does not replace chronology, occupancy, count conservation or leakage checks. Schema/list nullability and codec presence checks preserve valid C0 rows. Clock codecs reject unknown offsets and nonzero sub-nanosecond digits; years0001–9999 define calendar text range independently of full-width physical integer boundary tests. C1.1 remains open until actual mapper outputs and independent IPC/Parquet reader/writer parity satisfy all parent joins.
