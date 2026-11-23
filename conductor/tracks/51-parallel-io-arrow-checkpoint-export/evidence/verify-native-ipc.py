"""Read the Rust native IPC fixture using a separately installed PyArrow."""

import hashlib
import sys
from pathlib import Path

import pyarrow as pa


def read_batch(path, stream, fields, expected):
    schema = pa.schema(fields, metadata={b"kairo.stream": stream.encode()})
    data = path.read_bytes()
    with pa.ipc.open_stream(data) as reader:
        assert reader.schema.equals(schema, check_metadata=True), reader.schema
        batches = list(reader)
    assert len(batches) == 1, len(batches)
    batches[0].validate(full=True)
    assert batches[0].to_pydict() == expected, batches[0].to_pydict()
    assert data.endswith(b"\xff\xff\xff\xff\0\0\0\0")
    print(path.name, len(data), batches[0].num_rows, hashlib.sha256(data).hexdigest())


def main(path):
    print(f"Python {sys.version.split()[0]}, PyArrow {pa.__version__}")
    u32 = pa.uint32()
    u64 = pa.uint64()
    utf8 = pa.large_string()
    fields = [
        ("schema_version", pa.uint16()),
        ("run_id", utf8),
        ("event_id_index", u64),
        ("event_id_gen", u32),
        ("entity_id_index", u64, True),
        ("entity_id_gen", u32, True),
        ("time_ticks_hi", u64),
        ("time_ticks_lo", u64),
        ("time_scale", utf8),
        ("priority", pa.int32()),
        ("sequence", u64),
        ("event_kind", utf8),
        ("status", utf8),
        ("payload_ref", utf8, True),
    ]
    read_batch(
        path / "events.arrows",
        "kairo_ecs.event_log.v2",
        [pa.field(*field, False) if len(field) == 2 else pa.field(*field) for field in fields],
        {
            "schema_version": [2, 2, 2],
            "run_id": ["run-箭"] * 3,
            "event_id_index": [1, 2, 2**64 - 1],
            "event_id_gen": [7, 7, 7],
            "entity_id_index": [None, 40, 50],
            "entity_id_gen": [None, 2**32 - 1, 2],
            "time_ticks_hi": [0, 2**63, 2**64 - 1],
            "time_ticks_lo": [5, 9, 2**64 - 3],
            "time_scale": ["ticks"] * 3,
            "priority": [-3, -3, -3],
            "sequence": [31, 32, 33],
            "event_kind": ["custom:17"] * 3,
            "status": ["dispatched", "cancelled", "dispatched"],
            "payload_ref": [None, "payload/é", None],
        },
    )
    read_batch(
        path / "world.arrows",
        "kairo_ecs.world_snapshot.v1",
        [
            pa.field("entity_id", u64, False),
            pa.field("generation", u32, False),
            pa.field("active", pa.bool_(), False),
        ],
        {
            "entity_id": [2**64 - 1, 7],
            "generation": [2**32 - 1, 3],
            "active": [True, False],
        },
    )

    header, descriptor, *_ = (path / "checkpoint.contract").read_text(encoding="utf-8").splitlines()
    assert header == "kairo-ecs-checkpoint-v1"
    checkpoint_id, file_format, run_id, fingerprint, rows, checksum, byte_len = descriptor.split("\t")
    assert (checkpoint_id, file_format, run_id, rows) == ("cp-1", "hdf5", "run-箭", "3")
    blocks = pa.struct([pa.field(name, u64, False) for name in ("offset_bytes", "byte_len", "row_count")])
    read_batch(
        path / "checkpoint.arrows",
        "kairo_ecs.checkpoint_metadata.v1",
        [
            pa.field("checkpoint_id", utf8, False),
            pa.field("format", utf8, False),
            pa.field("run_id", utf8, False),
            pa.field("event_schema_fingerprint", utf8, False),
            pa.field("event_row_count", u64, False),
            pa.field("manifest_checksum", u64, False),
            pa.field("blocks", pa.large_list(pa.field("item", blocks, False)), False),
        ],
        {
            "checkpoint_id": [checkpoint_id],
            "format": [file_format],
            "run_id": [run_id],
            "event_schema_fingerprint": [fingerprint],
            "event_row_count": [int(rows)],
            "manifest_checksum": [int(checksum)],
            "blocks": [[
                {"offset_bytes": 0, "byte_len": 128, "row_count": 1},
                {"offset_bytes": 128, "byte_len": int(byte_len) - 128, "row_count": 2},
            ]],
        },
    )


if __name__ == "__main__":
    main(Path(sys.argv[1]))
