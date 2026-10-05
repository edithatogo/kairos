#!/usr/bin/env python3
"""Independent exact readback of supplemental C4.4 group diagnostics."""
from __future__ import annotations
import argparse
import copy
import json
from fractions import Fraction
from pathlib import Path
from typing import Any


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def count(value: Any) -> int:
    require(type(value) is int and value >= 0, "count must be a nonnegative integer")
    return value


def tick(value: Any) -> int:
    require(isinstance(value, str) and value.isascii() and value.isdecimal(), "tick must be decimal text")
    n = int(value)
    require(str(n) == value and n < 2**128, "noncanonical or overflowing tick")
    return n


def validate(join: dict, source: dict, metrics: list, *, required: bool = False) -> dict:
    entries = join.get("metric_group_diagnostics")
    if entries is None and not required:
        return {"status": "not_supplied"}
    require(isinstance(entries, list) and bool(entries), "missing statistical diagnostics")
    declared = source.get("metric_groups")
    require(isinstance(declared, list) and bool(declared), "missing declared metric groups")
    definitions = {}
    for d in declared:
        require(isinstance(d, dict) and set(d) == {"group", "strata"}, "invalid group declaration")
        g, s = d["group"], d["strata"]
        require(isinstance(g, str) and bool(g.strip()) and g not in definitions, "empty/duplicate group")
        require(isinstance(s, dict), "strata must be an object")
        definitions[g] = s
    require(list(definitions) == sorted(definitions), "noncanonical declared groups")
    require(len({json.dumps(s, sort_keys=True) for s in definitions.values()}) == len(definitions), "ambiguous strata")
    require([e.get("group") for e in entries if isinstance(e, dict)] == list(definitions), "group list mismatch")
    window = source["source_window"]
    start, end = tick(window["start_ticks"]), tick(window["end_ticks"])
    require(start < end, "invalid source window")
    raw = join.get("metric_raw_records")
    require(isinstance(raw, list), "missing raw metric records")
    seen = set()
    for r in raw:
        require(isinstance(r, dict) and r.get("group") in definitions, "undeclared raw group")
        require(r.get("side") in {"reference", "simulation"}, "invalid raw side")
        key = (r["side"], r.get("key"))
        require(isinstance(key[1], str) and bool(key[1].strip()) and key not in seen, "empty/duplicate raw key")
        seen.add(key)
        require(r.get("outcome") in {"Point", "Missing", "Censored", "Failed", "Infeasible"}, "invalid outcome")
        for flag in ("excluded", "censored", "missing", "failed", "infeasible"):
            require(type(r.get(flag)) is bool, "diagnostic flag must be Boolean")
        require((r["outcome"] == "Point") == (r.get("value") is not None), "point/value contradiction")
        require(r.get("weight") is None, "sidecar requires empirical equal weights")
        if r.get("selection_time") is not None:
            tick(r["selection_time"])
    for side in ("reference", "simulation"):
        require(sum(r["side"] == side for r in raw) == count(source[f"metric_{side}_rows"]), "raw/source count mismatch")
    verified = []
    for e in entries:
        require(set(e) == {"group", "strata", "status", "reference_count", "simulation_count", "reference_tie_count", "simulation_tie_count", "coverage_warnings"}, "diagnostic field mismatch")
        group, status = e["group"], e["status"]
        require(e["strata"] == definitions[group], "source strata mismatch")
        matched = [m for m in metrics if m.get("metric") in {"W1", "KS_D"} and m.get("strata") == e["strata"]]
        require(len(matched) == 2 and {m["metric"] for m in matched} == {"W1", "KS_D"}, "metric binding mismatch")
        require(status in {"computed", "empty", "insufficient_data", "invalid", "unverified"}, "invalid status")
        for m in matched:
            require(m["status"] == status and m["window"] == window, "status/window mismatch")
            require(m["algorithm_version"] == "empirical_equal.v1" and m["uncertainty"] is None, "algorithm/inference mismatch")
            for side in ("reference", "simulation"):
                require(count(e[f"{side}_count"]) == count(m[f"{side}_count"]), "sample count mismatch")
        rows = [r for r in raw if r["group"] == group]
        selected = {"reference": [], "simulation": []}
        totals = dict.fromkeys(("excluded", "censored", "missing", "failed", "infeasible"), 0)
        for r in rows:
            time = r["selection_time"]
            outside = time is None or not start <= tick(time) < end
            totals["excluded"] += int(r["excluded"] or outside)
            totals["censored"] += int(r["censored"] or r["outcome"] == "Censored")
            totals["missing"] += int(r["missing"] or r["outcome"] == "Missing" or time is None)
            totals["failed"] += int(r["failed"] or r["outcome"] == "Failed")
            totals["infeasible"] += int(r["infeasible"] or r["outcome"] == "Infeasible")
            if not r["excluded"] and not outside and r["outcome"] == "Point":
                selected[r["side"]].append(r)
        for m in matched:
            for label, total in totals.items():
                require(count(m[f"{label}_count"]) == total, "coverage count mismatch")
            require(count(m["unmatched_count"]) == 0, "unpaired unmatched count invented")
        ties = {}
        for side, points in selected.items():
            observed = e[f"{side}_tie_count"]
            if status in {"invalid", "unverified"}:
                require(observed is None, "unverified/rejected ties must be null")
                ties[side] = None
            else:
                require(count(e[f"{side}_count"]) == len(points), "eligible count mismatch")
                require(all(isinstance(p["value"], str) for p in points), "supports must be exact text")
                values = [Fraction(p["value"]) for p in points]
                expected = len(values) - len(set(values))
                require(count(observed) == expected, "exact tie count mismatch")
                ties[side] = expected
        if status not in {"invalid", "unverified"}:
            sizes = [len(p) for p in selected.values()]
            expected = "computed" if all(sizes) else "empty" if not any(sizes) else "insufficient_data"
            require(status == expected, "population status mismatch")
        labels = {"censored": "censored_observations_present", "excluded": "excluded_observations_present", "failed": "failed_outcomes_present", "infeasible": "infeasible_outcomes_present", "missing": "missing_outcomes_present"}
        warnings = [labels[k] for k, n in totals.items() if n]
        if any(n is not None and n > 0 for n in ties.values()):
            warnings.append("tied_observations_present")
        if any(r["outcome"] == "Censored" for r in rows):
            warnings.append("uncensored_subset_no_survival_correction")
        require(e["coverage_warnings"] == sorted(warnings), "source-derived warnings mismatch")
        verified.append({"group": group, "ties": ties, "coverage_warnings": sorted(warnings), "status": status})
    require(len([m for m in metrics if m.get("metric") in {"W1", "KS_D"}]) == 2 * len(definitions), "undeclared metrics")
    return {"status": "pass", "groups": verified}


