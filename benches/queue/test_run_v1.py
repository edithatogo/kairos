from __future__ import annotations

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = Path(__file__).with_name("run_v1.py")
SPEC = importlib.util.spec_from_file_location("q52_run_v1", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class RuntimeCollectorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / ".artifacts/q52-runtime")
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def executable(self, name: str, body: str) -> Path:
        path = self.root / name
        path.write_text("#!/usr/bin/env python3\n" + body)
        path.chmod(0o755)
        return path

    def success_program(self) -> str:
        return """import json, sys
args = dict(zip(sys.argv[1::2], sys.argv[2::2]))
scenario, n, resources, capacity = args["--scenario"], int(args["--n"]), int(args["--resources"]), int(args["--capacity"])
if scenario == "interruptions":
    events, retained, terminal = capacity + 2*n, capacity+n, n
    occupancy = [{"resource_index": 0, "active": capacity, "queued": 0}]
    preemptions = resumptions = completions = n
else:
    queues = [n//resources + int(i < n%resources) for i in range(resources)]
    if scenario == "churn":
        queues = [n//2//resources + int(i < (n//2)%resources) for i in range(resources)]
        events, retained, terminal = resources*capacity + 2*n, resources*capacity+n, n-n//2
    else:
        events, retained, terminal = resources*capacity+n, resources*capacity+n, 0
    occupancy = [{"resource_index": i, "active": capacity, "queued": queues[i]} for i in range(resources)]
    preemptions = resumptions = completions = 0
row = {"schema": 1, "status": "ok", "timing_scope": "test timing scope", "scenario": scenario,
       "n": n, "resources": resources, "capacity": capacity, "seed": int(args["--seed"]),
       "setup_ns": 100, "dispatch_ns": 200, "initial_dispatch_ns": 200, "churn_dispatch_ns": 0,
       "events_completed": events, "events_per_second": 60000000.0, "waiters_per_second": 50000000.0,
       "preemptions": preemptions, "resumptions": resumptions, "completions": completions,
       "retained_request_count": retained, "terminal_request_count": terminal, "occupancy": occupancy}
print(json.dumps(row))
"""

    def test_frozen_matrix_has_requested_runtime_dimensions(self) -> None:
        cases = runner.matrix_cases()
        self.assertEqual(len(cases), 39)
        self.assertEqual({case["n"] for case in cases}, {10, 1_000, 100_000})
        self.assertEqual({case["scenario"] for case in cases}, set(runner.SCENARIOS))
        self.assertEqual(
            {(case["resources"], case["capacity"]) for case in cases if case["scenario"] == "many_resources"},
            {(10, 1), (100, 1)},
        )

    def test_percentiles_use_nearest_rank_on_completed_samples(self) -> None:
        self.assertEqual(runner.percentile([4, 1, 3, 2, 5], 0.50), 3)
        self.assertEqual(runner.percentile([4, 1, 3, 2, 5], 0.95), 5)
        self.assertIsNone(runner.percentile([], 0.95))

    def test_rss_units_are_normalized_by_host(self) -> None:
        class Usage:
            ru_maxrss = 12

        self.assertEqual(runner.peak_rss_bytes(Usage(), "Darwin"), 12)
        self.assertEqual(runner.peak_rss_bytes(Usage(), "Linux"), 12_288)
        with self.assertRaises(RuntimeError):
            runner.peak_rss_bytes(Usage(), "Other")

    def test_repeat_records_child_row_and_per_process_rss(self) -> None:
        binary = self.executable("ok", self.success_program())
        case = {"scenario": "fifo", "n": 10, "resources": 1, "capacity": 1}
        result = runner.run_repeat(binary, case, 42, 2.0, 1, self.root / "success-repeat")
        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["returncode"], 0)
        self.assertGreater(result["peak_rss_bytes"], 0)
        self.assertEqual(result["row"]["n"], 10)
        self.assertEqual(len(result["stdout_sha256"]), 64)
        self.assertTrue((ROOT / result["stdout_path"]).is_file())

    def test_child_row_rejects_boolean_counts_nan_rates_and_inconsistent_totals(self) -> None:
        case = {"scenario": "fifo", "n": 10, "resources": 1, "capacity": 1}
        binary = self.executable("valid-for-mutation", self.success_program())
        result = runner.run_repeat(binary, case, 42, 2.0, 1, self.root / "valid-repeat")
        self.assertEqual(result["status"], "ok")
        valid = result["row"]
        for key, value in (("setup_ns", True), ("events_per_second", float("nan"))):
            invalid = dict(valid)
            invalid[key] = value
            with self.assertRaises(ValueError):
                runner._parse_child_row(json.dumps(invalid).encode(), case, 42)
        invalid = dict(valid)
        invalid["terminal_request_count"] = 1
        with self.assertRaises(ValueError):
            runner._parse_child_row(json.dumps(invalid).encode(), case, 42)

    def test_timeout_preserves_partial_output_and_is_not_a_pass(self) -> None:
        binary = self.executable("slow", "import sys, time\nprint('started', flush=True)\nprint('Q52_PHASE=dispatch', file=sys.stderr, flush=True)\ntime.sleep(2)\n")
        case = {"scenario": "fifo", "n": 10, "resources": 1, "capacity": 1}
        result = runner.run_repeat(binary, case, 42, 0.50, 1, self.root / "timeout-repeat")
        self.assertEqual(result["status"], "timeout")
        self.assertTrue(result["timed_out"])
        self.assertNotEqual(result["returncode"], 0)
        self.assertIn(b"started", (ROOT / result["stdout_path"]).read_bytes())
        self.assertIn("unresolved", result["error"])
        self.assertEqual(result["timeout_phase"], "dispatch")

    def test_nonzero_or_invalid_child_output_is_failure(self) -> None:
        case = {"scenario": "fifo", "n": 10, "resources": 1, "capacity": 1}
        nonzero = self.executable("exit", "import sys\nprint('diagnostic')\nsys.exit(7)\n")
        failed = runner.run_repeat(nonzero, case, 42, 2.0, 1, self.root / "exit-repeat")
        self.assertEqual(failed["status"], "failed")
        self.assertEqual(failed["returncode"], 7)
        invalid = self.executable("invalid", "print('not json')\n")
        malformed = runner.run_repeat(invalid, case, 42, 2.0, 1, self.root / "invalid-repeat")
        self.assertEqual(malformed["status"], "invalid_result")
        self.assertTrue(malformed["error"])

    def test_main_stops_repeats_for_timed_out_case_and_exits_nonzero(self) -> None:
        binary = self.executable("main-slow", "import time\nprint('partial', flush=True)\ntime.sleep(2)\n")
        output = io.StringIO()
        args = ["--binary", str(binary), "--scenario", "fifo", "--n", "10",
                "--resources", "1", "--capacity", "1", "--repeats", "5",
                "--timeout-seconds", "0.50", "--seed", "42",
                "--output-dir", str(self.root / "main-results")]
        with mock.patch.object(runner, "read_toolchain", return_value={"stdout": "test"}), \
             mock.patch.object(runner, "cpu_model", return_value="test-cpu"), \
             contextlib.redirect_stdout(output):
            exit_code = runner.main(args)
        summary_line = json.loads(output.getvalue().strip().splitlines()[-1])
        self.assertEqual(exit_code, 1)
        self.assertEqual(summary_line["status"], "unresolved")
        receipt = json.loads((ROOT / summary_line["result"]).read_text())
        self.assertEqual(receipt["status"], "unresolved")
        self.assertEqual(receipt["cases"][0]["attempted_repeats"], 1)
        self.assertEqual(receipt["cases"][0]["repeats"][0]["status"], "timeout")
        self.assertEqual(receipt["cases"][0]["repeats"][0]["timeout_phase"], "setup")


if __name__ == "__main__":
    unittest.main()
