from __future__ import annotations

import hashlib
import gzip
import importlib.util
import io
import json
import os
from pathlib import Path
import struct
import sys
import tarfile
import tempfile
import time
import unittest
import zipfile
from unittest import mock

SCRIPT = Path(__file__).parents[1] / "packaging/scripts/verify_archive_supply_chain_evidence.py"
SPEC = importlib.util.spec_from_file_location("archive_evidence_verifier_test", SCRIPT)
assert SPEC and SPEC.loader
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)
PROVENANCE = importlib.util.spec_from_file_location("archive_fixture_provenance", SCRIPT.with_name("validate_archive_copy_provenance.py"))
assert PROVENANCE and PROVENANCE.loader
provenance = importlib.util.module_from_spec(PROVENANCE)
sys.modules[PROVENANCE.name] = provenance
PROVENANCE.loader.exec_module(provenance)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _json_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _put(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def _make_zip(members: dict[str, bytes]) -> bytes:
    result = io.BytesIO()
    with zipfile.ZipFile(result, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, payload in members.items():
            archive.writestr(name, payload)
    return result.getvalue()


def _make_tar(member: str, payload: bytes) -> bytes:
    result = io.BytesIO()
    with tarfile.open(fileobj=result, mode="w:gz") as archive:
        info = tarfile.TarInfo(member)
        info.size = len(payload)
        archive.addfile(info, io.BytesIO(payload))
    return result.getvalue()


class CompleteEvidenceFixture:
    """Build a tiny complete profile with one real-format archive per ecosystem."""

    def __init__(self, root: Path):
        self.root = root
        self.bundle = root / "acquisition" / "bundle"
        self.evidence = root / "result"
        self.acquisition = root / "acquisition"
        self.expected = root / "expected"
        self.commit, self.pr_head, self.tree = "a" * 40, "b" * 40, "c" * 40
        self.run_id, self.artifact_id = 37273717088, 11329331832
        self.specs = [
            ("go", "go-source-archive", "go/fixture.tar.gz", "go.mod", b"module example.org/fixture-go\n", "example.org/fixture-go", None, "go"),
            ("julia", "julia-source-archive", "julia/fixture.tar.gz", "Project.toml", b'name = "fixture-julia"\nversion = "1.0"\n', "fixture-julia", "1.0", "julia"),
            ("nuget", "nuget-package", "nuget/fixture.nupkg", "Fixture.nuspec", b'<package><metadata><id>fixture-nuget</id><version>1.0</version></metadata></package>', "fixture-nuget", "1.0", "zip"),
            ("python", "python-distribution", "python/fixture.whl", "fixture_python-1.dist-info/METADATA", b"Metadata-Version: 2.1\nName: fixture-python\nVersion: 1.0\n\n", "fixture-python", "1.0", "zip"),
            ("r", "r-source-package", "r/fixture.tar.gz", "fixture/DESCRIPTION", b"Package: fixtureR\nVersion: 1.0\n", "fixtureR", "1.0", "tar"),
            ("rust", "crate", "rust/fixture.crate", "fixture/Cargo.toml", b'[package]\nname = "fixture-rust"\nversion = "1.0"\n', "fixture-rust", "1.0", "tar"),
            ("typescript", "npm-package", "typescript/fixture.tgz", "package/package.json", b'{"name":"fixture-ts","version":"1.0"}', "fixture-ts", "1.0", "tar"),
        ]
        self.rows: list[dict[str, object]] = []
        self.archive_contents: dict[str, bytes] = {}
        self.identities: dict[str, dict[str, object]] = {}
        self._build()

    def _build(self) -> None:
        self.bundle.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.acquisition.mkdir(parents=True, exist_ok=True)
        self.expected.mkdir(parents=True)
        ecosystem_receipts = {
            ecosystem: {"ecosystem": ecosystem, "source_commit": self.commit, "exit_status": 0,
                        "command": "synthetic archive fixture", "toolchain": "fixture", "platform": "fixture"}
            for ecosystem in verifier.ECOSYSTEMS
        }
        for ecosystem, kind, path, metadata_path, metadata, name, version, archive_type in self.specs:
            payload = _make_zip({metadata_path: metadata}) if archive_type == "zip" else _make_tar(metadata_path, metadata)
            self.archive_contents[path] = payload
            row = {"ecosystem": ecosystem, "kind": kind, "path": path, "bytes": len(payload),
                   "sha256": _sha(payload), "builder": ecosystem_receipts[ecosystem]}
            self.rows.append(row)
            _put(self.bundle / path, payload)
            self.identities[path] = {"name": name, "version": version,
                                     "metadata_path": metadata_path, "metadata_sha256": _sha(metadata),
                                     "repository_commit": None}
        self.rows.sort(key=lambda row: row["path"])
        index = {"schema_version": 1, "source_commit": self.commit, "created_at_utc": "2026-10-05T00:00:00Z", "artifacts": self.rows}
        self.index_bytes = _json_bytes(index)
        self.receipt_bytes = _json_bytes({"source_commit": self.commit, "ecosystems": ecosystem_receipts})
        _put(self.bundle / "ARCHIVE-INDEX.json", self.index_bytes)
        _put(self.bundle / "BUILD-RECEIPT.json", self.receipt_bytes)
        bundle_sums = "".join(f"{row['sha256']}  {row['path']}\n" for row in self.rows).encode()
        _put(self.bundle / "SHA256SUMS", bundle_sums)
        zip_members = {"ARCHIVE-INDEX.json": self.index_bytes, "BUILD-RECEIPT.json": self.receipt_bytes, "SHA256SUMS": bundle_sums}
        zip_members.update(self.archive_contents)
        self.archive_zip_bytes = _make_zip(zip_members)
        self.archive_zip = self.acquisition / "artifact.zip"
        _put(self.archive_zip, self.archive_zip_bytes)
        self.binding = {
            "schema_version": 1, "repository": "edithatogo/kairos", "source_commit": self.commit,
            "producer_pr_head": self.pr_head, "producer_tree": self.tree, "original_run_id": self.run_id,
            "acquisition_artifact_id": self.artifact_id, "archive_zip_sha256": _sha(self.archive_zip_bytes),
            "archive_zip_bytes": len(self.archive_zip_bytes), "spdx_schema_sha256": "",
        }
        schema = SCRIPT.parents[2] / "tests/fixtures/archive-supply-chain/spdx-2.3/spdx-schema.json"
        self.schema = self.expected / "spdx-2.3-schema.json"
        schema_bytes = schema.read_bytes()
        self.binding["spdx_schema_sha256"] = _sha(schema_bytes)
        _put(self.schema, schema_bytes)
        _put(self.acquisition / "artifact-metadata.json", _json_bytes({
            "id": self.artifact_id, "size_in_bytes": len(self.archive_zip_bytes),
            "digest": "sha256:" + _sha(self.archive_zip_bytes),
            "workflow_run": {"id": self.run_id, "head_sha": self.pr_head},
        }))
        _put(self.acquisition / "source-commit-readback.json", _json_bytes({self.commit: {
            "sha": self.commit, "tree": {"sha": self.tree}, "parents": [{"sha": self.pr_head,
                "url": f"https://api.github.com/repos/edithatogo/kairos/git/commits/{self.pr_head}",
                "html_url": f"https://github.com/edithatogo/kairos/commit/{self.pr_head}"}],
            "verification": {"verified": True, "reason": "valid"},
        }}))
        artifact_hash = _sha((self.acquisition / "artifact-metadata.json").read_bytes())
        source_hash = _sha((self.acquisition / "source-commit-readback.json").read_bytes())
        zip_hash = _sha(self.archive_zip_bytes)
        acquisition_receipt = {
            "exit_status": 0, "artifact_id": self.artifact_id, "workflow_run": self.run_id,
            "producer_checkout_source": self.commit, "producer_pr_head": self.pr_head, "producer_tree": self.tree,
            "archive_zip_sha256": zip_hash, "actual_archive_count": len(self.rows),
            "ecosystems": sorted(verifier.ECOSYSTEMS),
            "scope": "Actual retained archive structural/source acquisition verification; no producer SLSA attestation, release or registry acceptance",
        }
        _put(self.acquisition / "receipt.json", _json_bytes(acquisition_receipt))
        acquisition_data = {
            "archive_count": len(self.rows), "archive_index_sha256": _sha(self.index_bytes),
            "artifact_digest": "sha256:" + zip_hash, "artifact_id": self.artifact_id,
            "derivation": {"status": "derived local adapter receipt; not original acquisition history", "inputs": {
                "archive_index_sha256": _sha(self.index_bytes),
                "artifact_metadata_sha256": artifact_hash,
                "original_local_verification_receipt_sha256": _sha((self.acquisition / "receipt.json").read_bytes()),
                "source_commit_readback_sha256": source_hash,
            }},
            "ecosystems": sorted(verifier.ECOSYSTEMS), "repository": "edithatogo/kairos",
            "run_id": self.run_id, "source_commit": self.commit,
        }
        self.acquisition_data_bytes = _json_bytes(acquisition_data)
        for rel, data in (("build-inputs/ARCHIVE-INDEX.json", self.index_bytes),
                          ("build-inputs/BUILD-RECEIPT.json", self.receipt_bytes),
                          ("build-inputs/acquisition.json", self.acquisition_data_bytes)):
            _put(self.evidence / rel, data)
        index_sha = _sha(self.index_bytes)
        self.manifest_rows = []
        for row in self.rows:
            path = row["path"]
            self.manifest_rows.append({"path": "archives/" + path, "sha256": row["sha256"], "bytes": row["bytes"],
                "ecosystem": "csharp" if row["ecosystem"] == "nuget" else row["ecosystem"],
                "archive_ecosystem": row["ecosystem"], "kind": row["kind"]})
            _put(self.evidence / ("archives/" + path), self.archive_contents[path])
        self.manifest_rows.sort(key=lambda row: row["path"])
        _put(self.evidence / "release-artifact-manifest.json", _json_bytes({
            "schema_version": 1, "release_stage": "actual-package-archives", "source_commit": self.commit,
            "production_publish_enabled": False, "archive_index_sha256": index_sha, "artifacts": self.manifest_rows,
        }))
        helper_hashes = {"packaging/scripts/" + helper: _sha((SCRIPT.parent / helper).read_bytes()) for helper in verifier.HELPERS}
        syft_hash = "d" * 64
        dependencies = [{"id": "https://github.com/edithatogo/kairos/actions/runs/" + str(self.run_id), "sha256": zip_hash}]
        dependencies.extend([
            {"id": "ARCHIVE-INDEX.json", "sha256": index_sha},
            {"id": "build-inputs/ARCHIVE-INDEX.json", "sha256": index_sha},
            {"id": "build-inputs/BUILD-RECEIPT.json", "sha256": _sha(self.receipt_bytes)},
            {"id": "build-inputs/acquisition.json", "sha256": _sha(self.acquisition_data_bytes)},
        ])
        dependencies.extend({"id": key, "sha256": value} for key, value in helper_hashes.items())
        dependencies.extend([{"id": "tool:syft", "sha256": syft_hash}, {"id": "schema:spdx-2.3", "sha256": self.binding["spdx_schema_sha256"]}])
        dependencies.sort(key=lambda item: item["id"])
        self.expected_inputs = {"archive_index_sha256": index_sha, "source_commit": self.commit,
                                "original_run_id": self.run_id, "acquisition_artifact_id": self.artifact_id,
                                "dependencies": dependencies}
        self.expected_inputs_bytes = _json_bytes(self.expected_inputs)
        # Independent caller file intentionally has different formatting from the
        # retained producer copy; equality is semantic while the receipt binds bytes.
        _put(self.expected / "expected-inputs.json", (json.dumps(self.expected_inputs, indent=2) + "\n").encode())
        _put(self.expected / "outer-binding.json", _json_bytes(self.binding))
        _put(self.evidence / "expected-inputs.json", self.expected_inputs_bytes)
        self._build_spdx()
        self._build_provenance()
        self._build_checksums()

    def _build_spdx(self) -> None:
        namespaces = {}
        coverage_rows = []
        external_refs = []
        relationships = []
        root_packages = []
        for row in self.rows:
            path = row["path"]
            identifier = hashlib.sha256(path.encode()).hexdigest()
            document_ref = "DocumentRef-" + identifier
            component_package_ids = []
            component_packages = []
            if row["ecosystem"] not in ("go", "julia"):
                identity = self.identities[path]
                component_packages = [{"SPDXID": "SPDXRef-Package", "name": identity["name"],
                    "versionInfo": identity["version"], "downloadLocation": "NOASSERTION",
                    "filesAnalyzed": False, "licenseConcluded": "NOASSERTION",
                    "licenseDeclared": "NOASSERTION", "copyrightText": "NOASSERTION"}]
                component_package_ids = ["SPDXRef-Package"]
            component = {"SPDXID": "SPDXRef-DOCUMENT", "spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0",
                "name": "fixture " + path, "documentNamespace": "https://example.invalid/spdx/" + identifier,
                "creationInfo": {"created": "2026-10-05T00:00:00Z", "creators": ["Tool: fixture"]},
                "packages": component_packages, "relationships": []}
            component_bytes = _json_bytes(component)
            component_rel = f"component-sboms/{identifier}.spdx.json"
            _put(self.evidence / component_rel, component_bytes)
            _put(self.evidence / f"component-sboms/{identifier}.stdout", b"")
            _put(self.evidence / f"component-sboms/{identifier}.stderr", b"")
            namespaces[path] = component["documentNamespace"]
            identity = self.identities[path]
            coverage_rows.append({"archive": path, "archive_sha256": row["sha256"], "identity": identity,
                "scanner_software_packages": len(component_packages),
                "scanner_identities": [{"name": p["name"], "version": p.get("versionInfo")} for p in component_packages],
                "manifest_identity_fallback": row["ecosystem"] in ("go", "julia") and not component_packages,
                "component_sbom": component_rel, "component_sbom_sha256": _sha(component_bytes)})
            root_package_id = "SPDXRef-archive-" + identifier
            pkg = {"SPDXID": root_package_id, "name": identity["name"], "packageFileName": "archives/" + path,
                "filesAnalyzed": False, "downloadLocation": "NOASSERTION", "licenseConcluded": "NOASSERTION",
                "licenseDeclared": "NOASSERTION", "copyrightText": "NOASSERTION",
                "checksums": [{"algorithm": "SHA256", "checksumValue": row["sha256"]}],
                "sourceInfo": f"Identity from packaged {identity['metadata_path']} SHA256 {identity['metadata_sha256']}"}
            if identity["version"] is not None:
                pkg["versionInfo"] = identity["version"]
            root_packages.append(pkg)
            external_refs.append({"externalDocumentId": document_ref, "spdxDocument": component["documentNamespace"],
                                  "checksum": {"algorithm": "SHA256", "checksumValue": _sha(component_bytes)}})
            relationships.append({"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": root_package_id})
            for package_id in component_package_ids:
                relationships.append({"spdxElementId": root_package_id, "relationshipType": "CONTAINS",
                                      "relatedSpdxElement": document_ref + ":" + package_id})
        coverage = {"coverage": coverage_rows, "source_identities": {},
                    "runtime": {"python": "fixture", "jsonschema": "fixture"},
                    "syft_sha256": "d" * 64, "schema_sha256": self.binding["spdx_schema_sha256"],
                    "provenance_scope": "Unsigned local copying/evidence build; does not claim original compilation attestation or SLSA level."}
        coverage["source_identities"] = {helper: next(d["sha256"] for d in self.expected_inputs["dependencies"] if d["id"] == "packaging/scripts/" + helper) for helper in verifier.HELPERS}
        _put(self.evidence / "SBOM-COVERAGE.json", _json_bytes(coverage))
        root = {"SPDXID": "SPDXRef-DOCUMENT", "spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0",
            "name": "fixture archive set", "documentNamespace": "https://example.invalid/spdx/root",
            "creationInfo": {"created": "2026-10-05T00:00:00Z", "creators": ["Tool: fixture"]},
            "packages": root_packages, "externalDocumentRefs": external_refs, "relationships": relationships}
        _put(self.evidence / "sbom.spdx.json", _json_bytes(root))

    def _build_provenance(self) -> None:
        deps = self.expected_inputs["dependencies"]
        run = self.run_id
        subjects = [{"name": "archives/" + row["path"], "digest": {"sha256": row["sha256"]}} for row in self.rows]
        resolved = [{"uri": provenance.canonical_dependency_uri(d["id"], run),
                     "digest": {"sha256": d["sha256"]}, "name": d["id"]} for d in deps]
        statement = {"_type": provenance.STATEMENT_TYPE, "predicateType": provenance.PREDICATE_TYPE,
            "subject": subjects, "predicate": {"buildDefinition": {"buildType": provenance.BUILD_TYPE,
                "externalParameters": {"source_commit": self.commit, "original_run_id": run},
                "internalParameters": {"acquisition_artifact_id": self.artifact_id}, "resolvedDependencies": resolved},
                "runDetails": {"builder": {"id": provenance.BUILDER_ID}, "metadata": {
                    "startedOn": "2026-10-05T00:00:00Z", "finishedOn": "2026-10-05T00:00:01Z"}}}}
        statement_bytes = _json_bytes(statement)
        _put(self.evidence / "provenance.json", statement_bytes)
        result = {"valid": True, "statement_sha256": _sha(statement_bytes),
            "archive_index_sha256": self.expected_inputs["archive_index_sha256"],
            "expected_inputs_sha256": _sha(self.expected_inputs_bytes),
            "validator_sha256": next(d["sha256"] for d in deps if d["id"] == "packaging/scripts/validate_archive_copy_provenance.py"),
            "issue_count": 0, "issues": [],
            "claim_scope": "local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance"}
        _put(self.evidence / "validation-result.json", _json_bytes(result))
        _put(self.evidence / "RELEASE.txt", f"Verified package archives from {self.commit}. Evidence preparation only; publication disabled.\n".encode())

    def _build_checksums(self) -> None:
        sums = "".join(f"{row['sha256']}  {row['path']}\n" for row in self.manifest_rows).encode()
        _put(self.evidence / "SHA256SUMS", sums)
        inventory, _ = verifier.scan_inventory(self.evidence)
        lines = []
        for path in sorted(inventory):
            if path == "SUPPLY-CHAIN-SHA256SUMS":
                continue
            lines.append(f"{_sha((self.evidence / path).read_bytes())}  {path}\n")
        _put(self.evidence / "SUPPLY-CHAIN-SHA256SUMS", "".join(lines).encode())

    def rebuild_supply_checksums(self) -> None:
        inventory, _ = verifier.scan_inventory(self.evidence)
        lines = [f"{_sha((self.evidence / path).read_bytes())}  {path}\n"
                 for path in sorted(inventory) if path != "SUPPLY-CHAIN-SHA256SUMS"]
        _put(self.evidence / "SUPPLY-CHAIN-SHA256SUMS", "".join(lines).encode())

    def rewrite_validation_receipt(self) -> None:
        statement = (self.evidence / "provenance.json").read_bytes()
        receipt_path = self.evidence / "validation-result.json"
        receipt = json.loads(receipt_path.read_bytes())
        receipt["statement_sha256"] = _sha(statement)
        _put(receipt_path, _json_bytes(receipt))

    def write_inputs(self, value: dict[str, object]) -> None:
        self.expected_inputs = value
        self.expected_inputs_bytes = _json_bytes(value)
        _put(self.expected / "expected-inputs.json", json.dumps(value, indent=2).encode() + b"\n")
        _put(self.evidence / "expected-inputs.json", self.expected_inputs_bytes)

    def args(self):
        return type("Args", (), {"evidence_dir": self.evidence, "archive_bundle": self.bundle,
            "archive_zip": self.archive_zip, "acquisition_dir": self.acquisition,
            "expected_inputs": self.expected / "expected-inputs.json",
            "expected_binding": self.expected / "outer-binding.json", "spdx_schema": self.schema,
            "expected_verifier_sha256": _sha(SCRIPT.read_bytes())})()


class ArchiveEvidenceVerifierTests(unittest.TestCase):
    def test_complete_seven_ecosystem_profile_passes_end_to_end(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            events = []
            real_preflight, real_zip = verifier.preflight_zip, verifier.zipfile.ZipFile
            def preflight(stream, label):
                events.append(("preflight", label))
                return real_preflight(stream, label)
            def zipfile(*args, **kwargs):
                events.append(("zipfile", ""))
                return real_zip(*args, **kwargs)
            with mock.patch.object(verifier, "preflight_zip", side_effect=preflight), mock.patch.object(verifier.zipfile, "ZipFile", side_effect=zipfile):
                result = verifier.verify_profile(fixture.args())
            self.assertTrue(result["valid"])
            self.assertEqual(result["archive_count"], 7)
            self.assertEqual(result["ecosystem_count"], 7)
            self.assertEqual(result["spdx_document_count"], 8)
            self.assertEqual(len(events) % 2, 0)
            self.assertTrue(all(events[pos][0] == "preflight" and events[pos + 1][0] == "zipfile"
                                for pos in range(0, len(events), 2)))

    def test_external_expected_inputs_must_match_bundled_map_semantically(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            external = json.loads(fixture.expected_inputs_bytes)
            external["original_run_id"] += 1
            _put(fixture.expected / "expected-inputs.json", _json_bytes(external))
            with self.assertRaises(verifier.VerificationFailure) as raised:
                verifier.verify_profile(fixture.args())
            self.assertEqual(raised.exception.code, "run_id_mismatch")

    def test_outer_source_binding_mutation_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            binding = dict(fixture.binding)
            binding["source_commit"] = "f" * 40
            _put(fixture.expected / "outer-binding.json", _json_bytes(binding))
            with self.assertRaises(verifier.VerificationFailure) as raised:
                verifier.verify_profile(fixture.args())
            self.assertEqual(raised.exception.code, "source_commit_mismatch")

    def test_native_parent_objects_require_expected_parent_and_exact_urls(self) -> None:
        for parents in ([], [{"sha": "d" * 40, "url": f"https://api.github.com/repos/edithatogo/kairos/git/commits/{'d' * 40}",
                              "html_url": f"https://github.com/edithatogo/kairos/commit/{'d' * 40}"}],
                        ["b" * 40]):
            with self.subTest(parents=parents), tempfile.TemporaryDirectory() as temporary:
                fixture = CompleteEvidenceFixture(Path(temporary).resolve())
                path = fixture.acquisition / "source-commit-readback.json"
                document = json.loads(path.read_bytes())
                document[fixture.commit]["parents"] = parents
                _put(path, _json_bytes(document))
                with self.assertRaises(verifier.VerificationFailure) as raised:
                    verifier.verify_profile(fixture.args())
                self.assertEqual(raised.exception.code, "source_readback_parent")

    def test_helper_self_and_schema_pins_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            args = fixture.args()
            args.expected_verifier_sha256 = "0" * 64
            with self.assertRaises(verifier.VerificationFailure) as self_pin:
                verifier.verify_profile(args)
            self.assertEqual(self_pin.exception.code, "verifier_pin_mismatch")
            helper_id = "packaging/scripts/build_archive_supply_chain.py"
            modified = json.loads(fixture.expected_inputs_bytes)
            next(item for item in modified["dependencies"] if item["id"] == helper_id)["sha256"] = "0" * 64
            fixture.write_inputs(modified)
            with self.assertRaises(verifier.VerificationFailure) as helper_pin:
                verifier.verify_profile(fixture.args())
            self.assertEqual(helper_pin.exception.code, "helper_pin_mismatch")

        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            with fixture.schema.open("ab") as stream:
                stream.write(b" ")
            with self.assertRaises(verifier.VerificationFailure) as schema_pin:
                verifier.verify_profile(fixture.args())
            self.assertEqual(schema_pin.exception.code, "schema_hash_mismatch")

    def test_original_zip_digest_mismatch_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            with fixture.archive_zip.open("ab") as stream:
                stream.write(b"tamper")
            with self.assertRaises(verifier.VerificationFailure) as raised:
                verifier.verify_profile(fixture.args())
            self.assertEqual(raised.exception.code, "archive_zip_identity")

    def test_rehashed_extra_and_missing_evidence_files_fail_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            _put(fixture.evidence / "unexpected.txt", b"extra")
            fixture.rebuild_supply_checksums()
            with self.assertRaises(verifier.VerificationFailure) as extra:
                verifier.verify_profile(fixture.args())
            self.assertEqual(extra.exception.code, "evidence_inventory")

        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            row = fixture.rows[0]
            identifier = hashlib.sha256(row["path"].encode()).hexdigest()
            (fixture.evidence / f"component-sboms/{identifier}.stderr").unlink()
            with self.assertRaises(verifier.VerificationFailure) as missing:
                verifier.verify_profile(fixture.args())
            self.assertEqual(missing.exception.code, "evidence_inventory")

    def test_provenance_subject_path_swap_and_invalid_receipt_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            statement_path = fixture.evidence / "provenance.json"
            statement = json.loads(statement_path.read_bytes())
            statement["subject"][0]["name"], statement["subject"][1]["name"] = statement["subject"][1]["name"], statement["subject"][0]["name"]
            _put(statement_path, _json_bytes(statement))
            fixture.rewrite_validation_receipt()
            fixture.rebuild_supply_checksums()
            with self.assertRaises(verifier.VerificationFailure) as swapped:
                verifier.verify_profile(fixture.args())
            self.assertEqual(swapped.exception.code, "provenance_invalid")

        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            receipt_path = fixture.evidence / "validation-result.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["valid"] = False
            _put(receipt_path, _json_bytes(receipt))
            fixture.rebuild_supply_checksums()
            with self.assertRaises(verifier.VerificationFailure) as invalid:
                verifier.verify_profile(fixture.args())
            self.assertEqual(invalid.exception.code, "validation_receipt_binding")

    def test_cross_archive_relationship_swap_omission_and_duplicate_fail_after_rehash(self) -> None:
        for mutation in ("swap", "omit", "duplicate"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                fixture = CompleteEvidenceFixture(Path(temporary).resolve())
                root_path = fixture.evidence / "sbom.spdx.json"
                root = json.loads(root_path.read_bytes())
                contains = [edge for edge in root["relationships"] if edge["relationshipType"] == "CONTAINS"]
                if mutation == "swap":
                    first = contains[0]
                    other_archive = next(pkg["SPDXID"] for pkg in root["packages"] if pkg["SPDXID"] != first["spdxElementId"])
                    first["spdxElementId"] = other_archive
                elif mutation == "omit":
                    root["relationships"].remove(contains[0])
                else:
                    root["relationships"].append(dict(contains[0]))
                _put(root_path, _json_bytes(root))
                fixture.rebuild_supply_checksums()
                with self.assertRaises(verifier.VerificationFailure) as raised:
                    verifier.verify_profile(fixture.args())
                self.assertEqual(raised.exception.code, "spdx_relationship_binding")

    def test_schema_invalid_root_spdx_fails_after_checksum_rehash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = CompleteEvidenceFixture(Path(temporary).resolve())
            root_path = fixture.evidence / "sbom.spdx.json"
            root = json.loads(root_path.read_bytes())
            del root["spdxVersion"]
            _put(root_path, _json_bytes(root))
            fixture.rebuild_supply_checksums()
            with self.assertRaises(verifier.VerificationFailure) as raised:
                verifier.verify_profile(fixture.args())
            self.assertEqual(raised.exception.code, "spdx_schema_invalid")

    def test_dynamic_seven_ecosystem_index_and_manifest_mapping(self) -> None:
        commit = "a" * 40
        receipt = {
            "source_commit": commit,
            "ecosystems": {
                ecosystem: {
                    "ecosystem": ecosystem,
                    "source_commit": commit,
                    "exit_status": 0,
                    "command": "fixture build",
                    "toolchain": "fixture 1",
                    "platform": "fixture",
                }
                for ecosystem in verifier.ECOSYSTEMS
            },
        }
        extensions = {
            "rust": ("rust", "crate", ".crate"),
            "python": ("python", "python-distribution", ".whl"),
            "r": ("r", "r-source-package", ".tar.gz"),
            "julia": ("julia", "julia-source-archive", ".tar.gz"),
            "typescript": ("typescript", "npm-package", ".tgz"),
            "nuget": ("nuget", "nuget-package", ".nupkg"),
            "go": ("go", "go-source-archive", ".tar.gz"),
        }
        rows = []
        for ecosystem, (prefix, kind, extension) in extensions.items():
            path = f"{prefix}/fixture-{ecosystem}{extension}"
            rows.append({
                "ecosystem": ecosystem,
                "kind": kind,
                "path": path,
                "bytes": 1,
                "sha256": hashlib.sha256(ecosystem.encode()).hexdigest(),
                "builder": receipt["ecosystems"][ecosystem],
            })
        rows.sort(key=lambda item: item["path"])
        index = {"schema_version": 1, "source_commit": commit, "created_at_utc": "fixture", "artifacts": rows}
        parsed = verifier.validate_builder_receipt(index, receipt, commit)
        index_sha = hashlib.sha256(b"index").hexdigest()
        manifest = {
            "schema_version": 1,
            "release_stage": "actual-package-archives",
            "source_commit": commit,
            "production_publish_enabled": False,
            "archive_index_sha256": index_sha,
            "artifacts": [
                {
                    "path": "archives/" + row["path"],
                    "sha256": row["sha256"],
                    "bytes": row["bytes"],
                    "ecosystem": "csharp" if row["ecosystem"] == "nuget" else row["ecosystem"],
                    "archive_ecosystem": row["ecosystem"],
                    "kind": row["kind"],
                }
                for row in parsed
            ],
        }
        manifest["artifacts"].sort(key=lambda item: item["path"])
        self.assertEqual(verifier.validate_manifest(manifest, parsed, index_sha, commit), manifest["artifacts"])
        self.assertEqual({row["ecosystem"] for row in parsed}, set(verifier.ECOSYSTEMS))

    def test_index_rejects_missing_ecosystem_even_when_rows_are_well_formed(self) -> None:
        commit = "b" * 40
        receipt = {
            "source_commit": commit,
            "ecosystems": {
                ecosystem: {"ecosystem": ecosystem, "source_commit": commit, "exit_status": 0,
                            "command": "x", "toolchain": "x", "platform": "x"}
                for ecosystem in verifier.ECOSYSTEMS
            },
        }
        row = {"ecosystem": "rust", "kind": "crate", "path": "rust/a.crate", "bytes": 1,
               "sha256": "0" * 64, "builder": receipt["ecosystems"]["rust"]}
        index = {"schema_version": 1, "source_commit": commit, "created_at_utc": "fixture", "artifacts": [row]}
        with self.assertRaises(verifier.VerificationFailure) as raised:
            verifier.validate_builder_receipt(index, receipt, commit)
        self.assertEqual(raised.exception.code, "archive_ecosystems")

    def test_expected_input_requires_exact_dependency_ids_and_five_field_shape(self) -> None:
        binding = {
            "schema_version": 1,
            "repository": "edithatogo/kairos",
            "source_commit": "c" * 40,
            "producer_pr_head": "d" * 40,
            "producer_tree": "e" * 40,
            "original_run_id": 123,
            "acquisition_artifact_id": 456,
            "archive_zip_sha256": "1" * 64,
            "archive_zip_bytes": 1234,
            "spdx_schema_sha256": "2" * 64,
        }
        deps = sorted(verifier.DEPENDENCY_IDS | {"https://github.com/edithatogo/kairos/actions/runs/123"})
        inputs = {
            "archive_index_sha256": "3" * 64,
            "source_commit": "c" * 40,
            "original_run_id": 123,
            "acquisition_artifact_id": 456,
            "dependencies": [{"id": item, "sha256": "4" * 64} for item in deps],
        }
        by_id = {item["id"]: item for item in inputs["dependencies"]}
        by_id["schema:spdx-2.3"]["sha256"] = binding["spdx_schema_sha256"]
        by_id["https://github.com/edithatogo/kairos/actions/runs/123"]["sha256"] = binding["archive_zip_sha256"]
        by_id["ARCHIVE-INDEX.json"]["sha256"] = inputs["archive_index_sha256"]
        by_id["build-inputs/ARCHIVE-INDEX.json"]["sha256"] = inputs["archive_index_sha256"]
        self.assertEqual(verifier.validate_expected_inputs(inputs, binding), inputs)
        inputs["dependencies"] = [entry for entry in inputs["dependencies"] if entry["id"] != "tool:syft"]
        with self.assertRaises(verifier.VerificationFailure) as raised:
            verifier.validate_expected_inputs(inputs, binding)
        self.assertEqual(raised.exception.code, "dependency_set")

    def test_checksum_parser_rejects_duplicate_and_unsafe_paths(self) -> None:
        digest = "a" * 64
        with self.assertRaises(verifier.VerificationFailure) as duplicate:
            verifier.parse_checksums(f"{digest}  a\n{digest}  a\n".encode(), "sums")
        self.assertEqual(duplicate.exception.code, "checksum_duplicate")
        for unsafe in ("../a", "/a", "a\\b", "a:b"):
            with self.subTest(unsafe=unsafe), self.assertRaises(verifier.VerificationFailure):
                verifier.parse_checksums(f"{digest}  {unsafe}\n".encode(), "sums")

    def test_secure_open_rejects_symlink_and_nonregular_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / "target"
            target.write_bytes(b"safe")
            link = root / "link"
            link.symlink_to(target)
            with self.assertRaises(verifier.VerificationFailure) as raised:
                with verifier.secure_open(link, "fixture"):
                    pass
            self.assertEqual(raised.exception.code, "unreadable_file")
            fifo = root / "fifo"
            os.mkfifo(fifo)
            with self.assertRaises(verifier.VerificationFailure) as raised_fifo:
                with verifier.secure_open(fifo, "fixture"):
                    pass
            self.assertIn(raised_fifo.exception.code, {"not_regular_file", "unreadable_file"})

    def test_zip_preflight_rejects_zip64_locator_and_central_directory_limit(self) -> None:
        valid = _make_zip({"a": b"x"})
        with io.BytesIO(valid) as stream:
            verifier.preflight_zip(stream, "valid")
            self.assertEqual(stream.tell(), 0)
        zip64 = valid[:-22] + b"PK\x06\x07" + b"\0" * 16 + valid[-22:]
        with self.assertRaises(verifier.VerificationFailure) as raised:
            verifier.preflight_zip(io.BytesIO(zip64), "zip64")
        self.assertEqual(raised.exception.code, "zip64_unsupported")
        central_limit = bytearray(valid)
        eocd = central_limit.rfind(b"PK\x05\x06")
        struct.pack_into("<I", central_limit, eocd + 12, verifier.MAX_ZIP_CENTRAL_DIRECTORY_BYTES + 1)
        with self.assertRaises(verifier.VerificationFailure) as limited:
            verifier.preflight_zip(io.BytesIO(central_limit), "central")
        self.assertEqual(limited.exception.code, "zip_directory_limit")

    def test_inner_zip_preflight_runs_before_zipfile_allocation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / "fixture.whl"
            _put(path, _make_zip({"fixture.dist-info/METADATA": b"Name: fixture\nVersion: 1\n"}))
            events = []
            real_preflight, real_zip = verifier.preflight_zip, verifier.zipfile.ZipFile
            def preflight(stream, label):
                events.append("preflight")
                return real_preflight(stream, label)
            def zipfile(*args, **kwargs):
                events.append("zipfile")
                return real_zip(*args, **kwargs)
            row = {"bytes": path.stat().st_size, "ecosystem": "python"}
            with mock.patch.object(verifier, "preflight_zip", side_effect=preflight), mock.patch.object(verifier.zipfile, "ZipFile", side_effect=zipfile):
                verifier.validate_package_archive(path, row, "fixture")
            self.assertEqual(events, ["preflight", "zipfile"])

    def test_tar_stream_budget_counts_pax_metadata_and_deadline(self) -> None:
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as archive:
            info = tarfile.TarInfo("x" * 300)
            payload = b"z"
            info.size = len(payload)
            archive.addfile(info, io.BytesIO(payload))
        compressed = gzip.compress(raw.getvalue())
        with gzip.GzipFile(fileobj=io.BytesIO(compressed), mode="rb") as stream:
            bounded = verifier.BoundedReader(stream, 100, 30, "pax-fixture")
            with self.assertRaises(verifier.VerificationFailure) as raised:
                while bounded.read(64):
                    pass
            self.assertEqual(raised.exception.code, "archive_stream_limit")
        expired = verifier.BoundedReader(io.BytesIO(b"x"), 10, 0, "deadline-fixture")
        with self.assertRaises(verifier.VerificationFailure) as deadline:
            expired.read()
        self.assertEqual(deadline.exception.code, "archive_parse_deadline")

    def test_cli_profile_deadline_interrupts_a_running_operation(self) -> None:
        with self.assertRaises(verifier.ProfileTimeout):
            with verifier.profile_deadline(0.02):
                time.sleep(0.2)

    def test_inventory_enumerates_extra_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "a").write_text("one")
            inventory, directories = verifier.scan_inventory(root)
            self.assertEqual(set(inventory), {"a"})
            self.assertEqual(directories, set())

    def test_bootstrap_json_rejects_duplicates_nan_and_depth(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, data in (("duplicate", b'{"x":1,"x":2}'), ("nan", b'{"x":NaN}'),
                               ("infinity", b'{"x":Infinity}'), ("overflow", b'{"x":1e999}'),
                               ("truncated", b'{"x":'), ("oversized", b'"' + b"x" * (verifier.MAX_JSON_BYTES + 1) + b'"')):
                path = root / name
                path.write_bytes(data)
                with self.subTest(name=name), self.assertRaises(verifier.VerificationFailure):
                    verifier.bootstrap_json(path, name)
            deep = root / "deep"
            deep.write_text("[" * 130 + "0" + "]" * 130)
            with self.assertRaises(verifier.VerificationFailure):
                verifier.bootstrap_json(deep, "deep")

    def test_manifest_binds_full_path_and_ecosystem_identity(self) -> None:
        row = {"ecosystem": "nuget", "kind": "nuget-package", "path": "nuget/x.nupkg", "bytes": 5, "sha256": "a" * 64}
        manifest = {
            "schema_version": 1, "release_stage": "actual-package-archives", "source_commit": "f" * 40,
            "production_publish_enabled": False, "archive_index_sha256": "b" * 64,
            "artifacts": [{"path": "archives/nuget/x.nupkg", "sha256": "a" * 64, "bytes": 5,
                           "ecosystem": "csharp", "archive_ecosystem": "nuget", "kind": "nuget-package"}],
        }
        self.assertEqual(len(verifier.validate_manifest(manifest, [row], "b" * 64, "f" * 40)), 1)
        manifest["artifacts"][0]["path"] = "archives/nuget/y.nupkg"
        with self.assertRaises(verifier.VerificationFailure) as raised:
            verifier.validate_manifest(manifest, [row], "b" * 64, "f" * 40)
        self.assertEqual(raised.exception.code, "manifest_archive_binding")

    def test_evidence_inventory_is_dynamic_by_archive_path_hash(self) -> None:
        rows = [
            {"path": "rust/a.crate"},
            {"path": "python/b.whl"},
        ]
        inventory = verifier.build_expected_inventory(rows)
        for row in rows:
            identifier = hashlib.sha256(row["path"].encode()).hexdigest()
            self.assertIn(f"component-sboms/{identifier}.spdx.json", inventory)
            self.assertIn(f"component-sboms/{identifier}.stdout", inventory)
            self.assertIn(f"component-sboms/{identifier}.stderr", inventory)
        self.assertNotIn("component-sboms/1.spdx.json", inventory)

    def test_relationships_bind_each_archive_to_only_its_component_packages(self) -> None:
        rows = [{"path": "go/a.tar.gz"}, {"path": "rust/b.crate"}]
        components = {
            "go/a.tar.gz": {"packages": []},
            "rust/b.crate": {"packages": [{"SPDXID": "SPDXRef-Package", "name": "fixture"}]},
        }
        go_id = hashlib.sha256(b"go/a.tar.gz").hexdigest()
        rust_id = hashlib.sha256(b"rust/b.crate").hexdigest()
        edges = [
            {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": f"SPDXRef-archive-{go_id}"},
            {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": f"SPDXRef-archive-{rust_id}"},
            {"spdxElementId": f"SPDXRef-archive-{rust_id}", "relationshipType": "CONTAINS", "relatedSpdxElement": f"DocumentRef-{rust_id}:SPDXRef-Package"},
        ]
        verifier.validate_archive_relationships(rows, components, edges)
        wrong_owner = list(edges)
        wrong_owner[-1] = {**wrong_owner[-1], "spdxElementId": f"SPDXRef-archive-{go_id}"}
        for mutation in (wrong_owner, edges[:-1], edges + [edges[-1]]):
            with self.subTest(mutation=mutation), self.assertRaises(verifier.VerificationFailure) as raised:
                verifier.validate_archive_relationships(rows, components, mutation)
            self.assertEqual(raised.exception.code, "spdx_relationship_binding")

    def test_coverage_row_rejects_unbound_extra_fields(self) -> None:
        digest = "a" * 64
        dependencies = {"packaging/scripts/" + helper: digest for helper in verifier.HELPERS}
        dependencies["tool:syft"] = digest
        dependencies["schema:spdx-2.3"] = digest
        row = {"ecosystem": "rust", "path": "rust/a.crate", "sha256": digest}
        identity = {"name": "fixture", "version": "1.0", "metadata_path": "Cargo.toml",
                    "metadata_sha256": digest, "repository_commit": "b" * 40}
        coverage_row = {"archive": row["path"], "archive_sha256": digest, "identity": identity,
                        "scanner_software_packages": 0, "scanner_identities": [],
                        "manifest_identity_fallback": False, "component_sbom": "component-sboms/x.spdx.json",
                        "component_sbom_sha256": digest, "unbound": True}
        coverage = {"coverage": [coverage_row], "source_identities": {helper: digest for helper in verifier.HELPERS},
                    "runtime": {}, "syft_sha256": digest, "schema_sha256": digest,
                    "provenance_scope": "Unsigned local copying/evidence build; does not claim original compilation attestation or SLSA level."}
        with self.assertRaises(verifier.VerificationFailure) as raised:
            verifier.validate_coverage(coverage, [row], dependencies,
                                      {"source_commit": "b" * 40, "spdx_schema_sha256": digest})
        self.assertEqual(raised.exception.code, "coverage_row_shape")


if __name__ == "__main__":
    unittest.main()
