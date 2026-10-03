import importlib.util
import hashlib
import json
import os
import shutil
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

    def runner_for(self, document, mutate=None, inspect=None):
        output = json.dumps(document)

        def run(command, cwd, env, **_kwargs):
            if mutate:
                mutate(Path(cwd))
            if inspect:
                inspect(command, env)
            return SimpleNamespace(returncode=0, stdout=output, stderr="")

        return run

    def fake_toolchain_resolver(self, location, source_root, base_env):
        sysroot = location / "sysroot"
        toolchain_bin = sysroot / "bin"
        toolchain_bin.mkdir(parents=True, exist_ok=True)
        rustc = toolchain_bin / "rustc"
        cargo = toolchain_bin / "cargo"
        rustc.write_text("test rustc binary")
        cargo.write_text("test cargo binary")
        rustc.chmod(0o755)
        cargo.chmod(0o755)
        return {
            "toolchain": "1.98.1",
            "source_root": str(Path(source_root).resolve()),
            "rustup_path": "/test/rustup",
            "sysroot": str(sysroot.resolve()),
            "toolchain_bin": str(toolchain_bin.resolve()),
            "rustc_path": str(rustc.resolve()),
            "rustc_version": "rustc 1.98.1 (test)\nrelease: 1.98.1",
            "rustc_sha256": hashlib.sha256(rustc.read_bytes()).hexdigest(),
            "cargo_path": str(cargo.resolve()),
            "cargo_version": "cargo 1.98.1 (test)",
            "cargo_verbose_version": "cargo 1.98.1 (test)\nrelease: 1.98.1",
            "cargo_sha256": hashlib.sha256(cargo.read_bytes()).hexdigest(),
            "cargo_config_sha256": collector.cargo_wrapper_config_hashes(
                Path(source_root), base_env
            ),
            "disabled_wrapper_environment": [
                key for key in collector.WRAPPER_ENVIRONMENT_KEYS if base_env.get(key)
            ],
            "cleared_empty_rustflags_environment": collector.rustflag_environment_keys(
                base_env
            ),
        }

    def test_clean_source_receipt_and_ignored_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            self.make_repo(root)
            (root / "artifacts").mkdir()
            (root / "artifacts" / "ignored.bin").write_bytes(b"artifact")
            artifacts = Path(temporary) / "evidence"
            provenance = self.fake_toolchain_resolver(
                Path(temporary) / "toolchain", root, os.environ
            )

            def inspect_exact_compiler(command, env):
                self.assertEqual(command[0], provenance["cargo_path"])
                self.assertEqual(command[1:], collector.CARGO_ARGS)
                self.assertEqual(env["RUSTC"], provenance["rustc_path"])
                self.assertEqual(Path(env["PATH"].split(os.pathsep, 1)[0]), Path(provenance["toolchain_bin"]))

            result = collector.collect_run(
                root,
                artifacts,
                runner=self.runner_for(self.document(), inspect=inspect_exact_compiler),
                metadata={"toolchain": {}, "hardware": {}},
                toolchain_resolver=lambda _root, _env: provenance,
            )
            evidence = json.loads(result.read_text())
            self.assertEqual(evidence["head_before"], evidence["head_after"])
            self.assertEqual(evidence["status_before"], "")
            self.assertEqual(evidence["source_sha256_before"], evidence["source_sha256_after"])
            self.assertEqual(evidence["command"][0], provenance["cargo_path"])
            self.assertEqual(evidence["compiler_provenance"], provenance)

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
                    toolchain_resolver=lambda actual_root, env: self.fake_toolchain_resolver(
                        Path(temporary) / "toolchain", actual_root, env
                    ),
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
                    toolchain_resolver=lambda actual_root, env: self.fake_toolchain_resolver(
                        Path(temporary) / "toolchain", actual_root, env
                    ),
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
                        toolchain_resolver=lambda actual_root, env: self.fake_toolchain_resolver(
                        Path(temporary) / "toolchain", actual_root, env
                    ),
                    )
                receipt = json.loads((artifacts / "time_warp_attempt.json").read_text())
                self.assertIn("validation_error", receipt)
                self.assertEqual(receipt["head_before"], receipt["head_after"])

    def test_injected_path_compiler_drift_is_resolved_then_rejected_if_rebound(self):
        with tempfile.TemporaryDirectory() as temporary:
            temporary = Path(temporary)
            root = temporary / "repo"
            root.mkdir()
            drift_dir = temporary / "homebrew"
            drift_dir.mkdir()
            drift_rustc = drift_dir / "rustc"
            drift_rustc.write_text("Homebrew rustc 1.99.0")
            drift_rustc.chmod(0o755)
            rustup_shim = drift_dir / "rustup"
            rustup_shim.write_text("injected rustup selector")
            rustup_shim.chmod(0o755)
            provenance = self.fake_toolchain_resolver(temporary / "rustup", root, os.environ)
            calls = []

            def metadata_runner(args, **_kwargs):
                calls.append(args)
                if args[1:4] == ["run", "1.98.1", "rustc"]:
                    output = provenance["sysroot"]
                elif args[1:5] == ["which", "rustc", "--toolchain", "1.98.1"]:
                    output = provenance["rustc_path"]
                elif args[1:5] == ["which", "cargo", "--toolchain", "1.98.1"]:
                    output = provenance["cargo_path"]
                elif args[0] == provenance["rustc_path"]:
                    output = provenance["rustc_version"]
                elif args[0] == provenance["cargo_path"] and args[1:] == ["--version"]:
                    output = provenance["cargo_version"]
                elif args[0] == provenance["cargo_path"] and args[1:] == ["-vV"]:
                    output = provenance["cargo_verbose_version"]
                else:
                    self.fail(f"unexpected toolchain metadata command: {args!r}")
                return SimpleNamespace(returncode=0, stdout=output, stderr="")

            base_env = dict(os.environ)
            base_env["PATH"] = str(drift_dir) + os.pathsep + base_env.get("PATH", "")
            base_env["RUSTC_WRAPPER"] = "/homebrew/bin/sccache"
            base_env["CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER"] = "/homebrew/bin/wrapper"
            base_env["RUSTFLAGS"] = ""
            base_env["RUSTC"] = "/homebrew/bin/rustc"
            base_env["CARGO_BUILD_RUSTC"] = "/homebrew/bin/rustc"
            base_env["RUSTUP_TOOLCHAIN"] = "1.99.0"
            resolved = collector.resolved_toolchain(root, base_env, metadata_runner)
            execution_env = collector.prepare_toolchain_environment(base_env, resolved)
            collector.verify_toolchain_environment(execution_env, resolved)
            self.assertEqual(Path(shutil.which("rustc", path=execution_env["PATH"])), Path(resolved["rustc_path"]))
            self.assertEqual(Path(execution_env["RUSTC"]), Path(resolved["rustc_path"]))
            self.assertEqual(Path(resolved["rustup_path"]), rustup_shim.resolve())
            self.assertEqual(
                resolved["disabled_wrapper_environment"],
                ["RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER"],
            )
            self.assertEqual(resolved["cleared_empty_rustflags_environment"], ["RUSTFLAGS"])
            self.assertEqual(
                resolved["disabled_toolchain_environment"],
                ["RUSTC", "CARGO_BUILD_RUSTC", "RUSTUP_TOOLCHAIN"],
            )
            self.assertNotIn("CARGO_BUILD_RUSTC", execution_env)
            self.assertEqual(execution_env["RUSTUP_TOOLCHAIN"], "1.98.1")
            self.assertNotIn("RUSTC_WRAPPER", execution_env)
            self.assertNotIn("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER", execution_env)
            self.assertNotIn("RUSTFLAGS", execution_env)
            self.assertTrue(any("1.98.1" in " ".join(call) for call in calls))

            drifted_env = dict(execution_env)
            drifted_env["RUSTC"] = str(drift_rustc)
            with self.assertRaisesRegex(ValueError, "RUSTC does not point"):
                collector.verify_toolchain_environment(drifted_env, resolved)

    def test_cargo_wrapper_and_rustflags_configuration_fail_closed(self):
        configurations = (
            '[build]\n"rustc-wrapper" = "ccache"\n',
            'build = { "rustc-workspace-wrapper" = "wrapper" }\n',
            'build.rustc-wrapper = "ccache"\n',
            '[build]\nrustflags = ["-C", "opt-level=2"]\n',
            '[target."aarch64-apple-darwin"]\nrustflags = ["-C", "target-cpu=native"]\n',
            '[env]\nRUSTC = { value = "/injected/rustc", force = true }\n',
            '[env]\nCARGO_BUILD_RUSTC = { value = "/injected/rustc", force = true }\n',
            '[env]\nRUSTUP_TOOLCHAIN = { value = "1.99.0", force = true }\n',
            '[env]\nCARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS = { value = "-C target-cpu=native", force = true }\n',
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = root / ".cargo" / "config.toml"
            config.parent.mkdir()
            env = {"CARGO_HOME": str(root / "empty-cargo-home")}
            for text in configurations:
                with self.subTest(config=text):
                    config.write_text(text)
                    with self.assertRaisesRegex(ValueError, "configured in"):
                        collector.cargo_wrapper_config_hashes(root, env)

    def test_nonempty_rustflags_environment_is_rejected_without_value_disclosure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            provenance = self.fake_toolchain_resolver(
                Path(temporary) / "toolchain", root, os.environ
            )
            env = dict(os.environ)
            env["CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS"] = "-C link-arg=private-marker"

            def unused_runner(*_args, **_kwargs):
                self.fail("nonempty Rust flags must reject before any toolchain command")

            with self.assertRaisesRegex(ValueError, "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS=sha256:") as error:
                collector.resolved_toolchain(root, env, unused_runner)
            self.assertNotIn("private-marker", str(error.exception))


if __name__ == "__main__":
    unittest.main()
