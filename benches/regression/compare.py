#!/usr/bin/env python3
"""Track 31 benchmark threshold validator and regression comparator."""


import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_THRESHOLDS = ROOT / "conductor" / "performance-thresholds.md"
DEFAULT_SMOKE = ROOT / "benches" / "benchmark-smoke.json"


@dataclass(frozen=True)
class Threshold:
    benchmark: str
    measure: str
    owner: str
    regression_percent: float
    gate: str

    @property
    def blocking(self) -> bool:
        return self.gate.strip().lower() == "blocking"


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def parse_thresholds(path: Path) -> dict[str, Threshold]:
    thresholds: dict[str, Threshold] = {}
    table_started = False
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line.startswith("|"):
            if table_started:
                break
            continue
        if "`" not in line:
            if "Benchmark" in line:
                table_started = True
            continue

        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 7:
            continue
        benchmark_match = re.fullmatch(r"`([^`]+)`", cells[0])
        threshold_match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)%", cells[4])
        if not benchmark_match or not threshold_match:
            continue

        benchmark = benchmark_match.group(1)
        if benchmark in thresholds:
            raise ValueError(f"duplicate threshold row for {benchmark}")
        thresholds[benchmark] = Threshold(
            benchmark=benchmark,
            measure=cells[1],
            owner=cells[2],
            regression_percent=float(threshold_match.group(1)),
            gate=cells[6],
        )

    if not thresholds:
        raise ValueError(f"no threshold rows found in {path}")
    return thresholds


def canonical_benchmarks(smoke_path: Path) -> dict[str, dict[str, Any]]:
    smoke = load_json(smoke_path)
    manifest_path = ROOT / smoke["source_manifest"]
    manifest = load_json(manifest_path)
    manifest_benchmarks = {
        item["id"]: item for item in manifest.get("benchmarks", []) if item.get("status") == "canonical"
    }
    smoke_benchmarks = {item["id"]: item for item in smoke.get("scenarios", [])}

    missing_from_manifest = sorted(set(smoke_benchmarks) - set(manifest_benchmarks))
    if missing_from_manifest:
        raise ValueError(
            "smoke metadata references non-canonical manifest benchmark(s): "
            + ", ".join(missing_from_manifest)
        )

    return {benchmark_id: manifest_benchmarks[benchmark_id] for benchmark_id in smoke_benchmarks}


def validate_threshold_coverage(
    thresholds: dict[str, Threshold], benchmarks: dict[str, dict[str, Any]]
) -> dict[str, list[str]]:
    benchmark_ids = set(benchmarks)
    threshold_ids = set(thresholds)
    missing = sorted(benchmark_ids - threshold_ids)
    orphaned = sorted(threshold_ids - benchmark_ids)
    owner_mismatches = sorted(
        benchmark_id
        for benchmark_id in benchmark_ids & threshold_ids
        if str(benchmarks[benchmark_id].get("owner")) != thresholds[benchmark_id].owner
    )
    measure_mismatches = sorted(
        benchmark_id
        for benchmark_id in benchmark_ids & threshold_ids
        if str(benchmarks[benchmark_id].get("measure")) != thresholds[benchmark_id].measure
    )
    return {
        "missing_thresholds": missing,
        "orphaned_thresholds": orphaned,
        "owner_mismatches": owner_mismatches,
        "measure_mismatches": measure_mismatches,
    }


def extract_results(path: Path) -> dict[str, float]:
    payload = load_json(path)
    records = payload.get("benchmarks") if isinstance(payload, dict) else payload
    if not isinstance(records, list):
        raise ValueError(f"{path} must contain a benchmark list or a benchmarks array")

    results: dict[str, float] = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError(f"{path} contains a non-object benchmark record")
        benchmark_id = first_present(record, ["id", "name", "benchmark", "scenario"])
        if benchmark_id is None:
            raise ValueError(f"{path} contains a record without an ID")
        benchmark_id = str(benchmark_id)
        if benchmark_id in results:
            raise ValueError(f"{path} contains duplicate result ID {benchmark_id}")
        mean = extract_mean(record)
        if mean <= 0:
            raise ValueError(f"{benchmark_id} in {path} has non-positive mean {mean}")
        results[benchmark_id] = mean
    return results


def first_present(record: dict[str, Any], keys: list[str]) -> Any | None:
    for key in keys:
        if key in record:
            return record[key]
    return None


def extract_mean(record: dict[str, Any]) -> float:
    for key in ["mean", "mean_seconds", "mean_ms", "time"]:
        if key in record:
            return float(record[key])
    estimates = record.get("estimates")
    if isinstance(estimates, dict):
        mean = estimates.get("mean")
        if isinstance(mean, dict) and "point_estimate" in mean:
            return float(mean["point_estimate"])
    raise ValueError(f"{record!r} does not expose a supported mean duration")


def compare_results(
    base: dict[str, float], current: dict[str, float], thresholds: dict[str, Threshold]
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for benchmark_id, threshold in sorted(thresholds.items()):
        if benchmark_id not in base or benchmark_id not in current:
            continue
        base_mean = base[benchmark_id]
        current_mean = current[benchmark_id]
        change_percent = ((current_mean - base_mean) / base_mean) * 100
        status = "pass"
        if change_percent > threshold.regression_percent:
            status = "fail" if threshold.blocking else "advisory"
        rows.append(
            {
                "benchmark": benchmark_id,
                "base_mean": base_mean,
                "current_mean": current_mean,
                "change_percent": round(change_percent, 4),
                "threshold_percent": threshold.regression_percent,
                "gate": threshold.gate,
                "status": status,
            }
        )
    return rows


def validate_result_ids(
    base: dict[str, float], current: dict[str, float], thresholds: dict[str, Threshold]
) -> dict[str, list[str]]:
    threshold_ids = set(thresholds)
    return {
        "missing_base_results": sorted(threshold_ids - set(base)),
        "missing_current_results": sorted(threshold_ids - set(current)),
        "unknown_base_results": sorted(set(base) - threshold_ids),
        "unknown_current_results": sorted(set(current) - threshold_ids),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--thresholds", type=Path, default=DEFAULT_THRESHOLDS)
    parser.add_argument("--smoke", type=Path, default=DEFAULT_SMOKE)
    parser.add_argument("--base", type=Path, help="accepted baseline benchmark JSON")
    parser.add_argument("--current", type=Path, help="current branch benchmark JSON")
    parser.add_argument("--report", type=Path, help="optional JSON report output path")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    thresholds = parse_thresholds(args.thresholds)
    benchmarks = canonical_benchmarks(args.smoke)
    coverage = validate_threshold_coverage(thresholds, benchmarks)
    failures = [name for name, values in coverage.items() if values]

    report: dict[str, Any] = {
        "status": "pass" if not failures else "fail",
        "thresholds": sorted(thresholds),
        "canonical_benchmarks": sorted(benchmarks),
        "coverage": coverage,
    }

    if args.base or args.current:
        if not args.base or not args.current:
            raise SystemExit("--base and --current must be supplied together")
        base = extract_results(args.base)
        current = extract_results(args.current)
        result_id_coverage = validate_result_ids(base, current, thresholds)
        comparison = compare_results(base, current, thresholds)
        report["comparison"] = comparison
        report["result_id_coverage"] = result_id_coverage
        failures.extend(name for name, values in result_id_coverage.items() if values)
        if any(row["status"] == "fail" for row in comparison):
            failures.append("blocking_regression")
        report["status"] = "pass" if not failures else "fail"

    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
