import hashlib
import json
import random
import statistics
import subprocess
import sys
from pathlib import Path

stage = Path(sys.argv[1])
protocol = json.loads((stage / "frontier-protocol.json").read_text())
base = stage / "frontier-base"
candidate = stage / "frontier-candidate"
binaries = {"base": base, "candidate": candidate}
hashes = {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in binaries.items()}

def frozen(row):
    return {
        key: value
        for key, value in row.items()
        if key != "first_run" and not key.endswith(("_ns", "_per_second"))
    }

def run(name, work):
    argv = [
        str(binaries[name]), "--phold", "--seed", str(protocol["seed"]),
        "--repetitions", str(protocol["timed_repeats_per_process"]), "--event-work", str(work),
    ]
    output = subprocess.run(argv, check=True, capture_output=True)
    result = json.loads(output.stdout)
    assert result["schema_version"] == "kairoecs.pdes.phold.v1"
    assert result["pdes_executor"] == "pooled"
    assert len(result["rows"]) == 5
    assert all(row["parity"] for row in result["rows"])
    return {"argv": argv, "executable_sha256": hashes[name], "exit": 0, "result": result}

def interval(values, rng):
    samples = sorted(
        statistics.median(rng.choices(values, k=len(values)))
        for _ in range(protocol["bootstrap_resamples"])
    )
    return [samples[(len(samples) - 1) * 25 // 1000], samples[(len(samples) - 1) * 975 // 1000]]

samples = {}
with (stage / "frontier-pairs.jsonl").open("w") as stream:
    for cost in protocol["mixes_per_event"]:
        for index in range(protocol["process_pairs_per_cost"]):
            order = ("base", "candidate") if index % 2 == 0 else ("candidate", "base")
            runs = {name: run(name, cost) for name in order}
            left = runs["base"]["result"]["rows"]
            right = runs["candidate"]["result"]["rows"]
            assert [frozen(row) for row in left] == [frozen(row) for row in right]
            for before, after in zip(left, right, strict=True):
                key = (cost, before["lp_count"], before["lookahead_ticks"], before["remote_percent_target"])
                elapsed = [new / old for old, new in zip(before["pdes_ns"], after["pdes_ns"], strict=True)]
                assert len(elapsed) == protocol["timed_repeats_per_process"]
                samples.setdefault(key, []).append(statistics.geometric_mean(elapsed))
            stream.write(json.dumps({"cost": cost, "pair": index, "order": order, "runs": runs}) + "\n")
rng = random.Random(protocol["bootstrap_seed"])
result = {
    f"mixes={work} lps={lps} lookahead={lookahead} remote={remote}": {
        "candidate_over_baseline": statistics.median(values),
        "interval_95": interval(values, rng),
    }
    for (work, lps, lookahead, remote), values in samples.items()
}
(stage / "frontier-summary.json").write_text(json.dumps(result, indent=2) + "\n")
