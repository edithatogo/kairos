"""Check the paired Rust producer outputs with Apache PyArrow in an isolated directory."""
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq


def main():
    with pa.ipc.open_stream("events.ipc") as reader:
        ipc = reader.read_all()
    parquet = pq.ParquetFile("events.parquet")
    decoded = parquet.read()
    assert ipc.schema.names == decoded.schema.names
    for left, right in zip(ipc.schema, decoded.schema, strict=True):
        if left.name in {"run_id", "time_scale", "event_kind", "status", "payload_ref"}:
            assert left.type == pa.large_string() and right.type == pa.string_view()
        else:
            assert left.type == right.type
        assert ipc[left.name].to_pylist() == decoded[right.name].to_pylist()
    assert ipc.num_rows == 2
    ticks = [(hi << 64) | lo for hi, lo in zip(
        ipc["time_ticks_hi"].to_pylist(), ipc["time_ticks_lo"].to_pylist(), strict=True
    )]
    assert ticks == [0, (1 << 128) - 1]
    assert ipc["entity_id_index"].null_count == 2
    group = parquet.metadata.row_group(0)
    assert all(group.column(i).compression == "ZSTD" for i in range(group.num_columns))
    for filename, read in [("events.parquet", pq.read_table),
                           ("events.ipc", lambda data: pa.ipc.open_stream(data).read_all())]:
        raw = Path(filename).read_bytes()
        try:
            read(pa.BufferReader(raw[:len(raw) // 2]))
        except (pa.ArrowInvalid, OSError):
            continue
        raise AssertionError(f"truncated {filename} was accepted")
    print(f"PyArrow {pa.__version__}: IPC/Parquet rows, ticks, nulls, ZSTD and truncation passed")


if __name__ == "__main__":
    main()
