"""Fail-closed checks for Track 47 evidence collection."""

import importlib.util
from pathlib import Path
import json
import unittest


SCRIPT = Path(__file__).with_name("collect_evidence.py")
SPEC = importlib.util.spec_from_file_location("collect_evidence", SCRIPT)
collector = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(collector)


def sample_result() -> dict:
    rows = []
    for scaling in ("strong", "weak"):
        for lp_count in (4, 8, 16, 32):
            initial_events = 2_048 if scaling == "strong" else 128 * lp_count
            processed = initial_events * 2
            elapsed = 100_000
            rows.append(
                {
                    "scaling": scaling,
                    "lp_count": lp_count,
                    "initial_events": initial_events,
                    "expected_processed_events": processed,
                    "seed": 47_2026,
                    "sequential_ns": [elapsed],
                    "pdes_ns": [elapsed * 2],
                    "sequential_events_per_second": [processed * 1_000_000_000 / elapsed],
                    "pdes_events_per_second": [processed * 1_000_000_000 / (elapsed * 2)],
                    "parity": True,
                    "runtime_counters": {
                        "processed_events": processed,
                        "remote_events": initial_events,
                        "emitted_events": initial_events,
                        "null_messages": 1,
                        "rounds": 1,
                        "gvt_ticks": 1,
                        "worker_count": lp_count,
                    },
                }
            )
    return {
        "schema_version": "kairoecs.pdes.benchmark.v1",
        "seed": 47_2026,
        "repetitions": 1,
        "warmup_runs": 1,
        "strong_total_events": 2_048,
        "weak_events_per_lp": 128,
        "rows": rows,
    }


class CollectorValidation(unittest.TestCase):
    def test_execution_metadata_survives_json_serialization_and_identifies_input(self) -> None:
        scenario = {"seed": 47_2026, "lp_counts": [4, 8, 16, 32]}
        metadata = json.loads(json.dumps(collector.execution_metadata(scenario, 0)))
        self.assertEqual(metadata["working_directory"], ".")
        self.assertEqual(metadata["benchmark_exit_status"], 0)
        self.assertRegex(metadata["input_scenario_sha256"], r"^sha256:[0-9a-f]{64}$")
        reordered = {"lp_counts": [4, 8, 16, 32], "seed": 47_2026}
        self.assertEqual(metadata["input_scenario_sha256"], collector.execution_metadata(reordered, 0)["input_scenario_sha256"])
        changed = {**scenario, "seed": 1}
        self.assertNotEqual(metadata["input_scenario_sha256"], collector.execution_metadata(changed, 0)["input_scenario_sha256"])
        self.assertEqual(collector.execution_metadata(scenario, 7)["benchmark_exit_status"], 7)

    def test_rejects_dirty_non_owned_core_source(self) -> None:
        status = collector.normalize_git_status(" M crates/kairo-ecs-core/src/lib.rs\n")
        self.assertTrue(status.startswith(" M "))
        self.assertEqual(
            collector.non_evidence_source_changes(status),
            ["crates/kairo-ecs-core/src/lib.rs"],
        )

    def test_ignores_prior_generated_evidence_for_source_cleanliness(self) -> None:
        status = "?? benches/pdes/evidence/20261003T000000Z/result.json\n"
        self.assertEqual(collector.non_evidence_source_changes(status), [])

    def test_rejects_source_mutation_during_run(self) -> None:
        with self.assertRaisesRegex(ValueError, "source tree changed"):
            collector.require_unchanged_source("before", "after")

    def test_accepts_unchanged_source(self) -> None:
        collector.require_unchanged_source("same", "same")

    def test_rejects_wrong_seed_and_repetition_count(self) -> None:
        result = sample_result()
        with self.assertRaisesRegex(ValueError, "seed/repetitions"):
            collector.validate_result(json.dumps(result), repetitions=2, seed=47_2026)
        with self.assertRaisesRegex(ValueError, "seed/repetitions"):
            collector.validate_result(json.dumps(result), repetitions=1, seed=1)

    def test_rejects_incomplete_lp_matrix(self) -> None:
        result = sample_result()
        result["rows"].pop()
        with self.assertRaisesRegex(ValueError, "8 strong/weak"):
            collector.validate_result(json.dumps(result), repetitions=1, seed=47_2026)

    def test_rejects_parity_and_counter_drift(self) -> None:
        result = sample_result()
        result["rows"][0]["parity"] = False
        with self.assertRaisesRegex(ValueError, "parity failure"):
            collector.validate_result(json.dumps(result), repetitions=1, seed=47_2026)

        result = sample_result()
        result["rows"][0]["runtime_counters"]["processed_events"] -= 1
        with self.assertRaisesRegex(ValueError, "processed-event count"):
            collector.validate_result(json.dumps(result), repetitions=1, seed=47_2026)


if __name__ == "__main__":
    unittest.main()
