# C1 RecordBatch transport contract v1 — partial capability

Separate optional kairo-ecs-arrow-io package, own Rust1.88 floor, default features
empty. IPC and Parquet features use exact Arrow/Parquet60.0.0 components; no
compression/async/CSV/JSON codecs. Legacy kairo-ecs-arrow Rust1.76 schema/fingerprint
and custom smoke bytes remain unchanged. Member status does not claim whole
workspace Rust1.76: selected legacy packages must still qualify explicitly.

## Transport API

Under either feature export arrow_array::RecordBatch and arrow_schema::{Schema,
SchemaRef}. IoLimits fields: max_input_bytes, max_output_bytes, max_batch_rows,
max_total_rows, max_batches, max_columns (all usize). Constructor/calls reject zero
caps via IoError::InvalidLimits. IoError separates LimitExceeded(&'static str),
SchemaMismatch, malformed-format errors from upstream Arrow/Parquet and std IO.
No panic for ordinary invalid inputs. No default unbounded limits.

IPC: read_ipc_file and read_ipc_stream(input: &[u8], expected: SchemaRef,
limits: IoLimits) -> Result<Vec<RecordBatch>,IoError>; matching write_ipc_file and
write_ipc_stream(expected: SchemaRef,batches: &[RecordBatch],limits: IoLimits)
-> Result<Vec<u8>,IoError>. Parquet: same signatures read_parquet/write_parquet.
Format is explicit, no sniffing. Exact schema equality includes field order,
name/type/nullability/nested fields and schema/field metadata. Empty input batch
list must preserve the explicit schema in the encoded artifact. Null is distinct
from empty/zero. Reader rejects metadata/schema mismatch before accepting rows.

Validate encoded input size before decoder construction. Check schema column
count and decoded batch/total row count, batch count using checked sums. Writer
prevalidates all input batches and counts before serialization, uses a writer
that rejects exceeding max_output_bytes before allocating/appending beyond cap.
Readers must not collect an unbounded stream. Parquet returned decoder batches
need not preserve writer batch boundaries: batch size is max_batch_rows, and
max_batches applies to returned batches. Footer row counts are prechecked.
Parquet writer row-group size is max_batch_rows, making multiple row groups
explicit; IPC retains its original batch framing. Zero-row batches still count.
Schema column count is prechecked before decoder construction. Output cap IO
failures translate to LimitExceeded("max_output_bytes"); flush/finish failures
are propagated and no partial encoded bytes returned. These are encoded/output/consumed-row
caps, not proof of peak decoder allocation: hostile metadata/dictionary/pages may
allocate before returned-batch validation. Intended for trusted synthetic/local
artifacts; D4 must qualify process isolation and decoder memory limits before
untrusted cross-process input is accepted. No Arrow C Data pointers in this API.

## Schema and semantics ownership

Caller supplies the frozen expected schema. This generic transport never infers
clinical events or changes units. Existing event_log.v1 fields use 12-byte IDs
and16-byte little-endian u128 ticks. Separate calibration physical schemas,
UTC timestamp range policy, nested lineage/source-field encoding, source partial
orders, DST parsing, exclusions and trace manifests remain Track21/04 work.
Generic successful IO cannot close all C1 tasks or establish unknown local feeds.

## Qualification

Actual none/ipc/parquet/both builds/tests on own1.88/current; malformed/truncated
input, schema mismatch, zero caps and each configured limit. Multi-batch IPC and
multi-row-group uncompressed Parquet, empty artifact and null/empty/binary tick
extremes. Independent exact PyArrow25.0.1 writes synthetic fixtures read by Rust,
and reads Rust artifacts including metadata/types/values. Cross-language evidence
requires actual runs, not two Rust code paths. No clinical/private fixture data.
