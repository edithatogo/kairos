# Native Arrow IPC streams

Enable `parallel-io` and `polars-native` in `kairo-ecs-arrow`. The
`native_ipc::NativeIpcBatch` constructors build owned Arrow batches from
validated event records, entity snapshot rows or a checkpoint manifest.
`write_contiguous` writes a complete uncompressed IPC stream to a Rust
writer. `write_contiguous_to_path` stages and publishes it with the existing
checkpoint file publication routine. Use a trusted output directory and
one writer per destination. A Unix parent-directory sync error after the
rename may leave the new stream visible. Read back the destination before
retrying. Open the result with an IPC stream reader such as
`pyarrow.ipc.open_stream`.

Each schema carries the stream name as `kairo.stream` metadata.

| Stream | Fields |
| --- | --- |
| `kairo_ecs.event_log.v2` | The canonical 14 [event-log fields](../../schemas/arrow/event_log_v2.schema.json). The two unsigned tick limbs preserve all 128 bits. Entity index and generation are nullable together. Strings use Arrow `LargeUtf8`. |
| `kairo_ecs.world_snapshot.v1` | `entity_id` (`UInt64`), `generation` (`UInt32`), `active` (`Boolean`), all required. The output follows caller row order. |
| `kairo_ecs.checkpoint_metadata.v1` | One descriptor row. `checkpoint_id`, `format`, `run_id` and `event_schema_fingerprint` are `LargeUtf8`. `event_row_count` and `manifest_checksum` are `UInt64`. `blocks` is a `LargeList` of structs with `UInt64` fields `offset_bytes`, `byte_len` and `row_count`. All fields and list items are required. |

The write receipt's SHA-256 schema fingerprint covers the unframed IPC schema
message including its metadata. The FNV-1a checksum and encoded length cover
the bytes written through the end-of-stream marker. A nonempty batch reports
one block spanning the complete stream at offset zero relative to its start
in the supplied writer. An empty batch reports zero row blocks and still
writes the schema and terminator.

`ParallelIoRecordBatch::to_record_batch_bytes` and its block writers retain
the v1 text serialization used to build historical checkpoint manifests.
The block list inside the IPC metadata row refers to the source manifest.
Use the HDF5 or ADIOS2 reader to restore the corresponding native checkpoint.

The standalone PyArrow reader in the [Track 51 evidence directory](../../conductor/archive/51-parallel-io-arrow-checkpoint-export/evidence/verify-native-ipc.py)
checks field order, physical types, nullability, values and stream metadata.
Live filesystem performance requires a run with the target mount and storage
configuration. [Evidence requirements](../parallel-io/evidence-boundary.md#live-proof-required).
