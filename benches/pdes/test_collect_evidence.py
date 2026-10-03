"""Fail-closed checks for Track 47 evidence collection."""

import importlib.util
from pathlib import Path
import json
import tempfile
import unittest
from unittest import mock


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
    def write_linux_fixture(
        self,
        root: Path,
        *,
        cpuinfo: str,
        cpu_topology: dict[int, dict[str, str]],
        lscpu_output: str = "unavailable (fixture)",
        lscpu_parse: str = "unavailable (fixture)",
    ) -> dict[str, Path]:
        proc_root = root / "proc"
        sys_root = root / "sys"
        cpu_root = sys_root / "devices/system/cpu"
        proc_root.mkdir(parents=True)
        (proc_root / "cpuinfo").write_text(cpuinfo, encoding="utf-8")
        (proc_root / "meminfo").write_text("MemTotal:       8192000 kB\n", encoding="utf-8")
        cpu_root.mkdir(parents=True)
        if cpu_topology:
            online = sorted(cpu_topology)
            (cpu_root / "online").write_text(
                f"{online[0]}-{online[-1]}\n", encoding="utf-8"
            )
        for cpu, fields in cpu_topology.items():
            topology = cpu_root / f"cpu{cpu}/topology"
            topology.mkdir(parents=True)
            for name, value in fields.items():
                (topology / name).write_text(value + "\n", encoding="utf-8")
        node_root = sys_root / "devices/system/node"
        node_root.mkdir(parents=True)
        return {
            "proc": proc_root,
            "sys": sys_root,
            "lscpu": lscpu_output,
            "lscpu_parse": lscpu_parse,
        }

    def linux_metadata(self, fixture: dict[str, Path], logical: int) -> dict:
        def command_output(
            command: list[str], *, timeout: int = 20, env: dict[str, str] | None = None
        ) -> str:
            del timeout
            del env
            if command == ["lscpu"]:
                return str(fixture["lscpu"])
            if command == ["lscpu", "--parse=CPU,SOCKET,CORE"]:
                return str(fixture["lscpu_parse"])
            return "unavailable (unexpected fixture command)"

        with (
            mock.patch.object(collector.platform, "system", return_value="Linux"),
            mock.patch.object(collector.os, "cpu_count", return_value=logical),
            mock.patch.object(collector, "LINUX_PROC_ROOT", fixture["proc"]),
            mock.patch.object(collector, "LINUX_SYS_ROOT", fixture["sys"]),
            mock.patch.object(collector, "command_output", side_effect=command_output),
        ):
            return collector.hardware_metadata()

    def test_linux_arm_hardware_metadata_uses_kernel_sibling_groups(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self.write_linux_fixture(
                Path(directory),
                cpuinfo=(
                    "processor\t: 0\nHardware\t: ARMv8 virtual CPU\n\n"
                    "processor\t: 1\nHardware\t: ARMv8 virtual CPU\n\n"
                    "processor\t: 2\nHardware\t: ARMv8 virtual CPU\n\n"
                    "processor\t: 3\nHardware\t: ARMv8 virtual CPU\n"
                ),
                cpu_topology={
                    cpu: {"thread_siblings_list": group}
                    for cpu, group in ((0, "0-1"), (1, "0-1"), (2, "2-3"), (3, "2-3"))
                },
            )
            metadata = self.linux_metadata(fixture, logical=4)

        self.assertEqual(metadata["physical_cpu_count"], 2)
        self.assertIn("2 kernel-reported core groups (sysfs)", metadata["cpu_topology"])
        self.assertEqual(metadata["logical_cpu_count"], 4)
        self.assertEqual(metadata["cpu_model"], "ARMv8 virtual CPU")
        self.assertEqual(metadata["memory_bytes"], 8192000 * 1024)

    def test_linux_virtualized_unknown_topology_remains_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self.write_linux_fixture(
                Path(directory),
                cpuinfo="processor\t: 0\n",
                cpu_topology={0: {"physical_package_id": "-1", "core_id": "-1"}},
            )
            metadata = self.linux_metadata(fixture, logical=1)

        self.assertEqual(metadata["physical_cpu_count"], 0)
        self.assertIn("unknown (kernel core topology unavailable)", metadata["cpu_topology"])
        self.assertEqual(metadata["logical_cpu_count"], 1)
        self.assertTrue(metadata["cpu_model"].startswith("unavailable"))

    def test_linux_arm_kernel_identifiers_are_reported_without_inventing_a_model(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self.write_linux_fixture(
                Path(directory),
                cpuinfo=(
                    "processor\t: 0\nCPU implementer\t: 0x41\n"
                    "CPU part\t: 0xd03\n"
                ),
                cpu_topology={0: {"thread_siblings_list": "0"}},
            )
            metadata = self.linux_metadata(fixture, logical=1)

        self.assertEqual(
            metadata["cpu_model"],
            "ARM CPU identifiers reported by kernel: implementer 0x41, part 0xd03",
        )

    def test_linux_arm_lscpu_fallback_counts_complete_socket_core_rows(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self.write_linux_fixture(
                Path(directory),
                cpuinfo="processor : 0\nHardware : ARMv8 host\n",
                cpu_topology={},
                lscpu_output="Architecture: aarch64\nModel name: ARMv8 host model\n",
                lscpu_parse="# CPU,SOCKET,CORE\n0,0,0\n1,0,0\n2,0,1\n3,0,1\n",
            )
            metadata = self.linux_metadata(fixture, logical=4)

        self.assertEqual(metadata["physical_cpu_count"], 2)
        self.assertIn("2 kernel-reported core groups (lscpu)", metadata["cpu_topology"])
        self.assertEqual(metadata["cpu_model"], "ARMv8 host")

    def test_linux_procfs_fallback_normalizes_tabbed_x86_topology_fields(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cpu_blocks = [
                (0, 0, 0),
                (1, 0, 0),
                (2, 0, 1),
                (3, 0, 1),
            ]
            cpuinfo = "\n\n".join(
                f"processor\t: {processor}\nphysical id\t: {package}\n"
                f"core id\t: {core}\nmodel name\t: x86 fixture CPU"
                for processor, package, core in cpu_blocks
            )
            fixture = self.write_linux_fixture(
                Path(directory), cpuinfo=cpuinfo, cpu_topology={}
            )
            metadata = self.linux_metadata(fixture, logical=4)

        self.assertEqual(metadata["physical_cpu_count"], 2)
        self.assertIn("2 kernel-reported core groups (procfs)", metadata["cpu_topology"])
        self.assertEqual(metadata["cpu_model"], "x86 fixture CPU")

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
