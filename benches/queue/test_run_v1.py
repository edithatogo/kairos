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
        (ROOT / ".artifacts/q52-runtime").mkdir(parents=True, exist_ok=True)
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
slots = resources*capacity
if scenario == "interruptions":
    events, records, retained, terminal, works = capacity+2*n, 2*capacity+5*n, capacity+n, n, capacity+n
    work_states = {"pending": 0, "active": capacity, "suspended": 0, "completed": n, "other_terminal": 0}
    occupancy = [{"resource_index": 0, "active": capacity, "queued": 0}]
    preemptions = resumptions = completions = n
elif scenario == "churn":
    rekeyed = n//2
    events, records, retained, terminal, works = slots+2*n, n+2*slots+rekeyed, slots+n, n-rekeyed, 0
    work_states = {"pending": 0, "active": 0, "suspended": 0, "completed": 0, "other_terminal": 0}
    occupancy = [{"resource_index": i, "active": capacity,
                  "queued": rekeyed//resources + int(i < rekeyed%resources)} for i in range(resources)]
    preemptions = resumptions = completions = 0
else:
    events, records, retained, terminal, works = n+slots, n+2*slots, n+slots, 0, 0
    work_states = {"pending": 0, "active": 0, "suspended": 0, "completed": 0, "other_terminal": 0}
    occupancy = [{"resource_index": i, "active": capacity,
                  "queued": n//resources + int(i < n%resources)} for i in range(resources)]
    preemptions = resumptions = completions = 0
row = {"schema": 1, "status": "ok", "timing_scope": "test timing scope", "scenario": scenario,
       "n": n, "resources": resources, "capacity": capacity, "seed": int(args["--seed"]),
       "setup_ns": 100, "dispatch_ns": 200, "initial_dispatch_ns": 200, "churn_dispatch_ns": 0,
       "events_completed": events, "lifecycle_records": records, "logical_waiters": n,
       "events_per_second": 60000000.0, "waiters_per_second": 50000000.0,
       "preemptions": preemptions, "resumptions": resumptions, "completions": completions,
       "retained_request_count": retained, "terminal_request_count": terminal,
       "retained_work_count": works, "work_states": work_states, "occupancy": occupancy}
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

    def test_validator_accepts_expected_rows_for_every_frozen_case(self) -> None:
        for case in runner.matrix_cases():
            expected = runner._expected_counts(case)
            row = {"schema": 1, "status": "ok", "timing_scope": "test timing scope",
                   **case, "seed": 42, "setup_ns": 1, "dispatch_ns": 2,
                   "initial_dispatch_ns": 2, "churn_dispatch_ns": 0,
                   "events_per_second": 1.0, "waiters_per_second": 1.0, **expected,
                   "occupancy": [{"resource_index": i, "active": active, "queued": queued}
                                 for i, (active, queued) in enumerate(expected["occupancy"])]}
            with self.subTest(case=case):
                self.assertEqual(runner._parse_child_row((json.dumps(row)+"\n").encode(), case, 42), row)

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
        for key, value in (("schema", True), ("n", True), ("setup_ns", True),
                           ("events_per_second", float("nan"))):
            invalid = dict(valid)
            invalid[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                runner._parse_child_row(json.dumps(invalid).encode(), case, 42)
        mutations = []
        invalid = dict(valid)
        invalid["terminal_request_count"] = 1
        mutations.append(invalid)
        invalid = dict(valid)
        invalid["lifecycle_records"] += 1
        mutations.append(invalid)
        invalid = dict(valid)
        invalid["work_states"] = dict(valid["work_states"], active=True)
        mutations.append(invalid)
        invalid = dict(valid)
        invalid["occupancy"] = [dict(valid["occupancy"][0], queued=True)]
        mutations.append(invalid)
        for invalid in mutations:
            with self.subTest(row=invalid):
                with self.assertRaises(ValueError):
                    runner._parse_child_row(json.dumps(invalid).encode(), case, 42)

    def test_expected_counts_cover_each_scenario_and_interruption_work_lifecycle(self) -> None:
        for case in runner.matrix_cases():
            expected = runner._expected_counts(case)
            self.assertEqual(expected["logical_waiters"], case["n"])
            self.assertEqual(sum(q for _, q in expected["occupancy"]),
                             case["n"] // 2 if case["scenario"] == "churn" else (0 if case["scenario"] == "interruptions" else case["n"]))
            self.assertEqual(sum(expected["work_states"].values()), expected["retained_work_count"])
        interruption = runner._expected_counts({"scenario": "interruptions", "n": 10, "resources": 1, "capacity": 10})
        self.assertEqual(interruption["lifecycle_records"], 70)
        self.assertEqual(interruption["work_states"], {"pending": 0, "active": 10, "suspended": 0,
                                                       "completed": 10, "other_terminal": 0})

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

    def test_source_provenance_qualification_fails_closed_on_dirty_or_unverified_inputs(self) -> None:
        hashes = {name: "a"*64 for name in runner.BUILD_INPUT_PATHS}
        source_hashes = {name: hashes[name] for name in runner.SOURCE_PATHS}
        readbacks = {"head": 0, "status": 0}
        build = {"git_status_before": "", "git_status_after": "", "head_before": "head",
                 "head_after": "head", "head_drift": False, "source_drift": False,
                 "git_readback_returncodes_before": readbacks, "git_readback_returncodes_after": readbacks,
                 "source_hashes_after": hashes, "binary_sha256": "binsha"}
        start = {"git_status_porcelain": "", "git_head": "head", "git_readback_returncodes": readbacks,
                 "source_hashes": source_hashes}
        end = dict(start)
        toolchain = {"returncode": 0, "stdout": "rustc 1.99.0 (test)\nrelease: 1.99.0"}
        self.assertTrue(runner.bounded_measurement_provenance_qualified(
            build, start, end, "binsha", "binsha", toolchain))
        dirty_build = dict(build, git_status_before=" M source")
        self.assertFalse(runner.bounded_measurement_provenance_qualified(
            dirty_build, start, end, "binsha", "binsha", toolchain))
        bad_readback = dict(start, git_readback_returncodes={"head": 0, "status": 1})
        self.assertFalse(runner.bounded_measurement_provenance_qualified(
            build, bad_readback, end, "binsha", "binsha", toolchain))
        wrong_toolchain = dict(toolchain, stdout="rustc 1.98.0 (test)")
        self.assertFalse(runner.bounded_measurement_provenance_qualified(
            build, start, end, "binsha", "binsha", wrong_toolchain))
        self.assertFalse(runner.bounded_measurement_provenance_qualified(
            build, start, end, "binsha", "changed", toolchain))

    def test_source_provenance_requires_complete_valid_runtime_and_build_hashes(self) -> None:
        hashes = {name: f"{index:064x}" for index, name in enumerate(runner.BUILD_INPUT_PATHS, 1)}
        source_hashes = {name: hashes[name] for name in runner.SOURCE_PATHS}
        readbacks = {"head": 0, "status": 0}
        build = {"git_status_before": "", "git_status_after": "", "head_before": "head",
                 "head_after": "head", "head_drift": False, "source_drift": False,
                 "git_readback_returncodes_before": readbacks, "git_readback_returncodes_after": readbacks,
                 "source_hashes_after": hashes, "binary_sha256": "binsha"}
        start = {"git_status_porcelain": "", "git_head": "head", "git_readback_returncodes": readbacks,
                 "source_hashes": source_hashes}
        end = dict(start, source_hashes=dict(source_hashes))
        toolchain = {"returncode": 0, "stdout": "rustc 1.99.0 (test)\nrelease: 1.99.0"}

        def qualifies(candidate_build=build, candidate_start=start, candidate_end=end):
            return runner.bounded_measurement_provenance_qualified(
                candidate_build, candidate_start, candidate_end, "binsha", "binsha", toolchain)

        self.assertTrue(qualifies())
        helper_paths = tuple(path for path in runner.SOURCE_PATHS
                             if path not in ("AGENTS.md", "crates/kairo-ecs-des/src/flow.rs",
                                             "crates/kairo-ecs-des/src/lib.rs",
                                             "crates/kairo-ecs-des/examples/flow_queue_benchmark_v1.rs",
                                             "benches/queue/run_v1.py"))
        self.assertEqual(len(helper_paths), 5)

        for path in helper_paths:
            with self.subTest(missing_from="build", path=path):
                changed = dict(build, source_hashes_after=dict(hashes))
                changed["source_hashes_after"].pop(path)
                self.assertFalse(qualifies(candidate_build=changed))
            for location in ("start", "end"):
                with self.subTest(missing_from=location, path=path):
                    changed_sources = dict(source_hashes)
                    changed_sources.pop(path)
                    changed_start = dict(start, source_hashes=changed_sources) if location == "start" else start
                    changed_end = dict(end, source_hashes=changed_sources) if location == "end" else end
                    self.assertFalse(qualifies(candidate_start=changed_start, candidate_end=changed_end))
            with self.subTest(changed_between_start_end=path):
                changed_sources = dict(source_hashes)
                changed_sources[path] = "f" * 64
                self.assertFalse(qualifies(candidate_end=dict(end, source_hashes=changed_sources)))

        with self.subTest(missing_from="build", path="rust-toolchain.toml"):
            changed = dict(build, source_hashes_after=dict(hashes))
            changed["source_hashes_after"].pop("rust-toolchain.toml")
            self.assertFalse(qualifies(candidate_build=changed))

        for malformed in ("", "g" * 64, "A" * 64, None, 42):
            with self.subTest(location="start", malformed=malformed):
                changed_sources = dict(source_hashes)
                changed_sources[helper_paths[0]] = malformed
                self.assertFalse(qualifies(candidate_start=dict(start, source_hashes=changed_sources)))
            with self.subTest(location="end", malformed=malformed):
                changed_sources = dict(source_hashes)
                changed_sources[helper_paths[0]] = malformed
                self.assertFalse(qualifies(candidate_end=dict(end, source_hashes=changed_sources)))
            with self.subTest(location="build", malformed=malformed):
                changed_hashes = dict(hashes)
                changed_hashes["rust-toolchain.toml"] = malformed
                self.assertFalse(qualifies(candidate_build=dict(build, source_hashes_after=changed_hashes)))

    def test_unsupported_rss_platform_is_explicitly_rejected(self) -> None:
        with mock.patch.object(runner.platform, "system", return_value="Windows"):
            self.assertIn("units are not qualified", runner.measurement_platform_error())
        with mock.patch.object(runner.os, "wait4", None):
            self.assertIn("requires POSIX os.wait4", runner.measurement_platform_error())

    def test_build_binary_path_must_match_reviewed_target_layout(self) -> None:
        expected = ROOT / "target/release/examples/flow_queue_benchmark_v1"
        self.assertEqual(runner.build_target_for_binary(Path("target/release/examples/flow_queue_benchmark_v1")),
                         (ROOT / "target").resolve())
        self.assertEqual(runner.build_target_for_binary(None), (ROOT / runner.DEFAULT_TARGET_DIR).resolve())
        with self.assertRaises(ValueError):
            runner.build_target_for_binary(expected.parent / "other-benchmark")

    def test_build_receipt_records_actual_argv_inputs_and_binary_hash(self) -> None:
        target = self.root / "target"
        binary = target / "release/examples/flow_queue_benchmark_v1"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"reviewed fake executable")
        binary.chmod(0o755)
        snapshot = {"git_head": "test-head", "git_status_porcelain": "",
                    "git_readback_returncodes": {"head": 0, "status": 0},
                    "source_hashes": {name: "a"*64 for name in runner.BUILD_INPUT_PATHS}}
        with mock.patch.object(runner, "provenance_snapshot", return_value=snapshot), \
             mock.patch.object(runner.subprocess, "run", return_value=mock.Mock(returncode=0, stdout="built", stderr="")) as run:
            receipt = runner.execute_release_build(target, self.root / "build-receipt")
        self.assertEqual(receipt["exit_code"], 0)
        self.assertEqual(receipt["source_hashes_before"], receipt["source_hashes_after"])
        self.assertEqual(receipt["binary_sha256"], runner.sha256_file(binary))
        self.assertEqual(receipt["argv"][-2:], ["--target-dir", runner._display_path(target)])
        self.assertIn("Cargo.lock", receipt["source_hashes_before"])
        self.assertIn("Cargo.toml", receipt["source_hashes_before"])
        self.assertIn("crates/kairo-ecs-des/Cargo.toml", receipt["source_hashes_before"])
        self.assertEqual(receipt["package"], {"name": "kairo-ecs-des", "version": "0.1.0"})
        self.assertEqual(receipt["build_profile"], "release")
        self.assertTrue(receipt["feature_selection"]["default_features"])
        self.assertEqual(run.call_args.args[0], receipt["argv"])
        self.assertTrue((self.root / "build-receipt/build.json").is_file())

    def test_main_rejects_nan_timeout_before_launch(self) -> None:
        output = io.StringIO()
        with mock.patch.object(runner, "measurement_platform_error", return_value=None), contextlib.redirect_stderr(output):
            exit_code = runner.main(["--binary", "/missing", "--scenario", "fifo", "--n", "10",
                                     "--resources", "1", "--capacity", "1", "--timeout-seconds", "nan",
                                     "--output-dir", str(self.root / "nan")])
        self.assertEqual(exit_code, 2)
        self.assertIn("timeout", output.getvalue())

    def test_build_launch_failure_is_recorded_and_not_treated_as_a_binary(self) -> None:
        target = self.root / "missing-target"
        with mock.patch.object(runner, "provenance_snapshot", return_value={
            "git_head": "head", "git_status_porcelain": "",
            "source_hashes": {name: "b"*64 for name in runner.BUILD_INPUT_PATHS},
            "git_readback_returncodes": {"head": 0, "status": 0},
        }), mock.patch.object(runner.subprocess, "run", side_effect=FileNotFoundError("rustup missing")):
            receipt = runner.execute_release_build(target, self.root / "build-failure")
        self.assertIsNone(receipt["exit_code"])
        self.assertIn("launch failed", receipt["error"])
        self.assertFalse(receipt["binary_executable"])
        self.assertTrue((self.root / "build-failure/build.json").is_file())

    def test_main_does_not_run_stale_binary_when_requested_build_fails(self) -> None:
        binary = ".artifacts/q52-runtime/target/release/examples/flow_queue_benchmark_v1"
        output = io.StringIO()
        build_receipt = {"exit_code": 1, "source_drift": False, "head_drift": False,
                         "binary_executable": True, "receipt_path": "build/build.json"}
        args = ["--build", "--binary", binary, "--scenario", "fifo", "--n", "10",
                "--resources", "1", "--capacity", "1", "--output-dir", str(self.root / "stale-build")]
        with mock.patch.object(runner, "measurement_platform_error", return_value=None), \
             mock.patch.object(runner, "execute_release_build", return_value=build_receipt), \
             mock.patch.object(runner, "run_repeat", side_effect=AssertionError("stale child launched")), \
             contextlib.redirect_stdout(output):
            exit_code = runner.main(args)
        self.assertEqual(exit_code, 1)
        self.assertEqual(json.loads(output.getvalue())["status"], "build_failed")

    def test_main_returns_nonzero_before_child_spawn_on_unsupported_platform(self) -> None:
        binary = self.executable("platform-check", self.success_program())
        output = io.StringIO()
        with mock.patch.object(runner.platform, "system", return_value="Windows"), \
             contextlib.redirect_stderr(output), \
             mock.patch.object(runner, "run_repeat", side_effect=AssertionError("child launched")):
            exit_code = runner.main(["--binary", str(binary), "--scenario", "fifo", "--n", "10",
                                     "--resources", "1", "--capacity", "1",
                                     "--output-dir", str(self.root / "unsupported")])
        self.assertEqual(exit_code, 2)
        self.assertIn("unsupported collector platform", output.getvalue())

    def test_main_stops_repeats_for_timed_out_case_and_exits_nonzero(self) -> None:
        binary = self.executable("main-slow", "import time\nprint('partial', flush=True)\ntime.sleep(2)\n")
        output = io.StringIO()
        args = ["--binary", str(binary), "--scenario", "fifo", "--n", "10",
                "--resources", "1", "--capacity", "1", "--repeats", "5",
                "--timeout-seconds", "0.50", "--seed", "42",
                "--output-dir", str(self.root / "main-results")]
        # This is a synthetic main-path timeout test. Keep its provenance inputs
        # synthetic too, so it does not require future runtime modules to exist.
        provenance = {"git_head": "test-head", "git_status_porcelain": "",
                      "git_readback_returncodes": {"head": 0, "status": 0},
                      "source_hashes": {name: "c"*64 for name in runner.SOURCE_PATHS}}
        with mock.patch.object(runner, "read_toolchain", return_value={"stdout": "test"}), \
             mock.patch.object(runner, "cpu_model", return_value="test-cpu"), \
             mock.patch.object(runner, "provenance_snapshot", return_value=provenance), \
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
        self.assertEqual(receipt["binary_producer_provenance"], "unverified_external_binary")
        self.assertEqual(receipt["input_manifest"]["binary_producer_provenance"], "unverified_external_binary")
        self.assertFalse(receipt["bounded_measurement_provenance_qualified"])
        self.assertEqual(receipt["q52_acceptance"], "not assessed by runtime collector; requires the complete matrix and coordinator thresholds")


if __name__ == "__main__":
    unittest.main()
