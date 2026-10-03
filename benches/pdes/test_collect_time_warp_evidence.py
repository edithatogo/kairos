import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

MODULE_PATH = Path(__file__).with_name("collect_time_warp_evidence.py")
spec = importlib.util.spec_from_file_location("track48_collector", MODULE_PATH)
collector = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(collector)


class EvidenceValidationTests(unittest.TestCase):
    def document(self):
        rows = []
        for lp_count in (4, 8):
            shared_inputs = collector.expected_input_events(lp_count)
            committed = len(shared_inputs) * 2
            for profile, emit_percent in (("sparse", 10), ("dense", 80)):
                rows.append(
                    {
                        "profile": profile,
                        "emit_percent": emit_percent,
                        "lp_count": lp_count,
                        "roots_per_lp": 8,
                        "max_hops": 8,
                        "horizon": (lp_count - 1) * 64 + 7 * 4 + 8 + 2,
                        "seed": 48_2027,
                        "expected_committed_events": committed,
                        "input_events": [dict(event) for event in shared_inputs],
                        "conservative_ns": [100, 101, 102, 103, 104],
                        "optimistic_ns": [120, 121, 122, 123, 124],
                        "parity": True,
                        "conservative_counters": {
                            "processed_events": committed,
                            "remote_events": committed // 2,
                            "emitted_events": committed - len(shared_inputs),
                            "null_messages": 0,
                            "rounds": 9,
                            "spawned_worker_cohort_max": lp_count,
                        },
                        "optimistic_counters": {
                            "executions": committed + 12,
                            "first_attempt_executions": committed + 7,
                            "extra_executions": 12,
                            "replay_executions": 5,
                            "rollback_attempts": 2,
                            "rolled_back_events": 5,
                            "max_rollback_depth": 3,
                            "canceled_sends": 4,
                            "fossil_collected_events": committed,
                            "checkpoints_before_fossil": 16,
                            "fossil_collected_checkpoints": 12,
                            "committed_logical_ids": committed,
                        },
                    }
                )
        return {
            "schema_version": "kairoecs.pdes.time_warp_benchmark.v1",
            "seed": 48_2027,
            "warmup_runs": 1,
            "repetitions": 5,
            "timed_boundary": "runtime_run_call",
            "excluded_costs": [
                "process construction",
                "partition and topology construction",
                "initial event scheduling",
                "final state/report extraction",
                "parity and logical ID validation",
                "fossil collection",
            ],
            "profiles": rows,
        }

    def test_validates_fixed_workload_and_complete_profile_matrix(self):
        hashes = collector.validate_output(self.document())
        self.assertEqual(len(hashes), 4)
        self.assertEqual(hashes["sparse-lp4"], hashes["dense-lp4"])
        self.assertEqual(hashes["sparse-lp8"], hashes["dense-lp8"])

    def test_rejects_wrong_seed_or_root_event_route(self):
        document = self.document()
        document["profiles"][0]["input_events"][0]["destination"] = 1
        with self.assertRaisesRegex(ValueError, "initial event count, type, route"):
            collector.validate_output(document)
        document = self.document()
        document["seed"] = True
        with self.assertRaisesRegex(ValueError, "benchmark seed"):
            collector.validate_output(document)

    def test_rejects_missing_repeat_and_tampered_fixed_graph(self):
        document = self.document()
        document["profiles"][0]["optimistic_ns"].pop()
        with self.assertRaisesRegex(ValueError, "five positive raw durations"):
            collector.validate_output(document)
        document = self.document()
        document["profiles"][1]["input_events"][0]["value"] += 1
        with self.assertRaisesRegex(ValueError, "initial event count, type, route"):
            collector.validate_output(document)

    def test_rejects_inconsistent_execution_accounting_and_malformed_counters(self):
        document = self.document()
        document["profiles"][0]["optimistic_counters"]["first_attempt_executions"] += 1
        with self.assertRaisesRegex(ValueError, "first-attempt execution count"):
            collector.validate_output(document)
        document = self.document()
        document["profiles"][0]["optimistic_counters"] = []
        with self.assertRaisesRegex(ValueError, "optimistic counters must be an object"):
            collector.validate_output(document)

    def test_rejects_missing_rollback_stress_or_runtime_parity(self):
        document = self.document()
        document["profiles"][3]["optimistic_counters"]["replay_executions"] = 0
        document["profiles"][3]["optimistic_counters"]["first_attempt_executions"] = document["profiles"][3]["optimistic_counters"]["executions"]
        with self.assertRaisesRegex(ValueError, "did not exercise replay_executions"):
            collector.validate_output(document)
        document = self.document()
        document["profiles"][2]["parity"] = False
        with self.assertRaisesRegex(ValueError, "parity was not established"):
            collector.validate_output(document)

    def make_repo(self, root):
        files = (
            "Cargo.toml",
            "Cargo.lock",
            ".gitignore",
            "crates/kairo-ecs-pdes/Cargo.toml",
            "crates/kairo-ecs-pdes/benches/time_warp.rs",
            "crates/kairo-ecs-pdes/src/lib.rs",
            "crates/kairo-ecs-types/Cargo.toml",
            "crates/kairo-ecs-types/src/lib.rs",
            "crates/kairo-ecs-core/Cargo.toml",
            "crates/kairo-ecs-core/src/lib.rs",
            "benches/pdes/collect_time_warp_evidence.py",
            "benches/pdes/test_collect_time_warp_evidence.py",
        )
        for relative in files:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"fixture {relative}\n")
        (root / ".gitignore").write_text("/artifacts/\n")
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.name", "Collector Test"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(["git", "commit", "-qm", "fixture"], cwd=root, check=True)
        return files

    def runner_for(self, document, mutate=None):
        output = json.dumps(document)

        def run(_command, cwd, **_kwargs):
            if mutate:
                mutate(Path(cwd))
            return SimpleNamespace(returncode=0, stdout=output, stderr="")

        return run

    def test_clean_source_receipt_and_ignored_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            self.make_repo(root)
            (root / "artifacts").mkdir()
            (root / "artifacts" / "ignored.bin").write_bytes(b"artifact")
            artifacts = Path(temporary) / "evidence"
            result = collector.collect_run(
                root,
                artifacts,
                ["bench"],
                self.runner_for(self.document()),
                metadata={"toolchain": {}, "hardware": {}},
            )
            evidence = json.loads(result.read_text())
            self.assertEqual(evidence["head_before"], evidence["head_after"])
            self.assertEqual(evidence["status_before"], "")
            self.assertEqual(evidence["source_sha256_before"], evidence["source_sha256_after"])

    def test_dirty_untracked_source_is_rejected_with_attempt_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            self.make_repo(root)
            (root / "untracked.rs").write_text("uncommitted")
            artifacts = Path(temporary) / "evidence"
            with self.assertRaisesRegex(ValueError, "not clean"):
                collector.collect_run(
                    root,
                    artifacts,
                    ["bench"],
                    self.runner_for(self.document()),
                    metadata={"toolchain": {}, "hardware": {}},
                )
            self.assertTrue((artifacts / "time_warp_attempt.json").is_file())

    def test_same_head_source_edit_during_run_is_rejected_and_recorded(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            self.make_repo(root)
            source = root / "crates/kairo-ecs-pdes/src/lib.rs"
            artifacts = Path(temporary) / "evidence"

            def edit_source(_root):
                source.write_text(source.read_text() + "edited during benchmark\n")

            with self.assertRaisesRegex(ValueError, "source hashes changed"):
                collector.collect_run(
                    root,
                    artifacts,
                    ["bench"],
                    self.runner_for(self.document(), edit_source),
                    metadata={"toolchain": {}, "hardware": {}},
                )
            receipt = json.loads((artifacts / "time_warp_attempt.json").read_text())
            self.assertEqual(receipt["head_before"], receipt["head_after"])
            self.assertNotEqual(receipt["source_sha256_before"], receipt["source_sha256_after"])

    def test_tampered_workload_or_counters_saves_failed_attempt(self):
        for mutate in (
            lambda doc: doc["profiles"][0]["input_events"][0].update(source=3),
            lambda doc: doc["profiles"][0]["optimistic_counters"].update(extra_executions=0),
        ):
            with self.subTest(mutate=mutate), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary) / "repo"
                root.mkdir()
                self.make_repo(root)
                doc = self.document()
                mutate(doc)
                artifacts = Path(temporary) / "evidence"
                with self.assertRaisesRegex(ValueError, "output validation failed"):
                    collector.collect_run(
                        root,
                        artifacts,
                        ["bench"],
                        self.runner_for(doc),
                        metadata={"toolchain": {}, "hardware": {}},
                    )
                receipt = json.loads((artifacts / "time_warp_attempt.json").read_text())
                self.assertIn("validation_error", receipt)
                self.assertEqual(receipt["head_before"], receipt["head_after"])


if __name__ == "__main__":
    unittest.main()
