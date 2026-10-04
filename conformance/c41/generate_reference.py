#!/usr/bin/env python3
"""Build/reproduce the independent C4.1 exact and floating reference packet."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIXTURES = HERE / "fixtures.json"
OUTPUT = HERE / "reference.json"
LOCK = HERE / "requirements-reference.lock"
PROVENANCE = HERE / "wheel-provenance.json"
U128_MAX = 2**128 - 1


def canonical(obj: object) -> bytes:
    return (
        json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def q(value: str | int) -> Fraction:
    return Fraction(str(value))


def serial(x: Fraction | None) -> str | None:
    return (
        None
        if x is None
        else (
            str(x.numerator) if x.denominator == 1 else f"{x.numerator}/{x.denominator}"
        )
    )


def exact_metric(
    left: list[Fraction],
    right: list[Fraction],
    lw: list[Fraction] | None = None,
    rw: list[Fraction] | None = None,
) -> tuple[Fraction, Fraction]:
    """Independent exact support sweep; ties are consumed as a single group."""
    if not left or not right:
        raise ValueError("empty eligible population")
    if lw is None:
        lw = [Fraction(1) for _ in left]
    if rw is None:
        rw = [Fraction(1) for _ in right]
    if len(left) != len(lw) or len(right) != len(rw):
        raise ValueError("weight length mismatch")
    if any(w < 0 for w in lw + rw):
        raise ValueError("negative weight")
    sl, sr = sum(lw), sum(rw)
    if sl <= 0 or sr <= 0:
        raise ValueError("zero mass")
    lm: dict[Fraction, Fraction] = {}
    rm: dict[Fraction, Fraction] = {}
    for x, w in zip(left, lw):
        lm[x] = lm.get(x, Fraction()) + w / sl
    for x, w in zip(right, rw):
        rm[x] = rm.get(x, Fraction()) + w / sr
    support = sorted(lm.keys() | rm.keys())
    cl = cr = area = maximum = Fraction()
    for i, x in enumerate(support):
        cl += lm.get(x, Fraction())
        cr += rm.get(x, Fraction())
        gap = abs(cl - cr)
        maximum = max(maximum, gap)
        if i + 1 < len(support):
            area += gap * (support[i + 1] - x)
    return area, maximum


def fnum(v: str) -> float:
    special = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}
    return special[v] if v in special else float(q(v))


def validate_input(inp: dict) -> None:
    if "reference_origin" in inp or "candidate_origin" in inp:
        raise ValueError("independent origins are forbidden")
    if "origin" in inp and inp["origin"] is not None:
        origin = int(inp["origin"])
        ticks = inp.get("reference", []) + inp.get("candidate", [])
        if origin < 0 or any(int(v) < origin or int(v) > U128_MAX for v in ticks):
            raise ValueError("origin/ticks outside common unsigned u128 scope")
    if q(inp["scale_ticks"]) <= 0:
        raise ValueError("scale_ticks must be positive")
    warnings = inp.get("coverage_warnings", [])
    if warnings != sorted(set(warnings)):
        raise ValueError("coverage warnings must be canonical, unique and sorted")


def classify(case: dict) -> tuple[str, str | None, str | None, str, list[str]]:
    inp = case["input"]
    left, right = inp.get("reference", []), inp.get("candidate", [])
    algo = inp["algorithm_version"]
    precision = "not_applicable"
    warnings = list(inp.get("coverage_warnings", []))
    try:
        validate_input(inp)
        if inp.get("origin") is not None:
            precision = "exact_offsets"
            origin = int(inp["origin"])
            offsets = [int(v) - origin for v in left + right]
            if (
                origin < 0
                or origin > U128_MAX
                or any(int(v) < origin or int(v) > U128_MAX for v in left + right)
            ):
                raise OverflowError
            # float conversion is accepted only if it preserves the exact integer offset.
            if any(int(float(x)) != x for x in offsets):
                return "invalid", None, None, "rejected", warnings
            scale = q(inp["scale_ticks"])
            if scale <= 0:
                raise ValueError("scale_ticks must be positive")
            left = [serial(Fraction(int(v) - origin, 1) / scale) for v in left]
            right = [serial(Fraction(int(v) - origin, 1) / scale) for v in right]
        # Null observations are excluded from point support and accounted as missing.
        left = [v for v in left if v is not None]
        right = [v for v in right if v is not None]
        lq, rq = [q(v) for v in left], [q(v) for v in right]
        if any(not math.isfinite(fnum(v)) for v in left + right):
            raise ValueError("nonfinite")
    except (ValueError, OverflowError, TypeError, ZeroDivisionError):
        return (
            "invalid",
            None,
            None,
            "rejected" if precision == "exact_offsets" else precision,
            warnings,
        )
    if not lq and not rq:
        return "empty", None, None, precision, warnings
    if not lq or not rq:
        return "insufficient_data", None, None, precision, warnings
    if algo == "weighted_descriptive.v1":
        try:
            lweights = [q(v) for v in inp["reference_weights"]]
            rweights = [q(v) for v in inp["candidate_weights"]]
            if not math.isfinite(float(sum(lweights))) or not math.isfinite(
                float(sum(rweights))
            ):
                return "invalid", None, None, precision, warnings
        except (ValueError, KeyError, ZeroDivisionError):
            return "invalid", None, None, precision, warnings
        except OverflowError:
            return "invalid", None, None, precision, warnings
        try:
            w, d = exact_metric(lq, rq, lweights, rweights)
        except ValueError:
            return "invalid", None, None, precision, warnings
    else:
        try:
            w, d = exact_metric(lq, rq)
        except ValueError:
            return "invalid", None, None, precision, warnings
    return "ok", serial(w), serial(d), precision, warnings


def make_fixture(case: dict) -> dict:
    inp = case["input"]
    status, w1, ks, precision, warnings = classify(case)
    raw_left, raw_right = inp.get("reference", []), inp.get("candidate", [])
    nleft, nright = (
        sum(v is not None for v in raw_left),
        sum(v is not None for v in raw_right),
    )
    missing_nulls = sum(v is None for v in raw_left + raw_right)
    supplied = inp.get("counts", {})
    counts = {
        "reference": str(nleft),
        "candidate": str(nright),
        "eligible": str(supplied.get("eligible", nleft + nright + missing_nulls)),
        "excluded": str(supplied.get("excluded", 0)),
        "unmatched": "0",
        "unmatched_estimand": "not_applicable",
        "cohort_total": str(
            int(supplied.get("eligible", nleft + nright + missing_nulls))
            + int(supplied.get("excluded", 0))
        ),
    }
    primary = {
        k: str(supplied.get("primary_dispositions", {}).get(k, v))
        for k, v in {
            "observed": nleft + nright,
            "censored": 0,
            "missing": missing_nulls,
            "failed": 0,
            "infeasible": 0,
        }.items()
    }
    result = {
        "status": status,
        "w1": w1,
        "ks_d": ks,
        "unit": inp["unit"],
        "scale_ticks": str(inp["scale_ticks"]),
        "algorithm_version": inp["algorithm_version"],
        "counts": counts,
        "precision": precision,
        "weighting": "explicit"
        if inp["algorithm_version"] == "weighted_descriptive.v1"
        else "equal",
        "warnings": warnings,
        "p_value": None,
    }
    result["diagnostic"] = {"primary_dispositions": primary}
    if "diagnostic" in inp:
        result["diagnostic"].update(inp["diagnostic"])
    grouped = inp.get("grouped_observations")
    if grouped:
        group_key, value_key = grouped["group_variable"], grouped["compared_variable"]
        rows = []
        for group in grouped["prespecified_groups"]:
            left_values = [
                r[value_key] for r in grouped["reference"] if r[group_key] == group
            ]
            right_values = [
                r[value_key] for r in grouped["candidate"] if r[group_key] == group
            ]
            if not left_values and not right_values:
                status, w1, ks = "empty", None, None
            elif not left_values or not right_values:
                status, w1, ks = "insufficient_data", None, None
            else:
                w, d = exact_metric(
                    [q(v) for v in left_values], [q(v) for v in right_values]
                )
                status, w1, ks = "ok", serial(w), serial(d)
            rows.append(
                {
                    "group": group,
                    "eligible_reference": str(len(left_values)),
                    "eligible_candidate": str(len(right_values)),
                    "status": status,
                    "w1": w1,
                    "ks_d": ks,
                }
            )
        result["diagnostic"]["group_variable"] = group_key
        result["diagnostic"]["compared_variable"] = value_key
        result["diagnostic"]["groups"] = rows
    if missing_nulls:
        result["diagnostic"] = result.get("diagnostic", {})
        result["diagnostic"]["raw_population_counts"] = {
            "reference": str(len(raw_left)),
            "candidate": str(len(raw_right)),
        }
    if any(k in supplied for k in ("censor_subtypes", "overlap_counts")):
        result["diagnostic"] = result.get("diagnostic", {})
        result["diagnostic"]["overlap_counts"] = supplied.get("overlap_counts", {})
        result["diagnostic"]["censor_subtypes"] = supplied.get("censor_subtypes", {})
    return {
        "id": case["id"],
        "family": case["family"],
        "input": inp,
        "expected": result,
    }


def scipy_crosschecks(fixtures: dict) -> list[dict]:
    import numpy as np
    import scipy
    from scipy.stats import ks_2samp, wasserstein_distance

    if np.__version__ != "2.5.3" or scipy.__version__ != "1.18.1":
        raise RuntimeError(
            f"pinned versions required; got scipy={scipy.__version__}, numpy={np.__version__}"
        )
    rows = []
    for c in fixtures["cases"]:
        inp = c["input"]
        if (
            c["expected"]["status"] != "ok"
            or inp["algorithm_version"]
            not in ("empirical_equal.v1", "weighted_descriptive.v1")
            or inp.get("origin") is not None
        ):
            continue
        a, b = (
            [fnum(v) for v in inp["reference"] if v is not None],
            [fnum(v) for v in inp["candidate"] if v is not None],
        )
        if not all(math.isfinite(x) for x in a + b):
            continue
        weighted = inp["algorithm_version"] == "weighted_descriptive.v1"
        lw = [float(q(v)) for v in inp["reference_weights"]] if weighted else None
        rw = [float(q(v)) for v in inp["candidate_weights"]] if weighted else None
        w = wasserstein_distance(a, b, u_weights=lw, v_weights=rw)
        tol_w = 1e-12 * max(1.0, float(q(inp["scale_ticks"])))
        if abs(float(w) - float(q(c["expected"]["w1"]))) > tol_w:
            raise ValueError(f"SciPy W1 crosscheck exceeds frozen tolerance: {c['id']}")
        row = {
            "id": c["id"],
            "scipy_w1": float(w),
            "w1_tolerance": tol_w,
            "versions": {"scipy": scipy.__version__, "numpy": np.__version__},
            "comparison": "floating_crosscheck_only",
        }
        if not weighted:
            # KS statistic only; do not retain or infer SciPy's p-value.
            d = ks_2samp(a, b, method="exact").statistic
            if abs(float(d) - float(q(c["expected"]["ks_d"]))) > 1e-12:
                raise ValueError(
                    f"SciPy KS crosscheck exceeds frozen tolerance: {c['id']}"
                )
            row.update({"scipy_ks_d": float(d), "ks_d_tolerance": 1e-12})
        rows.append(row)
    return rows


def build() -> dict:
    fixtures = json.loads(FIXTURES.read_text())
    # Fail closed if an expected record was edited independently of the exact oracle.
    rebuilt = [
        make_fixture({"id": c["id"], "family": c["family"], "input": c["input"]})
        for c in fixtures["cases"]
    ]
    if rebuilt != fixtures["cases"]:
        raise ValueError("fixture expected records do not match Fraction oracle")
    return {
        "schema_version": "c41.reference.v1",
        "fixtures_sha256": sha(FIXTURES),
        "generator_sha256": sha(Path(__file__)),
        "wheel_lock_sha256": sha(LOCK),
        "wheel_provenance_sha256": sha(PROVENANCE),
        "exact_cases": [
            {"id": c["id"], "expected": c["expected"]} for c in fixtures["cases"]
        ],
        "scipy_crosschecks": scipy_crosschecks(fixtures),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    expected = canonical(build())
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != expected:
            print(
                "reference.json is stale; regenerate with generate_reference.py",
                file=sys.stderr,
            )
            return 1
        print(
            f"reproduced {len(json.loads(FIXTURES.read_text())['cases'])} exact cases and {len(json.loads(expected)['scipy_crosschecks'])} SciPy crosschecks"
        )
        return 0
    OUTPUT.write_bytes(expected)
    print(f"wrote {OUTPUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
