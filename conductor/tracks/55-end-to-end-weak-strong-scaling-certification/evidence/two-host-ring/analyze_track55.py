"""Check exact native ring readback and summarize paired host measurements."""
import hashlib
import json
from pathlib import Path
from statistics import median

GROUPS = ("strong", "weak")
MASK = (1 << 64) - 1
STEP = 0x9E3779B97F4A7C15


def mix(value):
    next_seed = (value + STEP) & MASK
    middle = ((next_seed ^ (next_seed >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    middle = ((middle ^ (middle >> 27)) * 0x94D049BB133111EB) & MASK
    return middle ^ (middle >> 31)


def run_reference(roots, seed, work, memo):
    values = [[], []]
    tape = hashlib.sha256()
    for index in range(roots):
        lp = index % 2
        value = mix((seed + index * STEP) & MASK)
        values[lp].append(value)
        tape.update(lp.to_bytes(4, "little") * 2)
        tape.update(bytes(16) + (9).to_bytes(8, "little"))
        tape.update(bytes(1) + value.to_bytes(8, "little"))
        if value not in memo:
            folded = value
            for _ in range(work):
                folded = mix(folded)
            memo[value] = folded
    states = []
    for lp, own in enumerate(values):
        checksum = 0
        for key in own + values[1 - lp]:
            rotated = ((checksum << 9) | (checksum >> 55)) & MASK
            checksum = (rotated + memo[key]) & MASK
            # At tick 0 kind == 0 and at tick 1 kind == 1. Their XOR cancels.
        states.append([roots, checksum])
    return tape.hexdigest(), states


def main():
    evidence = Path(__file__).resolve().parent / "evidence"
    data = [json.loads(line) for line in (evidence / "measurements.jsonl").read_text().splitlines()]
    assert [(r["sample"], r["trial"]) for r in data] == [(i, i // 4) for i in range(44)]
    assert all((r["logical_lps"], r["input_seed"], r["event_work_mixes"]) ==
               (2, 47_2026, 8_192) for r in data)
    memo = {}
    expected = {count: run_reference(count, 47_2026, 8_192, memo) for count in (1_024, 2_048)}
    for row in data:
        hosts = row["physical_hosts"]
        roots = 1_024 if row["mode"] == "weak" and hosts == 1 else 2_048
        digest, states = expected[roots]
        assert (row["root_events"], row["events"]) == (roots, roots * 2)
        assert row["input_sha256"] == digest
        assert row["state"] == row["serial_expected"] == states
        assert row["parity"] and row["wall_ns"] > 0
        assert row["native_frames"] == hosts * 4
        assert row["cross_host_frames"] == (hosts - 1) * 4
        assert row["warmup"] == (row["trial"] == 0)
    matrix = {}
    for mode in GROUPS:
        measured = sorted(
            (r for r in data if not r["warmup"] and r["mode"] == mode),
            key=lambda r: (r["trial"], r["physical_hosts"]),
        )
        one, two = measured[::2], measured[1::2]
        assert [(r["trial"], r["physical_hosts"]) for r in one] == [(n, 1) for n in range(1, 11)]
        assert [(r["trial"], r["physical_hosts"]) for r in two] == [(n, 2) for n in range(1, 11)]
        ratios = [a["wall_ns"] / b["wall_ns"] for a, b in zip(one, two, strict=True)]
        matrix[mode] = {
            "pairs": len(ratios),
            "one_host_events": one[0]["events"],
            "two_host_events": two[0]["events"],
            "one_host_median_ms": median(r["wall_ns"] for r in one) / 1e6,
            "two_host_median_ms": median(r["wall_ns"] for r in two) / 1e6,
            "one_host_median_events_per_second": median(r["events"] * 1e9 / r["wall_ns"] for r in one),
            "two_host_median_events_per_second": median(r["events"] * 1e9 / r["wall_ns"] for r in two),
            "one_over_two_host_paired_ratio_median": median(ratios),
            "one_over_two_host_paired_ratio_range": [min(ratios), max(ratios)],
            "nominal_efficiency": median(ratios) / (2 if mode == "strong" else 1),
        }
    result = {
        "checks": {"native_serial_parity_samples": len(data), "independent_python_tapes": len(expected)},
        "rows": matrix,
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
