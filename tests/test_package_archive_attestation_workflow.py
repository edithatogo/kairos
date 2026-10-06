from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/package-dry-run.yml"
SOURCE_SHA = "a" * 40


def named_section(text: str, marker: str, next_marker: str | None = None) -> str:
    start = text.index(marker)
    if next_marker is None:
        return text[start:]
    end = text.index(next_marker, start + len(marker))
    return text[start:end]


def python_heredoc(section: str) -> str:
    match = re.search(r"python3 -B - <<'PY'\n(.*?)\n\s{10}PY(?:\n|$)", section, re.DOTALL)
    if match is None:
        raise AssertionError("expected a bounded Python heredoc in workflow step")
    return textwrap.dedent(match.group(1))


def fixture(workspace: Path) -> tuple[Path, dict[str, bytes]]:
    root = workspace / "dist/retained-package-archives"
    root.mkdir(parents=True)
    contents = {
        "rust/kairos.crate": b"rust archive",
        "python/kairos.whl": b"python wheel",
        "python/kairos.tar.gz": b"python sdist",
        "r/kairos.tar.gz": b"r archive",
        "julia/kairos.tar.gz": b"julia archive",
        "typescript/kairos.tgz": b"typescript archive",
        "nuget/kairos.nupkg": b"nuget archive",
        "go/kairos.tar.gz": b"go archive",
    }
    rows = []
    for relative, payload in sorted(contents.items()):
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)
        ecosystem, filename = relative.split("/", 1)
        kinds = {
            "rust": "crate",
            "python": "python-distribution",
            "r": "r-source-package",
            "julia": "julia-source-archive",
            "typescript": "npm-package",
            "nuget": "nuget-package",
            "go": "go-source-archive",
        }
        rows.append({
            "ecosystem": ecosystem,
            "kind": kinds[ecosystem],
            "path": relative,
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
            "builder": {},
        })
    index = {
        "schema_version": 1,
        "source_commit": SOURCE_SHA,
        "created_at_utc": "2026-10-06T00:00:00+00:00",
        "artifacts": rows,
    }
    index_bytes = (json.dumps(index, indent=2, sort_keys=True) + "\n").encode()
    (root / "ARCHIVE-INDEX.json").write_bytes(index_bytes)
    sums = "".join(f"{row['sha256']}  {row['path']}\n" for row in rows).encode()
    (root / "SHA256SUMS").write_bytes(sums)
    return root, contents


def rewrite_fixture(root: Path, mutate) -> None:
    index_path = root / "ARCHIVE-INDEX.json"
    index = json.loads(index_path.read_text(encoding="utf-8"))
    mutate(index)
    index_path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    sums = "".join(f"{row['sha256']}  {row['path']}\n" for row in index["artifacts"])
    (root / "SHA256SUMS").write_text(sums, encoding="utf-8")


class PackageArchiveAttestationWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
        cls.signer = named_section(cls.workflow, "  attest-retained-archives:\n")
        cls.projector_step = named_section(
            cls.signer,
            "      - name: Project exact archive checksums to workspace-relative subjects\n",
            "      # This step uploads signed metadata",
        )
        cls.projector = python_heredoc(cls.projector_step)

    def test_attestation_is_explicit_opt_in_on_manual_same_repo_main_only(self) -> None:
        self.assertIn("  pull_request:\n", self.workflow)
        self.assertIn("  workflow_dispatch:\n    inputs:\n      attest_archives:\n", self.workflow)
        input_block = named_section(self.workflow, "      attest_archives:\n", "\npermissions:")
        self.assertIn("required: true", input_block)
        self.assertIn("type: boolean", input_block)
        self.assertIn("default: false", input_block)
        signer_header = self.signer.split("    steps:\n", 1)[0]
        self.assertRegex(
            signer_header,
            r"(?m)^    if: inputs\.attest_archives == true && github\.event_name == 'workflow_dispatch' && github\.repository == 'edithatogo/kairos' && github\.ref == 'refs/heads/main'$",
        )
        for condition in (
            "github.event_name == 'workflow_dispatch'",
            "github.repository == 'edithatogo/kairos'",
            "github.ref == 'refs/heads/main'",
        ):
            self.assertIn(condition, self.signer)
        self.assertNotRegex(self.workflow, r"(?m)^  push:")
        self.assertIn('[[ "$ATTEST_ARCHIVES" == true ]]', self.signer)
        self.assertIn('[[ "$GITHUB_EVENT_NAME" == workflow_dispatch ]]', self.signer)
        self.assertIn('[[ "$GITHUB_REPOSITORY" == edithatogo/kairos ]]', self.signer)
        self.assertIn('[[ "$GITHUB_REF" == refs/heads/main ]]', self.signer)
        self.assertIn('[[ "$GITHUB_SHA" == "$GITHUB_WORKFLOW_SHA" ]]', self.signer)

    def test_signer_depends_on_exact_retained_archive_artifact_and_verifies_source(self) -> None:
        self.assertIn("needs: [retain-package-archives]", self.signer)
        self.assertIn("name: kairos-actual-package-archives-${{ github.sha }}", self.signer)
        self.assertIn("ref: ${{ github.workflow_sha }}", self.signer)
        self.assertIn("persist-credentials: false", self.signer)
        verify_step = named_section(
            self.signer,
            "      - name: Reverify exact source-bound archive tree\n",
            "      - name: Project exact archive checksums",
        )
        self.assertIn("--verify-existing", verify_step)
        self.assertIn("--source-commit \"$GITHUB_SHA\"", verify_step)
        self.assertLess(self.signer.index("Reverify exact source-bound archive tree"),
                        self.signer.index("Project exact archive checksums"))
        self.assertLess(self.signer.index("Project exact archive checksums"),
                        self.signer.index("uses: actions/attest@"))

    def test_permissions_and_action_pins_are_least_privilege(self) -> None:
        permission_block = re.search(
            r"(?m)^    permissions:\n((?:      [^\n]+\n)+)", self.signer
        )
        self.assertIsNotNone(permission_block)
        permissions = [line.strip().split(":", 1) for line in permission_block.group(1).splitlines()]
        self.assertEqual([(key, value.split(" #", 1)[0].strip()) for key, value in permissions], [
            ("actions", "read"),
            ("attestations", "write"),
            ("contents", "read"),
            ("id-token", "write"),
        ])
        self.assertNotRegex(self.workflow, r"(?m)^\s+(artifact-metadata|packages|contents|actions):\s*write\b")
        build_jobs = self.workflow.split("  attest-retained-archives:\n", 1)[0]
        self.assertNotIn("id-token: write", build_jobs)
        self.assertNotIn("attestations: write", build_jobs)
        self.assertEqual(self.workflow.count("id-token: write"), 1)
        self.assertEqual(self.workflow.count("attestations: write"), 1)
        self.assertNotIn("artifact-metadata: write", self.signer)
        self.assertIn("actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6 # v4.2.2", self.signer)
        self.assertIn("actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1", self.signer)
        self.assertIn("actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1", self.signer)

    def test_attestation_subjects_and_retained_bundle_are_explicit_and_nonpublishing(self) -> None:
        self.assertIn("subject-checksums: dist/package-provenance-evidence/package-archive-subjects.sha256", self.signer)
        self.assertIn("push-to-registry: false", self.signer)
        self.assertIn("create-storage-record: false", self.signer)
        self.assertIn("outputs.bundle-path", self.signer)
        self.assertIn("attestation.bundle.json", self.signer)
        self.assertIn("ARCHIVE-INDEX.json", self.signer)
        self.assertIn("SHA256SUMS", self.signer)
        self.assertIn("package-archive-subjects.sha256", self.signer)
        self.assertIn("retention-days: 90", self.signer)
        self.assertIn("if-no-files-found: error", self.signer)
        self.assertIn("uploads signed metadata to the GitHub Attestations API", self.signer)
        self.assertIn("does not publish packages", self.signer)
        self.assertNotRegex(self.signer, r"(?i)SLSA\s*(?:level\s*)?3|per-job compiler provenance")

    def run_projector(self, workspace: Path, source_sha: str = SOURCE_SHA) -> subprocess.CompletedProcess[str]:
        env = {
            **os.environ,
            "GITHUB_SHA": source_sha,
            "GITHUB_WORKSPACE": str(workspace),
        }
        return subprocess.run(
            [sys.executable, "-B", "-c", self.projector],
            cwd=workspace,
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_projector_emits_exact_eight_workspace_relative_subjects_without_changing_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            root, contents = fixture(workspace)
            original_index = (root / "ARCHIVE-INDEX.json").read_bytes()
            original_sums = (root / "SHA256SUMS").read_bytes()
            result = self.run_projector(workspace)
            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = workspace / "dist/package-provenance-evidence"
            self.assertEqual(
                sorted(path.name for path in evidence.iterdir()),
                ["ARCHIVE-INDEX.json", "SHA256SUMS", "package-archive-subjects.sha256"],
            )
            rows = json.loads(original_index)["artifacts"]
            expected = "".join(
                f"{row['sha256']}  dist/retained-package-archives/{row['path']}\n"
                for row in rows
            )
            projected = (evidence / "package-archive-subjects.sha256").read_text(encoding="ascii")
            self.assertEqual(projected, expected)
            self.assertEqual(len(projected.splitlines()), 8)
            self.assertTrue(projected.endswith("\n"))
            self.assertEqual((evidence / "ARCHIVE-INDEX.json").read_bytes(), original_index)
            self.assertEqual((evidence / "SHA256SUMS").read_bytes(), original_sums)
            self.assertEqual((root / "ARCHIVE-INDEX.json").read_bytes(), original_index)
            self.assertEqual((root / "SHA256SUMS").read_bytes(), original_sums)
            for relative, payload in contents.items():
                self.assertEqual((root / relative).read_bytes(), payload)

    def test_projector_rejects_wrong_source_missing_archive_extra_row_tampered_sums(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            root, _ = fixture(workspace)
            result = self.run_projector(workspace, source_sha="b" * 40)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((workspace / "dist/package-provenance-evidence").exists())

        for mutation in ("missing", "extra", "sums", "changed-bytes", "changed-digest", "duplicate-path"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                workspace = Path(temporary)
                root, _ = fixture(workspace)
                if mutation == "missing":
                    (root / "rust/kairos.crate").unlink()
                elif mutation == "extra":
                    def add_extra(index):
                        index["artifacts"].append(dict(index["artifacts"][0], path="rust/extra.crate"))
                    rewrite_fixture(root, add_extra)
                elif mutation == "sums":
                    (root / "SHA256SUMS").write_text("0" * 64 + "  rust/kairos.crate\n", encoding="ascii")
                elif mutation == "changed-bytes":
                    (root / "rust/kairos.crate").write_bytes(b"changed bytes")
                elif mutation == "changed-digest":
                    rewrite_fixture(root, lambda index: next(row for row in index["artifacts"] if row["path"] == "rust/kairos.crate").update(sha256="0" * 64))
                else:
                    rewrite_fixture(root, lambda index: next(row for row in index["artifacts"] if row["path"].endswith(".whl")).update(path="python/kairos.tar.gz"))
                result = self.run_projector(workspace)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((workspace / "dist/package-provenance-evidence").exists())

    def test_projector_rejects_newline_path_python_pair_mismatch_and_symlinked_parent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            root, _ = fixture(workspace)
            rewrite_fixture(root, lambda index: index["artifacts"][0].update(path="go/archive.tar.gz\n0" * 1))
            result = self.run_projector(workspace)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((workspace / "dist/package-provenance-evidence").exists())

        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            root, _ = fixture(workspace)
            def replace_python_pair(index):
                wheel = next(row for row in index["artifacts"] if row["path"].endswith(".whl"))
                wheel_path = root / "python/second.tar.gz"
                wheel_path.write_bytes(b"second sdist")
                wheel["path"] = "python/second.tar.gz"
                wheel["bytes"] = wheel_path.stat().st_size
                wheel["sha256"] = hashlib.sha256(wheel_path.read_bytes()).hexdigest()
            rewrite_fixture(root, replace_python_pair)
            result = self.run_projector(workspace)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((workspace / "dist/package-provenance-evidence").exists())

        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            root, _ = fixture(workspace)
            python_root = root / "python"
            external = workspace / "external-python"
            python_root.rename(external)
            python_root.symlink_to(external, target_is_directory=True)
            result = self.run_projector(workspace)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((workspace / "dist/package-provenance-evidence").exists())


if __name__ == "__main__":
    unittest.main()
