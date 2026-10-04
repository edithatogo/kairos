# C1 physical calibration schema v2

Status: experimental physical v2 candidate, pending exact Python/Rust IPC file, IPC stream and Parquet qualification. This supersedes the rejected v1 map layout; no stable release or clinical-feed acceptance.

## Compatibility decision

The v1 PyArrow MapType used a default internal `entries` field. Independent PyArrow25.0.1 Parquet readback renamed it `raw_time_values`, so exact schema equality with metadata failed. Type-only equality is insufficient. Version2 explicitly changes the physical map encoding; logical C0 calibration-v1 and its SHA remain unchanged. Admission requires metadata physical_version=2 and the full exact schema. Version1 files are rejected; no silent coercion or compatibility claim. A separately reviewed migration can validate/reconstruct C0 records using the retained v1 codec at commit23c0995, then encode v2. No v1 fixture collection was accepted.

## Frozen layout

Retain all v1 named columns, nullability, metadata and scalar encodings except `raw_time_values`. Its C0 logical string-to-nullable-string map becomes a non-null list with non-null `element` structs, each with non-null UTF8 `key` and nullable UTF8 `value`. Metadata declares encoding=sorted_unique_entries_v2. Keys are sorted and unique; empty maps are empty lists, null values stay null. Null/missing/extra entry fields, duplicate/unsorted keys and wrong scalar types reject. The list element name is explicitly `element` for compliant Parquet nesting. It is a typed map representation, without JSON or opaque serialized schema bytes.

UTC retains exact RFC3339 text and signed_i128_le fixed16 bytes with ns_since_unix_epoch; ticks use unsigned_u128_le fixed16 bytes with1ns. Calendar text supports Gregorian0001..9999 separately from full integer byte codecs. Arbitrary event ranks remain canonical decimal UTF8, source_order UInt64, occurrence UInt32. Optional absence markers distinguish absent/null/empty; raw_event.source_fields alone is canonical JSON. Knowledge status and every nested clock lineage status have separate enums.

## Proof and boundary

Regression controls must fail against the old v1 map layout and pass exact schema+metadata and logical nullable-map payload readback through IPC file, IPC stream and Parquet. Independent Rust transport reads/writes the actual C0-validated mapper collection, followed by independent Python readback. Fixed layout/schema metadata are declared before encoding; no inference, casts, dropped fields, metadata relaxation or expected-as-actual inputs. Full logical C0 and per-request count validation remain required before/after transport. The fixture collection combines distinct synthetic datasets with explicit request membership; it is not one ingestion dataset. Classified DST controls do not claim a timezone resolver. C1.1 acceptance requires recorded runtime evidence; C1.2/C1.3 and full C1 phase remain separate.