def self_test() -> dict:
    strata = {"candidate_id": "a"}
    source = {"source_window": {"start_ticks": "0", "end_ticks": "10"}, "metric_groups": [{"group": "a", "strata": strata}], "metric_reference_rows": 3, "metric_simulation_rows": 1}
    def row(side, key, value, outcome="Point"):
        return dict(side=side, key=key, value=value, outcome=outcome, group="a", selection_time="1", weight=None, excluded=False, censored=False, missing=False, failed=False, infeasible=False)
    join = {"metric_raw_records": [row("reference", "r1", "1/2"), row("reference", "r2", "0.5"), row("reference", "r3", None, "Censored"), row("simulation", "s1", "1")], "metric_group_diagnostics": [dict(group="a", strata=strata, status="computed", reference_count=2, simulation_count=1, reference_tie_count=1, simulation_tie_count=0, coverage_warnings=["censored_observations_present", "tied_observations_present", "uncensored_subset_no_survival_correction"])]}
    metrics = [dict(metric=n, strata=strata, status="computed", window=source["source_window"], reference_count=2, simulation_count=1, excluded_count=0, censored_count=1, missing_count=0, failed_count=0, infeasible_count=0, unmatched_count=0, algorithm_version="empirical_equal.v1", uncertainty=None) for n in ("W1", "KS_D")]
    validate(join, source, metrics, required=True)
    controls = []
    for field, value in (("reference_tie_count", 0), ("reference_tie_count", True), ("reference_count", 3), ("coverage_warnings", []), ("coverage_warnings", ["caller_invented"]), ("strata", {}), ("status", "invalid")):
        changed = copy.deepcopy(join)
        changed["metric_group_diagnostics"][0][field] = value
        try:
            validate(changed, source, metrics, required=True)
        except (ValueError, KeyError, TypeError):
            controls.append(field)
        else:
            raise AssertionError(f"mutated {field} accepted")
    try:
        validate({}, source, metrics, required=True)
    except ValueError:
        controls.append("missing_diagnostics")
    else:
        raise AssertionError("missing diagnostics accepted")
    require(validate({}, {}, []) == {"status": "not_supplied"}, "legacy compatibility")
    return {"status": "pass", "mutation_rejections": len(controls), "controls": controls}


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--self-test", action="store_true")
    p.add_argument("directory", nargs="?", type=Path)
    a = p.parse_args()
    if a.self_test:
        result = self_test()
    else:
        if a.directory is None:
            p.error("directory is required")
        def load(name):
            return json.loads((a.directory / name).read_text(), parse_constant=lambda x: require(False, f"invalid JSON {x}"))
        result = validate(load("join_manifest.json"), load("source_manifest.json"), load("metric.json"), required=True)
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
