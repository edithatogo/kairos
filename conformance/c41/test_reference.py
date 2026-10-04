"""C4.1 reference oracle checks; these do not exercise a production metric."""

from __future__ import annotations

import copy
import importlib.util
import json
import math
import unittest
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "c41_reference", HERE / "generate_reference.py"
)
ref = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(ref)
FIXTURES = json.loads((HERE / "fixtures.json").read_text())


def compare_result(actual: dict, expected: dict) -> None:
    """Strict C4.1 candidate result comparator used only by self-tests."""
    allowed = set(expected)
    if set(actual) != allowed:
        raise ValueError("result fields differ")
    for key in allowed - {"w1", "ks_d"}:
        if actual[key] != expected[key]:
            raise ValueError(f"metadata differs: {key}")
    for key, tol in (
        (
            "w1",
            Fraction(1, 10**12) * max(Fraction(1), Fraction(expected["scale_ticks"])),
        ),
        ("ks_d", Fraction(1, 10**12)),
    ):
        value, target = actual[key], expected[key]
        if value is None or target is None:
            if value is not target:
                raise ValueError(f"null mismatch: {key}")
            continue
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
        ):
            raise ValueError(f"nonfinite or nonnumeric {key}")
        if (key == "w1" and value < 0) or (key == "ks_d" and not 0 <= value <= 1):
            raise ValueError(f"out of range: {key}")
        if abs(Fraction.from_float(float(value)) - Fraction(target)) > tol:
            raise ValueError(f"outside tolerance: {key}")


def compare_candidate(
    candidate: dict, fixtures: dict, *, self_test_mode: bool = False
) -> None:
    if candidate.get("schema_version") != "c41.candidate.v1":
        raise ValueError("schema")
    if candidate.get("fixture_sha256") != ref.sha(HERE / "fixtures.json"):
        raise ValueError("fixture hash")
    p = candidate.get("provenance", {})
    if any(
        not isinstance(p.get(k), str) or not p[k].strip()
        for k in ("producer_commit", "toolchain")
    ):
        raise ValueError("provenance")
    if p.get("algorithm_versions") != sorted(
        {c["input"]["algorithm_version"] for c in fixtures["cases"]}
    ):
        raise ValueError("algorithm versions")
    kind = p.get("kind")
    if self_test_mode:
        if kind != "comparator_mock":
            raise ValueError("self-test candidate must be labeled comparator_mock")
    else:
        if kind != "runtime_candidate":
            raise ValueError("runtime candidate required")
        if not isinstance(p.get("producer_api"), str) or not p["producer_api"].strip():
            raise ValueError("producer API")
        commit = p.get("producer_commit", "")
        if len(commit) != 40 or any(
            c not in "0123456789abcdef" for c in commit.lower()
        ):
            raise ValueError("producer commit")
    expected = {x["id"]: x["expected"] for x in fixtures["cases"]}
    rows = candidate.get("cases", [])
    ids = [x.get("id") for x in rows]
    if len(ids) != len(set(ids)) or set(ids) != set(expected):
        raise ValueError("case membership")
    for row in rows:
        compare_result(row.get("result", {}), expected[row["id"]])


