#!/usr/bin/env python3

import json
import random
import statistics
import sys
from collections import defaultdict
from pathlib import Path


def comparable(row):
    return {
        key: value
        for key, value in row.items()
        if key != "first_run" and not key.endswith(("_ns", "_per_second"))
    }


def ratio(left, right, repeats):
    assert len(left) == len(right) == repeats
    return statistics.geometric_mean(
        new / old for old, new in zip(left, right, strict=True)
    )


def summary(values, protocol):
    rng = random.Random(protocol["bootstrap_seed"])
    draws = sorted(
        statistics.median(rng.choices(values, k=len(values)))
        for _ in range(protocol["bootstrap_resamples"])
    )
    last = len(draws) - 1
    return {
        "median": statistics.median(values),
        "interval_95": [draws[last * 25 // 1000], draws[last * 975 // 1000]],
    }


def analyze(root):
    protocol = json.loads((root / "protocol.json").read_text())
    samples = root / "samples"
    measures = defaultdict(lambda: defaultdict(list))
    rss = defaultdict(list)
    raw = []
    repeats = protocol["timed_repeats_per_process"]
    for work in protocol["work_mixes"]:
        for pair in range(protocol["triplets_per_cost"]):
            order = protocol["order"][pair % len(protocol["order"])]
            runs = {}
            for arm in order:
                basename = samples / f"{work}-{pair}-{arm}"
                result = json.loads(basename.with_suffix(".json").read_text())
                peak, exit_code = map(int, basename.with_suffix(".rss").read_text().split())
                assert peak > 0 and exit_code == 0
                assert result["schema_version"] == "kairoecs.pdes.phold.v1"
                assert result["pdes_executor"] == "pooled"
                assert len(result["rows"]) == 5
                assert all(r["parity"] and r["event_work"] == work and r["seed"] == protocol["seed"]
                           for r in result["rows"])
                runs[arm] = {"result": result, "peak_rss_kib": peak}
                rss[f"{work}:{arm}"].append(peak)
            base, published, candidate = (runs[arm]["result"]["rows"]
                                          for arm in ("base", "published", "candidate"))
            for left, previous, right in zip(base, published, candidate, strict=True):
                assert comparable(left) == comparable(previous) == comparable(right)
                key = (
                    f"mixes={work} lps={right['lp_count']} lookahead={right['lookahead_ticks']} "
                    f"remote={right['remote_percent_target']}"
                )
                series = measures[key]
                for reference, old in (("base", left), ("published", previous)):
                    series[f"candidate_over_{reference}"].append(
                        ratio(old["pdes_ns"], right["pdes_ns"], repeats)
                    )
                series["published_over_base"].append(
                    ratio(left["pdes_ns"], previous["pdes_ns"], repeats)
                )
                series["candidate_pool_over_serial"].append(
                    ratio(right["sequential_ns"], right["pdes_ns"], repeats)
                )
                series["published_pool_over_serial"].append(
                    ratio(previous["sequential_ns"], previous["pdes_ns"], repeats)
                )
            raw.append({"work": work, "pair": pair, "order": order, "runs": runs})
    assert len(list(samples.glob("*.json"))) == 3 * len(raw)
    assert len(raw) == len(protocol["work_mixes"]) * protocol["triplets_per_cost"]
    profiles = {key: {name: summary(values, protocol) for name, values in series.items()}
                for key, series in measures.items()}
    target = protocol["target"]
    target_profile = profiles[target]
    gates = {}
    for comparison, maximum in (("candidate_over_base", .80), ("candidate_over_published", 1.0),
                                ("published_over_base", .80)):
        target_result = target_profile[comparison]
        other = max(profile[comparison]["median"] for key, profile in profiles.items()
                    if key != target)
        gates[comparison] = {
            "target": target_result, "largest_other": other,
            "pass": target_result["median"] <= maximum
            and target_result["interval_95"][1] < 1 and other <= 1.05,
        }
    output = {
        "triplets": len(raw),
        "profiles": profiles,
        "peak_rss_kib_medians": {key: statistics.median(values) for key, values in rss.items()},
        "gates": gates,
    }
    (root / "result.json").write_text(json.dumps(output, indent=2) + "\n")
    (root / "raw.jsonl").write_text("".join(json.dumps(row) + "\n" for row in raw))
    print(json.dumps(gates, indent=2))


if __name__ == "__main__":
    analyze(Path(sys.argv[1]))
