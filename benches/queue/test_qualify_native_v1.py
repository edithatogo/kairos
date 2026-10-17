from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

MODULE_PATH = Path(__file__).with_name("qualify_native_v1.py")
SPEC = importlib.util.spec_from_file_location("q52_native_qualifier", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
qualifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(qualifier)


class NativeQualificationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="q52-native-qualifier-")
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_estimates(self, side: str = "F") -> Path:
        criterion_root = self.root / "criterion" / side
        for index, (bench, group, function) in enumerate(qualifier.CANONICAL.values(), start=1):
            path = criterion_root / bench / group / function / "new" / "estimates.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({"mean": {"point_estimate": index * 1_000_000.0}}))
        return criterion_root

    def test_normalizes_exact_six_estimates_to_comparator_schema(self) -> None:
        root = self.write_estimates()
        normalized, hashes = qualifier.normalize_results(root)
        self.assertEqual(len(normalized["benchmarks"]), 6)
        self.assertEqual({row["id"] for row in normalized["benchmarks"]}, set(qualifier.CANONICAL))
        self.assertEqual(normalized["benchmarks"][0], {"id": "schedule_1m_events", "mean_seconds": 0.001})
        self.assertEqual(len(hashes), 6)
        for row in hashes:
            self.assertEqual(len(row["sha256"]), 64)

    def test_missing_estimate_fails_closed(self) -> None:
        root = self.write_estimates()
        path = root / "scheduler" / "pop_1m_events" / "pop" / "new" / "estimates.json"
        path.unlink()
        with self.assertRaisesRegex(qualifier.QualificationError, "missing Criterion"):
            qualifier.normalize_results(root)

    def test_bad_estimate_types_and_nonfinite_values_fail_closed(self) -> None:
        path = self.root / "estimate.json"
        for value in (True, 0, -1, float("nan"), float("inf"), "3", None):
            path.write_text(json.dumps({"mean": {"point_estimate": value}}))
            with self.subTest(value=value), self.assertRaises(qualifier.QualificationError):
                qualifier.parse_mean(path)

    def test_malformed_and_absent_estimate_fail_closed(self) -> None:
        path = self.root / "bad.json"
        path.write_text("{")
        with self.assertRaisesRegex(qualifier.QualificationError, "malformed"):
            qualifier.parse_mean(path)
        path.write_text(json.dumps({"mean": None}))
        with self.assertRaisesRegex(qualifier.QualificationError, "numeric"):
            qualifier.parse_mean(path)
        with self.assertRaisesRegex(qualifier.QualificationError, "missing"):
            qualifier.parse_mean(self.root / "absent.json")

    def test_normalized_rows_reject_missing_duplicate_unknown_and_invalid_means(self) -> None:
        valid = {"benchmarks": [{"id": key, "mean_seconds": 1.0} for key in qualifier.CANONICAL]}
        qualifier.validate_result_records(valid)
        cases = [
            {"benchmarks": valid["benchmarks"][:-1]},
            {"benchmarks": valid["benchmarks"] + [valid["benchmarks"][0]]},
            {"benchmarks": [{**valid["benchmarks"][0], "id": "not_canonical"}] + valid["benchmarks"][1:]},
            {"benchmarks": [{**valid["benchmarks"][0], "mean_seconds": True}] + valid["benchmarks"][1:]},
            {"benchmarks": [{**valid["benchmarks"][0], "mean_seconds": float("nan")}] + valid["benchmarks"][1:]},
        ]
        for payload in cases:
            with self.subTest(payload=payload), self.assertRaises(qualifier.QualificationError):
                qualifier.validate_result_records(payload)

    def test_lockstep_build_and_contract_hashes_must_match(self) -> None:
        baseline = {name: "a" * 64 for name in qualifier.LOCKSTEP_PATHS}
        current = dict(baseline)
        qualifier.require_lockstep(baseline, current)
        current["Cargo.lock"] = "b" * 64
        with self.assertRaisesRegex(qualifier.QualificationError, "Cargo.lock"):
            qualifier.require_lockstep(baseline, current)
        current["Cargo.lock"] = None
        with self.assertRaises(qualifier.QualificationError):
            qualifier.require_lockstep(baseline, current)

    def test_frozen_threshold_and_harness_hashes_must_match(self) -> None:
        baseline = {name: "a" * 64 for name in qualifier.LOCKSTEP_PATHS}
        current = dict(baseline)
        threshold = "conductor/performance-thresholds.md"
        current[threshold] = "b" * 64
        with self.assertRaisesRegex(qualifier.QualificationError, "performance-thresholds"):
            qualifier.require_lockstep(baseline, current)
        before = {"qualification_script_sha256": "a" * 64, "qualification_test_sha256": "b" * 64}
        after = dict(before, qualification_test_sha256="c" * 64)
        with self.assertRaisesRegex(qualifier.QualificationError, "qualification_test_sha256"):
            qualifier.require_unchanged_hashes(before, after, "qualification implementation")

    def test_baseline_and_current_head_must_match_expected_exact_hash(self) -> None:
        qualifier.require_head("a" * 40, "a" * 40, "current")
        for actual, expected, label in (("b" * 40, "a" * 40, "current"),
                                        ("b" * 40, qualifier.ACCEPTED_F, "accepted baseline")):
            with self.subTest(label=label), self.assertRaisesRegex(qualifier.QualificationError, "HEAD mismatch"):
                qualifier.require_head(actual, expected, label)

    def test_command_evidence_requires_existing_hashed_success_log(self) -> None:
        log = self.root / "commands" / "bench.log"
        log.parent.mkdir()
        log.write_text("Finished bench profile\n")
        record = {"name": "F-scheduler", "log": str(log), "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest(), "exit_code": 0}
        qualifier.validate_command_evidence(record, self.root)
        with self.assertRaisesRegex(qualifier.QualificationError, "hash mismatch"):
            qualifier.validate_command_evidence({**record, "log_sha256": "0" * 64}, self.root)
        with self.assertRaisesRegex(qualifier.QualificationError, "missing command log"):
            qualifier.validate_command_evidence({**record, "log": str(self.root / "missing.log")}, self.root)
        with self.assertRaisesRegex(qualifier.QualificationError, "did not complete"):
            qualifier.validate_command_evidence({**record, "exit_code": 1}, self.root)

    def test_run_command_timeout_preserves_partial_log_and_marks_failure(self) -> None:
        log = self.root / "timeout.log"
        error = qualifier.subprocess.TimeoutExpired(["cargo", "bench"], 1, output=b"partial output")
        with mock.patch.object(qualifier.subprocess, "run", side_effect=error):
            result = qualifier.run_command(["cargo", "bench"], self.root, log, 1)
        self.assertEqual(log.read_bytes(), b"partial output")
        self.assertTrue(result["timed_out"])
        self.assertIsNone(result["exit_code"])
        self.assertEqual(result["log_sha256"], hashlib.sha256(b"partial output").hexdigest())

    def test_comparator_uses_argv_signature_and_preserves_threshold_failure(self) -> None:
        artifact_root = self.root / "artifacts"
        artifact_root.mkdir()
        log = artifact_root / "logs" / "canonical-comparator.log"
        receipt = {"commands": []}

        def failed_comparison(argv, cwd, log_path, timeout_seconds, env_overrides=None):
            log_path.parent.mkdir(parents=True)
            log_path.write_text("threshold exceeded\n")
            (artifact_root / "comparison.json").write_text(json.dumps({"status": "fail"}))
            return {
                "argv": argv, "cwd": str(cwd), "timeout_seconds": timeout_seconds,
                "env_overrides": env_overrides or {}, "log": str(log_path),
                "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest(),
                "exit_code": 1, "output": "threshold exceeded\n",
            }

        with mock.patch.object(qualifier, "run_command", side_effect=failed_comparison) as command:
            with self.assertRaisesRegex(qualifier.QualificationError, "thresholds remain blocking"):
                qualifier.run_canonical_comparator(artifact_root, receipt)

        argv, cwd, actual_log, timeout, *rest = command.call_args.args
        self.assertEqual(argv, [
            qualifier.sys.executable, str(qualifier.ROOT / "benches/regression/compare.py"),
            "--base", str(artifact_root / "baseline.json"),
            "--current", str(artifact_root / "current.json"),
            "--report", str(artifact_root / "comparison.json"),
        ])
        self.assertEqual(cwd, qualifier.ROOT)
        self.assertEqual(actual_log, log)
        self.assertEqual(timeout, 120)
        self.assertEqual(rest, [])
        self.assertEqual(receipt["status"], "threshold_failure")
        self.assertEqual(receipt["comparison"]["status"], "fail")
        self.assertEqual(receipt["commands"][0]["name"], "canonical-comparator")

    def test_executable_must_be_actual_running_path_inside_owned_release_target(self) -> None:
        target = self.root / "target"
        deps = target / "release" / "deps"
        deps.mkdir(parents=True)
        binary = deps / "scheduler-abcd"
        binary.write_bytes(b"binary")
        text = f"Running benches/scheduler.rs ({binary})\n"
        self.assertEqual(qualifier.executable_from_running(text, target, self.root), binary.resolve())
        with self.assertRaisesRegex(qualifier.QualificationError, "found 0"):
            qualifier.executable_from_running("Running benches/scheduler.rs (/tmp/not-owned)\n", target, self.root)
        with self.assertRaisesRegex(qualifier.QualificationError, "found 0"):
            qualifier.executable_from_running("no Running line\n", target, self.root)

    def test_prebuild_requires_three_actual_executable_files(self) -> None:
        target = self.root / "target"
        deps = target / "release" / "deps"
        deps.mkdir(parents=True)
        paths = {}
        lines = []
        for name in qualifier.BENCHES:
            binary = deps / f"{name}-hash"
            binary.write_bytes(name.encode())
            paths[name] = binary
            lines.append(f"  Executable benches/{name}.rs ({binary})")
        built = qualifier.parse_prebuild_executables("\n".join(lines), target, self.root)
        self.assertEqual(set(built), set(qualifier.BENCHES))
        with self.assertRaisesRegex(qualifier.QualificationError, "all three"):
            qualifier.parse_prebuild_executables(lines[0], target, self.root)

    def test_workflow_keeps_q52_gate_conditional_and_artifact_upload_excludes_targets(self) -> None:
        workflow = (qualifier.ROOT / ".github/workflows/careops-native-owner.yml").read_text()
        self.assertIn("Q5.2 canonical native regression (Ubuntu)", workflow)
        self.assertIn("needs: [native, arrow-io-floor, q52-native-regression]", workflow)
        self.assertIn("Q52_REQUIRED", workflow)
        self.assertIn("benches/queue/qualify_native_v1.py", workflow)
        self.assertIn("benches/queue/test_qualify_native_v1.py", workflow)
        self.assertIn("!.artifacts/ci/q52-native/target/**", workflow)
        self.assertIn("retention-days: 7", workflow)


if __name__ == "__main__":
    unittest.main()