class ReferenceTests(unittest.TestCase):
    def test_fixture_protocol_and_required_coverage(self):
        self.assertEqual(FIXTURES["schema_version"], "c41.fixtures.v1")
        cases = FIXTURES["cases"]
        ids = [c["id"] for c in cases]
        self.assertGreaterEqual(len(cases), 30)
        self.assertEqual(len(ids), len(set(ids)))
        families = {c["family"] for c in cases}
        required = {
            "shift",
            "identical",
            "ties",
            "unequal_populations",
            "weighted_variants",
            "zero_weights",
            "duration_scaling",
            "unsorted_permuted_symmetric",
            "fractional_signed_residual",
            "tail_w1_180_d_1_5",
            "empty",
            "null_invalid",
            "nonfinite_invalid",
            "weight_invalid",
            "coverage_censoring",
            "coverage_missing",
            "coverage_failed",
            "coverage_infeasible",
            "no_observed",
            "large_common_origin",
            "near_u128_limit",
            "u128_overflow",
            "precision_rejection",
            "conditional_group_opposite_dependence",
        }
        self.assertTrue(required <= families, required - families)
        for c in cases:
            self.assertEqual(c["expected"]["p_value"], None)
            self.assertTrue(
                all(
                    isinstance(v, str)
                    for v in c["expected"]["counts"].values()
                    if not isinstance(v, dict)
                )
            )
            d = c["expected"]["diagnostic"]["primary_dispositions"]
            self.assertEqual(
                sum(map(int, d.values())), int(c["expected"]["counts"]["eligible"])
            )
            self.assertEqual(
                int(c["expected"]["counts"]["cohort_total"]),
                int(c["expected"]["counts"]["eligible"])
                + int(c["expected"]["counts"]["excluded"]),
            )

    def test_fraction_oracle_ties_weighted_and_tail(self):
        self.assertEqual(
            ref.exact_metric(
                [Fraction(0), Fraction(0), Fraction(2)],
                [Fraction(0), Fraction(1), Fraction(1)],
            ),
            (Fraction(2, 3), Fraction(1, 3)),
        )
        tail = next(c for c in FIXTURES["cases"] if c["id"] == "tail-180")
        self.assertEqual(tail["expected"]["w1"], "180")
        self.assertEqual(tail["expected"]["ks_d"], "1/5")
        self.assertEqual(
            ref.exact_metric(
                [Fraction(0), Fraction(2)],
                [Fraction(1), Fraction(3)],
                [Fraction(1), Fraction(3)],
                [Fraction(2), Fraction(2)],
            )[0],
            Fraction(1),
        )

    def test_invalid_empty_precision_and_null_classification(self):
        by_id = {c["id"]: c["expected"] for c in FIXTURES["cases"]}
        self.assertEqual(by_id["empty-ref"]["status"], "insufficient_data")
        self.assertEqual(by_id["empty-both"]["status"], "empty")
        self.assertEqual(by_id["no-observed"]["status"], "empty")
        self.assertEqual(by_id["null-points"]["status"], "ok")
        for case_id in (
            "nan-value",
            "inf-value",
            "negative-weight",
            "zero-total-mass",
            "nonfinite-total-mass",
            "u128-overflow",
            "precision-reject",
        ):
            self.assertEqual(by_id[case_id]["status"], "invalid", case_id)
        self.assertEqual(by_id["near-u128"]["precision"], "exact_offsets")
        self.assertEqual(by_id["common-origin-scaled"]["w1"], "1")
        for c in FIXTURES["cases"]:
            if c["expected"]["status"] == "invalid":
                self.assertEqual(c["expected"]["counts"]["reference"], "0", c["id"])
                self.assertEqual(c["expected"]["counts"]["candidate"], "0", c["id"])
                diagnostic = c["expected"]["diagnostic"]
                primary = diagnostic["primary_dispositions"]
                self.assertEqual(primary["observed"], "0", c["id"])
                attempted = sum(
                    map(int, diagnostic["attempted_support_counts"].values())
                )
                self.assertEqual(primary["rejected_input"], str(attempted), c["id"])
                raw = sum(map(int, diagnostic["raw_input_counts"].values()))
                self.assertEqual(raw, attempted + int(primary["missing"]), c["id"])
                self.assertEqual(
                    sum(map(int, primary.values())),
                    int(c["expected"]["counts"]["eligible"]),
                )
            else:
                self.assertEqual(
                    c["expected"]["diagnostic"]["primary_dispositions"][
                        "rejected_input"
                    ],
                    "0",
                    c["id"],
                )
        seconds = by_id["duration-scaled"]
        minutes = by_id["duration-minute"]
        self.assertEqual(Fraction(seconds["w1"]) / Fraction(minutes["w1"]), 60)
        self.assertEqual(seconds["ks_d"], minutes["ks_d"])
        sec_case = next(c for c in FIXTURES["cases"] if c["id"] == "duration-scaled")
        min_case = next(c for c in FIXTURES["cases"] if c["id"] == "duration-minute")
        self.assertEqual(
            int(min_case["input"]["scale_ticks"]),
            60 * int(sec_case["input"]["scale_ticks"]),
        )
        for side in ("reference", "candidate"):
            self.assertEqual(
                [60 * Fraction(v) for v in min_case["input"][side]],
                [Fraction(v) for v in sec_case["input"][side]],
            )

    def test_coverage_is_additive_primary_and_retains_overlaps(self):
        for case_id in (
            "coverage-censored",
            "coverage-missing",
            "coverage-failed",
            "coverage-infeasible",
            "censor-warning-zero",
            "infeasible-warning-zero",
        ):
            c = next(c for c in FIXTURES["cases"] if c["id"] == case_id)
            self.assertEqual(c["expected"]["status"], "ok")
            if case_id.endswith("zero"):
                self.assertTrue(c["expected"]["warnings"])
        e = next(
            c["expected"] for c in FIXTURES["cases"] if c["id"] == "coverage-infeasible"
        )
        self.assertEqual(e["diagnostic"]["primary_dispositions"]["infeasible"], "1")
        self.assertEqual(
            e["diagnostic"]["overlap_counts"]["infeasible_and_observed"], "1"
        )
        self.assertEqual(e["counts"]["unmatched"], "0")
        self.assertEqual(e["counts"]["unmatched_estimand"], "not_applicable")

    def test_equal_marginals_have_opposite_conditional_distributions(self):
        case = next(c for c in FIXTURES["cases"] if c["id"] == "conditional-opposite")
        raw = case["input"]["grouped_observations"]
        ref_marginal = [Fraction(r["Y"]) for r in raw["reference"]]
        cand_marginal = [Fraction(r["Y"]) for r in raw["candidate"]]
        self.assertEqual(
            ref.exact_metric(ref_marginal, cand_marginal), (Fraction(0), Fraction(0))
        )
        groups = case["expected"]["diagnostic"]["groups"]
        self.assertEqual(
            [(g["status"], g["w1"]) for g in groups],
            [("ok", "1"), ("ok", "1"), ("empty", None)],
        )
        self.assertEqual(groups[2]["eligible_reference"], "0")
        self.assertEqual(groups[2]["eligible_candidate"], "0")
        for group in raw["prespecified_groups"][:2]:
            a = [Fraction(r["Y"]) for r in raw["reference"] if r["X"] == group]
            b = [Fraction(r["Y"]) for r in raw["candidate"] if r["X"] == group]
            self.assertEqual(ref.exact_metric(a, b)[0], Fraction(1))

    def test_wrong_distance_and_tie_logic_are_detected(self):
        c = next(c for c in FIXTURES["cases"] if c["id"] == "ties-aggregate")
        with self.assertRaises(ValueError):
            compare_result({**c["expected"], "w1": 99.0}, c["expected"])
        c = next(c for c in FIXTURES["cases"] if c["id"] == "tail-180")
        with self.assertRaises(ValueError):
            compare_result({**c["expected"], "ks_d": 0.25}, c["expected"])

    def test_rejects_independent_origins_and_coverage_omissions(self):
        original = next(
            c for c in FIXTURES["cases"] if c["id"] == "large-common-origin"
        )
        broken = copy.deepcopy(original)
        broken["input"]["reference_origin"] = "9007199254740992"
        with self.assertRaises(ValueError):
            ref.validate_input(broken["input"])
        e = copy.deepcopy(original["expected"])
        e["warnings"] = ["right_censored_present"]
        zero = next(c for c in FIXTURES["cases"] if c["id"] == "censor-warning-zero")
        with self.assertRaises(ValueError):
            compare_result(zero["expected"], e)

    def test_candidate_comparator_rejects_metadata_and_membership_mutations(self):
        candidate = {
            "schema_version": "c41.candidate.v1",
            "fixture_sha256": ref.sha(HERE / "fixtures.json"),
            "provenance": {
                "kind": "comparator_mock",
                "producer_commit": "mock-self-test",
                "toolchain": "mock",
                "algorithm_versions": sorted(
                    {c["input"]["algorithm_version"] for c in FIXTURES["cases"]}
                ),
            },
            "cases": [
                {
                    "id": c["id"],
                    "result": {
                        **c["expected"],
                        "w1": None
                        if c["expected"]["w1"] is None
                        else float(Fraction(c["expected"]["w1"])),
                        "ks_d": None
                        if c["expected"]["ks_d"] is None
                        else float(Fraction(c["expected"]["ks_d"])),
                    },
                }
                for c in FIXTURES["cases"]
            ],
        }
        with self.assertRaises(ValueError):
            compare_candidate(candidate, FIXTURES)
        compare_candidate(candidate, FIXTURES, self_test_mode=True)
        bad = copy.deepcopy(candidate)
        bad["cases"].pop()
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES, self_test_mode=True)
        bad = copy.deepcopy(candidate)
        bad["cases"][0]["result"]["unit"] = "unknown"
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES, self_test_mode=True)
        bad = copy.deepcopy(candidate)
        bad["cases"][0]["result"]["unreviewed"] = True
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES, self_test_mode=True)
        bad = copy.deepcopy(candidate)
        bad["provenance"]["algorithm_versions"] = "mock"
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES, self_test_mode=True)
        bad = copy.deepcopy(candidate)
        bad["cases"][0]["result"]["w1"] = -1.0
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES, self_test_mode=True)
        bad = copy.deepcopy(candidate)
        bad["provenance"].update({"kind": "runtime_candidate", "producer_api": ""})
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES)
        bad = copy.deepcopy(candidate)
        bad["provenance"].update(
            {
                "kind": "runtime_candidate",
                "producer_api": "candidate API",
                "producer_commit": "mock",
            }
        )
        with self.assertRaises(ValueError):
            compare_candidate(bad, FIXTURES)


if __name__ == "__main__":
    unittest.main()
