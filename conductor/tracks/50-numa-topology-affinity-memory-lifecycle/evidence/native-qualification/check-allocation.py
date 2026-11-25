import csv
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path


def summarize(csv_path, protocol):
    samples = defaultdict(dict)
    for row in csv.DictReader(csv_path.open(), fieldnames=protocol["captured_column_order"]):
        key = (int(row["trial"]), row["setup"], row["mode"], int(row["width"]))
        worker = int(row["worker"])
        assert worker not in samples[key]
        samples[key][worker] = row

    wanted = {
        (trial, setup, mode, width)
        for trial in range(protocol["repeats_per_case"])
        for setup in protocol["cases"]
        for mode in ("serial", "parallel")
        for width in protocol["widths"]
    }
    assert samples.keys() == wanted
    groups = defaultdict(list)
    for (_, setup, mode, width), workers in samples.items():
        assert set(workers) == set(range(protocol["schedulers"]))
        assert {int(row["scheduled"]) for row in workers.values()} == {width * protocol["timed_epochs"]}
        span, = {int(row["matched_group_ns"]) for row in workers.values()}
        times = [int(row["worker_ns"]) for row in workers.values()]
        assert min(times) > 0
        if mode == "serial":
            assert span == sum(times)
        else:
            assert span >= max(times)
        groups[(setup, mode, width)].append((span, list(workers.values())))

    result = {}
    for (setup, mode, width), cases in sorted(groups.items()):
        measured = [worker for _, workers in cases for worker in workers]
        calls = [int(worker["timed_allocator_calls"]) for worker in measured]
        drop_bytes = [int(worker["bytes_retained_after_drop"]) for worker in measured]
        if setup == "reserved":
            assert set(calls) == {protocol["admitted_timed_allocator_calls_per_scheduler"]}
            assert set(drop_bytes) == {protocol["admitted_bytes_retained_after_drop_per_scheduler"]}
        result[f"{setup} {mode} {width}"] = {
            "workers": len(measured),
            "warm_calls": sorted({int(worker["warm_allocator_calls"]) for worker in measured}),
            "workers_with_timed_allocator_calls": sum(call > 0 for call in calls),
            "maximum_timed_allocator_calls": max(calls),
            "retained_bytes": sorted({int(worker["bytes_retained_before_drop"]) for worker in measured}),
            "post_drop_bytes": sorted(set(drop_bytes)),
            "median_group_ns": statistics.median(span for span, _ in cases),
        }
    return result


protocol = json.loads(Path(sys.argv[1]).read_text())
print(json.dumps(summarize(Path(sys.argv[2]), protocol), indent=2))
