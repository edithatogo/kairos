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
    source_runs = source.get("runs")
    require(isinstance(source_runs, list) and bool(source_runs), "missing source runs")
    run_ids = [r.get("run_id") for r in source_runs if isinstance(r, dict)]
    require(len(run_ids) == len(source_runs) and len(set(run_ids)) == len(run_ids), "invalid/duplicate source run")
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
        def cohort_identity(m: dict) -> tuple:
            return (m.get("endpoint"), m.get("strata"), m.get("window"), m.get("provenance"))
        require(cohort_identity(matched[0]) == cohort_identity(matched[1]), "W1/KS endpoint or provenance mismatch")
        provenance = matched[0].get("provenance")
        require(isinstance(provenance, dict), "metric provenance must be an object")
        bind_fields = ("dataset_id", "run_id", "mapping_version", "seed_schedule_id", "seed_map_ref",
                       "parameter_hash", "seed_contract_version")
        source_matches = [r for r in source_runs
                          if all(field not in r or provenance.get(field) == r.get(field)
                                 for field in bind_fields)]
        require(len(source_matches) == 1, "metric provenance does not resolve to exactly one source run")
        for m in matched:
            require(m["status"] == status and m["window"] == window, "status/window mismatch")
            require(m["algorithm_version"] == "empirical_equal.v1" and m["uncertainty"] is None, "algorithm/inference mismatch")
            for side in ("reference", "simulation"):
                require(count(e[f"{side}_count"]) == count(m[f"{side}_count"]), "sample count mismatch")
                if status in {"invalid", "unverified"}:
                    require(count(m[f"{side}_count"]) == 0, "invalid/unverified sample count must be zero")
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
    window = {"start_ticks": "0", "end_ticks": "10"}
    strata = {"candidate_id": "a"}
    run = {"run_id": "run-a", "dataset_id": "d", "mapping_version": "map-v1",
           "seed_schedule_id": "sched-v1", "seed_map_ref": "seed-map-v1",
           "parameter_hash": "a" * 64}
    provenance = {k: run[k] for k in run if k != "parameter_hash"}
    provenance.update(seed_contract_version="seed-v1", parameter_hash=run["parameter_hash"])

    def row(side, key, value, *, outcome="Point", time="1", group="a", **flags):
        result = dict(side=side, key=key, value=value, outcome=outcome, group=group,
                      selection_time=time, weight=None, excluded=False, censored=False,
                      missing=False, failed=False, infeasible=False)
        result.update(flags)
        return result

    def fixture(rows, status, counts, ties, warnings, diag_counts=None):
        diag_counts = diag_counts or dict(excluded=0, censored=0, missing=0, failed=0, infeasible=0)
        source = {"source_window": window, "metric_groups": [{"group": "a", "strata": strata}],
                  "metric_reference_rows": sum(r["side"] == "reference" for r in rows),
                  "metric_simulation_rows": sum(r["side"] == "simulation" for r in rows), "runs": [run]}
        join = {"metric_raw_records": rows, "metric_group_diagnostics": [dict(
            group="a", strata=strata, status=status, reference_count=counts[0], simulation_count=counts[1],
            reference_tie_count=ties[0], simulation_tie_count=ties[1], coverage_warnings=warnings)]}
        metrics = []
        for name in ("W1", "KS_D"):
            metrics.append(dict(metric=name, endpoint="departure", strata=strata, window=window,
                provenance=copy.deepcopy(provenance), status=status, reference_count=counts[0], simulation_count=counts[1],
                excluded_count=diag_counts["excluded"], censored_count=diag_counts["censored"],
                missing_count=diag_counts["missing"], failed_count=diag_counts["failed"],
                infeasible_count=diag_counts["infeasible"], unmatched_count=0,
                algorithm_version="empirical_equal.v1", uncertainty=None,
                value=0.25 if status == "computed" else None))
        return source, join, metrics

    def accepted(name, source, join, metrics):
        value = validate(join, source, metrics, required=True)
        return {"case": name, "status": value["status"]}

    positives = []
    # Rational-equivalent reference supports tie exactly once; the cross-side 1/2 does not count.
    rows = [row("reference", "r1", "1/2"), row("reference", "r2", "0.5"),
            row("reference", "r3", "2"), row("simulation", "s1", "0.5"),
            row("simulation", "s2", None, outcome="Censored")]
    source, join, metrics = fixture(rows, "computed", (3, 1), (1, 0),
        ["censored_observations_present", "tied_observations_present", "uncensored_subset_no_survival_correction"],
        dict(excluded=0, censored=1, missing=0, failed=0, infeasible=0))
    positives.append(accepted("computed_ties_cross_side_and_censoring", source, join, metrics))

    source, join, metrics = fixture([], "empty", (0, 0), (0, 0), [])
    positives.append(accepted("empty_declared_group", source, join, metrics))

    rows = [row("reference", "r1", "2"), row("reference", "r2", "2")]
    source, join, metrics = fixture(rows, "insufficient_data", (2, 0), (1, 0), ["tied_observations_present"])
    positives.append(accepted("insufficient_population", source, join, metrics))

    rows = [row("reference", "r1", "not-a-rational")]
    source, join, metrics = fixture(rows, "invalid", (0, 0), (None, None), [])
    positives.append(accepted("invalid_has_zero_counts_null_ties", source, join, metrics))

    rows = [row("reference", "r1", "1"), row("reference", "r2", "1"),
            row("simulation", "s1", None, outcome="Censored")]
    source, join, metrics = fixture(rows, "unverified", (0, 0), (None, None),
        ["censored_observations_present", "uncensored_subset_no_survival_correction"],
        dict(excluded=0, censored=1, missing=0, failed=0, infeasible=0))
    positives.append(accepted("unverified_null_ties_retains_raw_warnings", source, join, metrics))

    rows = [row("reference", "r1", "1"), row("simulation", "s1", "2", time="10")]
    source, join, metrics = fixture(rows, "insufficient_data", (1, 0), (0, 0),
        ["excluded_observations_present"], dict(excluded=1, censored=0, missing=0, failed=0, infeasible=0))
    positives.append(accepted("half_open_window_exclusion", source, join, metrics))

    rows = [row("reference", "r1", "1"), row("simulation", "s1", None, outcome="Censored",
             excluded=True, failed=True, infeasible=True)]
    warnings = ["censored_observations_present", "excluded_observations_present",
                "failed_outcomes_present", "infeasible_outcomes_present",
                "uncensored_subset_no_survival_correction"]
    source, join, metrics = fixture(rows, "insufficient_data", (1, 0), (0, 0), warnings,
        dict(excluded=1, censored=1, missing=0, failed=1, infeasible=1))
    positives.append(accepted("overlapping_raw_flags", source, join, metrics))

    controls = []
    def rejected(name, source, join, metrics, mutate):
        changed = copy.deepcopy((source, join, metrics))
        mutate(*changed)
        try:
            validate(changed[1], changed[0], changed[2], required=True)
        except (ValueError, KeyError, TypeError):
            controls.append(name)
        else:
            raise AssertionError(f"mutated {name} accepted")

    source, join, metrics = fixture(
        [row("reference", "r1", "1"), row("reference", "r2", "1"), row("simulation", "s1", "2")],
        "computed", (2, 1), (1, 0), ["tied_observations_present"])
    rejected("forged_tie_count", source, join, metrics,
             lambda s, j, m: j["metric_group_diagnostics"][0].__setitem__("reference_tie_count", 0))
    rejected("boolean_tie_count", source, join, metrics,
             lambda s, j, m: j["metric_group_diagnostics"][0].__setitem__("reference_tie_count", True))
    rejected("forged_coverage_warning", source, join, metrics,
             lambda s, j, m: j["metric_group_diagnostics"][0].__setitem__("coverage_warnings", []))
    rejected("endpoint_pair_mismatch", source, join, metrics,
             lambda s, j, m: m[1].__setitem__("endpoint", "arrival"))
    rejected("provenance_pair_mismatch", source, join, metrics,
             lambda s, j, m: m[1]["provenance"].__setitem__("run_id", "other-run"))
    rejected("source_run_provenance_mismatch", source, join, metrics,
             lambda s, j, m: (m[0]["provenance"].__setitem__("run_id", "other-run"),
                              m[1]["provenance"].__setitem__("run_id", "other-run")))
    rejected("missing_declared_groups", source, join, metrics, lambda s, j, m: s.pop("metric_groups"))
    rejected("missing_diagnostics", source, {"metric_raw_records": join["metric_raw_records"]}, metrics,
             lambda s, j, m: None)

    for status in ("invalid", "unverified"):
        source, join, metrics = fixture([row("reference", "r1", "1")], status, (0, 0), (None, None), [])
        positives.append(accepted(status + "_null_ties_zero_counts", source, join, metrics))
        rejected(status + "_nonzero_sample_count", source, join, metrics,
                 lambda s, j, m: (j["metric_group_diagnostics"][0].__setitem__("reference_count", 1),
                                  [x.__setitem__("reference_count", 1) for x in m]))
        rejected(status + "_non_null_tie_count", source, join, metrics,
                 lambda s, j, m: j["metric_group_diagnostics"][0].__setitem__("reference_tie_count", 0))

    require(validate({}, {}, []) == {"status": "not_supplied"}, "legacy compatibility")
    return {"status": "pass", "positive_cases": positives,
            "mutation_rejections": len(controls), "controls": controls}


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
