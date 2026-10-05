import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import tarfile
import tempfile
import textwrap
import unittest
import zipfile
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "supply_chain", Path(__file__).resolve().parents[1] / "packaging/scripts/build_archive_supply_chain.py"
)
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)

COMMIT = "a" * 40
OTHER_COMMIT = "b" * 40
RUN_ID = 123456
REPO_ROOT = Path(__file__).resolve().parents[1]
SPDX_SCHEMA = REPO_ROOT / "tests/fixtures/archive-supply-chain/spdx-2.3/spdx-schema.json"


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def add_tar_gz(path, entries):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(path, "w:gz") as archive:
        for name, data in entries.items():
            raw = data.encode("utf-8") if isinstance(data, str) else data
            info = tarfile.TarInfo(name)
            info.size = len(raw)
            info.mtime = 0
            archive.addfile(info, io.BytesIO(raw))


def add_zip(path, entries):
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in entries.items():
            archive.writestr(name, data)


def make_fixture(root, nuspec_commit=COMMIT):
    """Build eight small archives through the repository's real bundle builder."""
    package_source = root / "package-source"
    package_source.mkdir()
    contents = {
        "rust": [
            ("rust-one-0.1.0.crate", {"rust-one-0.1.0/Cargo.toml": '[package]\nname="rust-one"\nversion="0.1.0"\n', "rust-one-0.1.0/src/lib.rs": "// fixture\n"}),
            ("rust-two-0.2.0.crate", {"rust-two-0.2.0/Cargo.toml": '[package]\nname="rust-two"\nversion="0.2.0"\n', "rust-two-0.2.0/src/lib.rs": "// fixture\n"}),
        ],
        "python": [("demo_python-1.2.3-py3-none-any.whl", {"demo_python-1.2.3.dist-info/METADATA": "Metadata-Version: 2.1\nName: demo-python\nVersion: 1.2.3\n"})],
        "r": [("demo-r_1.0.0.tar.gz", {"demo-r/DESCRIPTION": "Package: demo-r\nVersion: 1.0.0\n"})],
        "julia": [("demo-julia-1.0.0.tar.gz", {"demo-julia/Project.toml": 'name = "demo-julia"\nversion = "1.0.0"\n'})],
        "typescript": [("demo-typescript-1.0.0.tgz", {"package/package.json": json.dumps({"name": "demo-typescript", "version": "1.0.0"})})],
        "nuget": [("Demo.Nuget.1.0.0.nupkg", {"Demo.Nuget.nuspec": f'<package><metadata><id>Demo.Nuget</id><version>1.0.0</version><repository type="git" url="https://example.invalid/repo" commit="{nuspec_commit}" /></metadata></package>'})],
        "go": [("demo-go.tar.gz", {"demo-go/go.mod": "module example.invalid/demo-go\ngo 1.24\n"})],
    }
    for ecosystem, rows in contents.items():
        folder = package_source / ecosystem
        folder.mkdir()
        (folder / "BUILD-INFO.json").write_text(json.dumps({
            "ecosystem": ecosystem,
            "source_commit": COMMIT,
            "command": f"synthetic {ecosystem} fixture",
            "toolchain": "fixture-toolchain 1",
            "platform": "test-platform",
            "exit_status": 0,
        }))
        for filename, entries in rows:
            archive = folder / filename
            if filename.endswith((".crate", ".tgz", ".tar.gz")):
                add_tar_gz(archive, entries)
            else:
                add_zip(archive, entries)

    bundle = root / "verified-bundle"
    builder = module.load_helper("build_package_archive_bundle")
    builder.build(package_source, bundle, COMMIT)
    index = json.loads((bundle / "ARCHIVE-INDEX.json").read_text())
    assert len(index["artifacts"]) == 8
    assert len({row["ecosystem"] for row in index["artifacts"]}) == 7

    acquisition = root / "acquisition.json"
    acquisition.write_text(json.dumps({
        "repository": "edithatogo/kairos",
        "run_id": RUN_ID,
        "source_commit": COMMIT,
        "artifact_digest": "sha256:" + "c" * 64,
        "archive_index_sha256": sha256(bundle / "ARCHIVE-INDEX.json"),
        "artifact_id": 987654,
    }))
    control = root / "scanner-control.json"
    control.write_text(json.dumps({"fault": None, "ecosystem": None, "namespace": None}))
    syft = root / "fake-syft"
    generator_path = str(Path(module.__file__).resolve())
    control_path = str(control.resolve())
    syft.write_text(textwrap.dedent(f"""\
        #!{os.sys.executable}
        import importlib.util, json, sys, uuid
        from pathlib import Path, PurePosixPath
        spec = importlib.util.spec_from_file_location("fixture_generator", {generator_path!r})
        generator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(generator)
        args = sys.argv[1:]
        out_arg = next(arg for arg in args if arg.startswith("spdx-json="))
        output = Path(out_arg.split("=", 1)[1])
        tree = Path(next(arg[4:] for arg in args if arg.startswith("dir:")))
        source_name = args[args.index("--source-name") + 1]
        ecosystem = PurePosixPath(source_name).parts[0]
        control = json.loads(Path({control_path!r}).read_text())
        ident = generator.identity(tree, ecosystem)
        fault = control.get("fault") if control.get("ecosystem") in (None, ecosystem) else None
        packages = []
        if ecosystem not in ("go", "julia") and fault != "missing-package":
            name = ident["name"]
            version = ident["version"]
            if ecosystem == "nuget": version = version + "+" + {COMMIT!r}
            if fault == "wrong-name": name = "wrong-package"
            if fault == "wrong-version": version = "9.9.9"
            if fault == "nuget-exact-version": version = ident["version"]
            if fault == "nuget-wrong-suffix": version = ident["version"] + "+" + {OTHER_COMMIT!r}
            if fault == "nuget-explicit-commit": version = ident["version"] + "+" + {COMMIT!r}
            spdx_id = "SPDXRef-Package"
            if fault == "invalid-spdx-id": spdx_id = None
            package = {{"name": name, "SPDXID": spdx_id, "versionInfo": version,
                       "downloadLocation": "NOASSERTION", "filesAnalyzed": False,
                       "licenseConcluded": "NOASSERTION", "licenseDeclared": "NOASSERTION",
                       "copyrightText": "NOASSERTION"}}
            packages.append(package)
            if fault == "duplicate-spdx-id": packages.append(dict(package, name=name + "-duplicate"))
        if ecosystem in ("go", "julia"):
            packages.append({{"name": ident["name"], "SPDXID": "SPDXRef-FixtureFile",
                             "primaryPackagePurpose": "FILE", "downloadLocation": "NOASSERTION",
                             "filesAnalyzed": False, "licenseConcluded": "NOASSERTION",
                             "licenseDeclared": "NOASSERTION", "copyrightText": "NOASSERTION"}})
        namespace = control.get("namespace") or "https://example.invalid/syft/" + str(uuid.uuid4())
        document = {{"spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0",
                    "SPDXID": "SPDXRef-DOCUMENT", "name": "synthetic scanner result",
                    "documentNamespace": namespace,
                    "creationInfo": {{"created": "2026-10-04T00:00:00Z", "creators": ["Tool: fake-syft 1"]}},
                    "packages": packages}}
        output.write_text(json.dumps(document))
        """))
    syft.chmod(0o755)
    return {"source": bundle, "acquisition": acquisition, "syft": syft, "control": control, "index": index}


def write_control(fixture, fault=None, ecosystem=None, namespace=None):
    fixture["control"].write_text(json.dumps({"fault": fault, "ecosystem": ecosystem, "namespace": namespace}))


def generate(fixture, output, schema=SPDX_SCHEMA):
    return module.generate(
        fixture["source"], output, COMMIT, fixture["acquisition"],
        fixture["syft"], module.digest(fixture["syft"]), schema,
        module.digest(schema), module.digest(fixture["acquisition"]), RUN_ID,
    )


def checksum_inventory(output):
    checksum_file = output / "SUPPLY-CHAIN-SHA256SUMS"
    listed = {}
    for line in checksum_file.read_text().splitlines():
        digest, relative = line.split("  ", 1)
        if relative in listed:
            raise AssertionError(f"duplicate checksum path: {relative}")
        listed[relative] = digest
    actual = {
        path.relative_to(output).as_posix()
        for path in output.rglob("*")
        if path.is_file() and path != checksum_file
    }
    if set(listed) != actual:
        raise AssertionError(f"checksum inventory differs: missing={actual-set(listed)}, extra={set(listed)-actual}")
    for relative, digest in listed.items():
        if sha256(output / relative) != digest:
            raise AssertionError(f"checksum mismatch: {relative}")
    return listed


def assert_spdx_graph(test, output, root_doc_override=None):
    root_doc = root_doc_override if root_doc_override is not None else json.loads((output / "sbom.spdx.json").read_text())
    coverage = json.loads((output / "SBOM-COVERAGE.json").read_text())["coverage"]
    components = {}
    namespaces = set()
    for item in coverage:
        component = json.loads((output / item["component_sbom"]).read_text())
        test.assertNotIn(component["documentNamespace"], namespaces)
        namespaces.add(component["documentNamespace"])
        package_ids = [p["SPDXID"] for p in component.get("packages", [])]
        test.assertEqual(len(package_ids), len(set(package_ids)), "SPDXID must be unique within one document")
        components[component["documentNamespace"]] = component
    test.assertNotIn(root_doc["documentNamespace"], namespaces)
    namespaces.add(root_doc["documentNamespace"])
    refs = root_doc.get("externalDocumentRefs", [])
    ref_ids = [ref["externalDocumentId"] for ref in refs]
    ref_namespaces = [ref["spdxDocument"] for ref in refs]
    test.assertEqual(len(ref_ids), len(set(ref_ids)), "external document IDs must be unique")
    test.assertEqual(len(ref_namespaces), len(set(ref_namespaces)), "external document namespaces must be unique")
    ref_by_id = {ref["externalDocumentId"]: ref for ref in refs}
    test.assertEqual(set(ref_namespaces), set(components))
    for ref in refs:
        component = components[ref["spdxDocument"]]
        component_path = next(item["component_sbom"] for item in coverage if json.loads((output / item["component_sbom"]).read_text())["documentNamespace"] == ref["spdxDocument"])
        test.assertEqual(ref["checksum"]["checksumValue"], sha256(output / component_path))
        # Component SPDXIDs are scoped to their own document; repeated values in
        # different component documents are valid and are checked independently.
        test.assertEqual(len([p["SPDXID"] for p in component.get("packages", [])]), len(set(p["SPDXID"] for p in component.get("packages", []))))
    root_ids = [p["SPDXID"] for p in root_doc.get("packages", [])]
    test.assertEqual(len(root_ids), len(set(root_ids)))
    descriptions = [r for r in root_doc.get("relationships", []) if r["relationshipType"] == "DESCRIBES"]
    test.assertEqual({r["relatedSpdxElement"] for r in descriptions}, set(root_ids))
    for relation in root_doc.get("relationships", []):
        if relation["relationshipType"] != "CONTAINS":
            continue
        test.assertIn(relation["spdxElementId"], set(root_ids))
        docref, package_id = relation["relatedSpdxElement"].split(":", 1)
        test.assertIn(docref, ref_by_id)
        component = components[ref_by_id[docref]["spdxDocument"]]
        test.assertIn(package_id, {p["SPDXID"] for p in component.get("packages", [])})
    return root_doc, components


class SupplyChainTests(unittest.TestCase):
    def test_zip_rejects_traversal_duplicates_and_links(self):
        for fault in ("traversal", "duplicate", "link"):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as d:
                root = Path(d); archive = root / "archive.zip"
                with zipfile.ZipFile(archive, "w") as z:
                    if fault == "traversal": z.writestr("../escape", b"bad")
                    elif fault == "duplicate":
                        z.writestr("same", b"a"); z.writestr("same", b"b")
                    else:
                        i = zipfile.ZipInfo("link"); i.create_system = 3
                        i.external_attr = (stat.S_IFLNK | 0o777) << 16
                        z.writestr(i, "outside")
                with self.assertRaises(ValueError): module.extract(archive, root / "out")
                self.assertFalse((root / "escape").exists())

    def test_tar_rejects_link_and_extraction_budget(self):
        for fault in ("link", "budget"):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as d:
                root = Path(d); archive = root / "archive.tar.gz"
                with tarfile.open(archive, "w:gz") as z:
                    i = tarfile.TarInfo("entry")
                    if fault == "link": i.type = tarfile.SYMTYPE; i.linkname = "outside"; z.addfile(i)
                    else: i.size = 4; z.addfile(i, io.BytesIO(b"data"))
                old = module.LIMIT
                try:
                    if fault == "budget": module.LIMIT = 3
                    with self.assertRaises(ValueError): module.extract(archive, root / "out")
                finally: module.LIMIT = old

    def test_go_identity_does_not_invent_version(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); (root / "go.mod").write_text("module example.org/pkg\ngo 1.24\n")
            found = module.identity(root, "go")
            self.assertEqual(found["name"], "example.org/pkg")
            self.assertIsNone(found["version"])
            self.assertEqual(found["metadata_sha256"], module.digest(root / "go.mod"))

    def test_manifest_identity_requires_unique_root_and_version(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "package.json").write_text(json.dumps({"name": "pkg"}))
            with self.assertRaises(ValueError): module.identity(root, "typescript")
            (root / "package.json").unlink()
            for name in ("a", "b"):
                (root / name).mkdir(); (root / name / "package.json").write_text(json.dumps({"name": "pkg", "version": "1"}))
            with self.assertRaises(ValueError): module.identity(root, "typescript")

    def test_acquisition_identity_rejects_wrong_repo_run_and_digest(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); tool = root / "tool"; schema = root / "schema"; acq = root / "acq"
            tool.write_bytes(b"tool"); schema.write_text("{}")
            base = {"repository": "edithatogo/kairos", "run_id": 12, "source_commit": "a" * 40, "artifact_digest": "sha256:" + "b" * 64}
            for change in ({"repository": "other/repo"}, {"run_id": 13}, {"run_id": True}, {"artifact_digest": "missing"}):
                acq.write_text(json.dumps(base | change))
                with self.assertRaises(ValueError):
                    module.generate(root / "absent", root / "output", "a" * 40, acq, tool, module.digest(tool), schema, module.digest(schema), module.digest(acq), 12)
                self.assertFalse((root / "output").exists())

    def test_acquisition_artifact_id_must_be_positive_exact_integer_before_scanning(self):
        for invalid in (True, 1.5, 0, -1, "987654"):
            with self.subTest(invalid=invalid), tempfile.TemporaryDirectory() as d:
                root = Path(d); fixture = make_fixture(root); output = root / "output"
                receipt = json.loads(fixture["acquisition"].read_text())
                receipt["artifact_id"] = invalid
                fixture["acquisition"].write_text(json.dumps(receipt))
                with patch.object(module, "load_helper") as adapter, patch.object(module, "run_scanner") as scanner:
                    with self.assertRaisesRegex(ValueError, "positive integer"):
                        generate(fixture, output)
                adapter.assert_not_called()
                scanner.assert_not_called()
                self.assertFalse(output.exists())

    def test_generate_eight_archive_sbom_provenance_and_exact_checksums(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            fixture = make_fixture(root)
            output = root / "evidence"
            generate(fixture, output)
            self.assertEqual(len(fixture["index"]["artifacts"]), 8)
            coverage_doc = json.loads((output / "SBOM-COVERAGE.json").read_text())
            coverage = coverage_doc["coverage"]
            self.assertEqual(len(coverage), 8)
            self.assertEqual({row["identity"]["name"] for row in coverage}, {
                "rust-one", "rust-two", "demo-python", "demo-r", "demo-julia",
                "demo-typescript", "Demo.Nuget", "example.invalid/demo-go",
            })
            self.assertEqual({row["identity"]["version"] for row in coverage if row["identity"]["name"] == "example.invalid/demo-go"}, {None})
            self.assertEqual({row["archive"].split("/", 1)[0] for row in coverage}, {"rust", "python", "r", "julia", "typescript", "nuget", "go"})
            self.assertEqual(sum(row["manifest_identity_fallback"] for row in coverage), 2)
            nuget = next(row for row in coverage if row["identity"]["name"] == "Demo.Nuget")
            self.assertEqual(nuget["identity"]["version"], "1.0.0")
            self.assertIn({"name": "Demo.Nuget", "version": "1.0.0+" + COMMIT}, nuget["scanner_identities"])
            for row in fixture["index"]["artifacts"]:
                archive = output / "archives" / row["path"]
                self.assertEqual(archive.stat().st_size, row["bytes"])
                self.assertEqual(sha256(archive), row["sha256"])
            schema_data = json.loads(SPDX_SCHEMA.read_text())
            import jsonschema
            validator_cls = jsonschema.validators.validator_for(schema_data)
            validator_cls.check_schema(schema_data)
            validator = validator_cls(schema_data, format_checker=jsonschema.FormatChecker())
            root_doc, components = assert_spdx_graph(self, output)
            validator.validate(root_doc)
            for component in components.values(): validator.validate(component)
            corrupt_ref = json.loads(json.dumps(root_doc))
            corrupt_ref["externalDocumentRefs"][0]["checksum"]["checksumValue"] = "0" * 64
            with self.assertRaises(AssertionError): assert_spdx_graph(self, output, corrupt_ref)
            corrupt_target = json.loads(json.dumps(root_doc))
            contains = next(row for row in corrupt_target["relationships"] if row["relationshipType"] == "CONTAINS")
            contains["relatedSpdxElement"] = "DocumentRef-absent:SPDXRef-absent"
            with self.assertRaises(AssertionError): assert_spdx_graph(self, output, corrupt_target)
            provenance = json.loads((output / "provenance.json").read_text())
            self.assertEqual(
                provenance["predicate"]["buildDefinition"]["buildType"],
                "urn:careops:build-type:verified-archive-copy:v1",
            )
            self.assertEqual(
                provenance["predicate"]["runDetails"]["builder"]["id"],
                "urn:careops:local-untrusted-builder:archive-copy",
            )
            metadata = provenance["predicate"]["runDetails"]["metadata"]
            self.assertRegex(metadata["startedOn"], r"^\d{4}-\d\d-\d\dT.*Z$")
            self.assertRegex(metadata["finishedOn"], r"^\d{4}-\d\d-\d\dT.*Z$")
            release_manifest = json.loads((output / "release-artifact-manifest.json").read_text())
            expected_subjects = {(row["path"], row["sha256"]) for row in release_manifest["artifacts"]}
            actual_subjects = {(row["name"], row["digest"]["sha256"]) for row in provenance["subject"]}
            self.assertEqual(actual_subjects, expected_subjects)
            dependencies = provenance["predicate"]["buildDefinition"]["resolvedDependencies"]
            by_name = {row["name"]: row for row in dependencies}
            self.assertEqual(len(by_name), len(dependencies))
            expected_dependency_names = {
                f"https://github.com/edithatogo/kairos/actions/runs/{RUN_ID}",
                "ARCHIVE-INDEX.json", "build-inputs/ARCHIVE-INDEX.json",
                "build-inputs/BUILD-RECEIPT.json", "build-inputs/acquisition.json",
                "packaging/scripts/build_archive_supply_chain.py",
                "packaging/scripts/build_archive_release_manifest.py",
                "packaging/scripts/build_package_archive_bundle.py",
                "packaging/scripts/acquire_package_archive_bundle.py",
                "packaging/scripts/validate_archive_copy_provenance.py",
                "tool:syft", "schema:spdx-2.3",
            }
            self.assertEqual(set(by_name), expected_dependency_names)
            validator = module.load_provenance_validator()
            for name, row in by_name.items():
                self.assertEqual(row["uri"], validator.canonical_dependency_uri(name, RUN_ID))
            expected_inputs = json.loads((output / "expected-inputs.json").read_text())
            self.assertEqual(expected_inputs["source_commit"], COMMIT)
            self.assertEqual(expected_inputs["original_run_id"], RUN_ID)
            self.assertEqual(expected_inputs["acquisition_artifact_id"], 987654)
            self.assertEqual(expected_inputs["archive_index_sha256"], sha256(fixture["source"] / "ARCHIVE-INDEX.json"))
            expected_digests = {item["id"]: item["sha256"] for item in expected_inputs["dependencies"]}
            self.assertEqual(expected_digests["ARCHIVE-INDEX.json"], expected_digests["build-inputs/ARCHIVE-INDEX.json"])
            self.assertEqual(expected_digests["build-inputs/ARCHIVE-INDEX.json"], sha256(output / "build-inputs/ARCHIVE-INDEX.json"))
            self.assertEqual(expected_digests["build-inputs/BUILD-RECEIPT.json"], sha256(output / "build-inputs/BUILD-RECEIPT.json"))
            self.assertEqual(expected_digests["build-inputs/acquisition.json"], sha256(output / "build-inputs/acquisition.json"))
            for name in (
                "build_archive_supply_chain.py", "build_archive_release_manifest.py",
                "build_package_archive_bundle.py", "acquire_package_archive_bundle.py",
                "validate_archive_copy_provenance.py",
            ):
                dep_name = "packaging/scripts/" + name
                self.assertEqual(expected_digests[dep_name], sha256(REPO_ROOT / "packaging/scripts" / name))
            self.assertEqual(expected_digests["tool:syft"], sha256(fixture["syft"]))
            self.assertEqual(expected_digests["schema:spdx-2.3"], sha256(SPDX_SCHEMA))
            validation = json.loads((output / "validation-result.json").read_text())
            self.assertTrue(validation["valid"])
            self.assertEqual(validation["validator_sha256"], expected_digests["packaging/scripts/validate_archive_copy_provenance.py"])
            self.assertEqual(validation["issue_count"], 0)
            self.assertIn("Unsigned local copying/evidence build", coverage_doc["provenance_scope"])
            self.assertEqual(coverage_doc["provenance_scope"], "Unsigned local copying/evidence build; does not claim original compilation attestation or SLSA level.")
            listed = checksum_inventory(output)
            self.assertEqual(set(listed), {p.relative_to(output).as_posix() for p in output.rglob("*") if p.is_file() and p.name != "SUPPLY-CHAIN-SHA256SUMS"})
            old = (output / "sbom.spdx.json").read_bytes()
            (output / "sbom.spdx.json").write_bytes(old + b" ")
            with self.assertRaises(AssertionError): checksum_inventory(output)
            (output / "sbom.spdx.json").write_bytes(old)
            (output / "unexpected.bin").write_bytes(b"unexpected")
            with self.assertRaises(AssertionError): checksum_inventory(output)

            second = root / "evidence-second"
            generate(fixture, second)
            second_doc = json.loads((second / "sbom.spdx.json").read_text())
            self.assertNotEqual(root_doc["documentNamespace"], second_doc["documentNamespace"])

    def test_nuget_version_suffix_requires_exact_recorded_commit(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            fixture = make_fixture(root)
            output = root / "exact-version"
            write_control(fixture, "nuget-exact-version", "nuget")
            generate(fixture, output)
            nuget = next(row for row in json.loads((output / "SBOM-COVERAGE.json").read_text())["coverage"] if row["identity"]["name"] == "Demo.Nuget")
            self.assertIn({"name": "Demo.Nuget", "version": "1.0.0"}, nuget["scanner_identities"])
            for fault, nuspec_commit in (("nuget-wrong-suffix", COMMIT), ("nuget-explicit-commit", OTHER_COMMIT)):
                with self.subTest(fault=fault), tempfile.TemporaryDirectory() as case_dir:
                    case = Path(case_dir)
                    bad_fixture = make_fixture(case, nuspec_commit=nuspec_commit)
                    write_control(bad_fixture, fault, "nuget")
                    rejected = case / "rejected"
                    with self.assertRaises(ValueError): generate(bad_fixture, rejected)
                    self.assertFalse(rejected.exists())

    def test_scanner_identity_must_match_except_documented_fallbacks(self):
        for fault in ("missing-package", "wrong-name", "wrong-version", "invalid-spdx-id"):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as d:
                root = Path(d); fixture = make_fixture(root)
                write_control(fixture, fault, "typescript")
                output = root / "rejected"
                with self.assertRaises(Exception): generate(fixture, output)
                self.assertFalse(output.exists())

    def test_duplicate_scanner_spdx_ids_and_document_namespaces_fail_closed(self):
        for fault in ("duplicate-spdx-id", "duplicate-document-namespace"):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as d:
                root = Path(d); fixture = make_fixture(root)
                if fault == "duplicate-document-namespace":
                    write_control(fixture, None, None, "https://example.invalid/duplicate")
                else:
                    write_control(fixture, fault, "typescript")
                output = root / "rejected"
                with self.assertRaises(ValueError): generate(fixture, output)
                self.assertFalse(output.exists())

    def test_invalid_spdx_schema_fails_and_cleans_partially_copied_output(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root)
            invalid_schema = root / "invalid-schema.json"
            invalid_schema.write_text(json.dumps({"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "not-a-json-schema-type"}))
            output = root / "rejected"
            with self.assertRaises(Exception): generate(fixture, output, invalid_schema)
            self.assertFalse(output.exists())

    def test_mutated_delivered_archive_fails_and_cleans_output(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root); output = root / "mutated-output"
            real_run = module.run_scanner
            changed = False
            def scan_then_mutate(command, env, stdout_path, stderr_path, sbom_path):
                nonlocal changed
                result = real_run(command, env, stdout_path, stderr_path, sbom_path)
                if not changed:
                    source_name = command[command.index("--source-name") + 1]
                    copied = output / "archives" / source_name
                    raw = copied.read_bytes()
                    copied.write_bytes(bytes([raw[0] ^ 1]) + raw[1:])
                    changed = True
                return result
            with patch.object(module, "run_scanner", side_effect=scan_then_mutate):
                with self.assertRaises(ValueError): generate(fixture, output)
            self.assertTrue(changed)
            self.assertFalse(output.exists())


    def test_scanner_environment_and_acquisition_helper_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = make_fixture(root)
            output = root / "evidence"
            with patch.object(module, "run_scanner", wraps=module.run_scanner) as calls:
                generate(fixture, output)
            self.assertEqual(len(calls.call_args_list), 8)
            for call in calls.call_args_list:
                self.assertNotIn("HOME", call.args[1])
            coverage = json.loads((output / "SBOM-COVERAGE.json").read_text())
            helper = "acquire_package_archive_bundle.py"
            expected = sha256(REPO_ROOT / "packaging/scripts" / helper)
            self.assertEqual(coverage["source_identities"][helper], expected)
            provenance = json.loads((output / "provenance.json").read_text())
            dependencies = provenance["predicate"]["buildDefinition"]["resolvedDependencies"]
            self.assertIn({"name": "packaging/scripts/" + helper, "uri": module.load_provenance_validator().canonical_dependency_uri("packaging/scripts/" + helper, RUN_ID), "digest": {"sha256": expected}}, dependencies)

    def test_provenance_validator_cli_reads_serialized_statement_and_independent_inputs(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root); output = root / "evidence"
            generate(fixture, output)
            validator_path = REPO_ROOT / "packaging/scripts/validate_archive_copy_provenance.py"
            command = [
                os.sys.executable, str(validator_path),
                "--statement", str(output / "provenance.json"),
                "--archive-index", str(output / "build-inputs/ARCHIVE-INDEX.json"),
                "--expected-inputs", str(output / "expected-inputs.json"),
            ]
            passed = __import__("subprocess").run(command, capture_output=True, text=True, check=False)
            self.assertEqual(passed.returncode, 0, passed.stderr + passed.stdout)
            self.assertTrue(json.loads(passed.stdout)["valid"])
            statement = json.loads((output / "provenance.json").read_text())
            statement["subject"][0]["digest"]["sha256"] = "0" * 64
            (output / "provenance.json").write_text(json.dumps(statement))
            failed = __import__("subprocess").run(command, capture_output=True, text=True, check=False)
            self.assertEqual(failed.returncode, 1, failed.stderr + failed.stdout)
            self.assertFalse(json.loads(failed.stdout)["valid"])

    def test_provenance_validation_failure_cleans_fresh_output_and_preserves_existing(self):
        validator = module.load_provenance_validator()
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root); fresh = root / "fresh"
            issue = validator.ValidationIssue("injected_failure", "$.predicate", "test failure")
            with patch.object(module, "load_provenance_validator", return_value=validator), patch.object(validator, "validate_provenance", return_value=[issue]):
                with self.assertRaisesRegex(ValueError, "injected_failure"):
                    generate(fixture, fresh)
            self.assertFalse(fresh.exists())
            existing = root / "existing"; existing.mkdir(); (existing / "keep").write_text("keep")
            before = {path.relative_to(existing).as_posix(): sha256(path) for path in existing.rglob("*") if path.is_file()}
            with self.assertRaisesRegex(ValueError, "output must not exist"):
                generate(fixture, existing)
            after = {path.relative_to(existing).as_posix(): sha256(path) for path in existing.rglob("*") if path.is_file()}
            self.assertEqual(after, before)

    def test_missing_or_raising_validator_fails_before_output_or_scan(self):
        for failure in (FileNotFoundError("validator missing"), RuntimeError("validator import failed")):
            with self.subTest(failure=type(failure).__name__), tempfile.TemporaryDirectory() as d:
                root = Path(d); fixture = make_fixture(root); output = root / "fresh"
                with patch.object(module, "load_provenance_validator", side_effect=failure), patch.object(module, "load_helper") as adapter, patch.object(module, "run_scanner") as scanner:
                    with self.assertRaises(type(failure)):
                        generate(fixture, output)
                adapter.assert_not_called()
                scanner.assert_not_called()
                self.assertFalse(output.exists())

    def test_serialized_expected_input_tampering_fails_and_cleans_output(self):
        validator = module.load_provenance_validator()
        real_loader = validator.load_json_file
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root); output = root / "substitution"
            def tampering_loader(path, label, max_bytes=validator.DEFAULT_MAX_BYTES):
                if label == "independent expected inputs":
                    value = json.loads(Path(path).read_text())
                    value["source_commit"] = OTHER_COMMIT
                    Path(path).write_text(json.dumps(value, sort_keys=True))
                return real_loader(path, label, max_bytes)
            with patch.object(module, "load_provenance_validator", return_value=validator), patch.object(validator, "load_json_file", side_effect=tampering_loader):
                with self.assertRaisesRegex(ValueError, "serialized expected inputs differ"):
                    generate(fixture, output)
            self.assertFalse(output.exists())

    def test_helper_hash_drift_before_validation_fails_and_cleans_output(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root); output = root / "source-drift"
            real_digest = module.digest
            drift_path = (REPO_ROOT / "packaging/scripts/build_archive_release_manifest.py").resolve()
            calls = 0
            def drifting_digest(path):
                nonlocal calls
                resolved = Path(path).resolve()
                if resolved == drift_path:
                    calls += 1
                    if calls == 2:
                        return "f" * 64
                return real_digest(path)
            with patch.object(module, "digest", side_effect=drifting_digest):
                with self.assertRaisesRegex(ValueError, "source changed before validation"):
                    generate(fixture, output)
            self.assertEqual(calls, 2)
            self.assertFalse(output.exists())

    def test_graph_references_resolve_local_and_external_ids(self):
        document = {"SPDXID": "SPDXRef-DOCUMENT", "documentNamespace": "urn:aggregate",
            "packages": [{"SPDXID": "SPDXRef-Package"}],
            "externalDocumentRefs": [{"externalDocumentId": "DocumentRef-component", "spdxDocument": "urn:component", "checksum": {"algorithm": "SHA256", "checksumValue": "a" * 64}}],
            "relationships": [{"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-Package"},
                {"spdxElementId": "SPDXRef-Package", "relationshipType": "CONTAINS", "relatedSpdxElement": "DocumentRef-component:SPDXRef-Found"}]}
        target = {"DocumentRef-component": {"namespace": "urn:component", "sha256": "a" * 64, "ids": {"SPDXRef-DOCUMENT", "SPDXRef-Found"}}}
        module.validate_component_graph(document, set(), target)

    def test_dangling_local_endpoint_and_reserved_values_are_scoped(self):
        base = {"SPDXID": "SPDXRef-DOCUMENT", "documentNamespace": "urn:graph", "packages": [{"SPDXID": "SPDXRef-Package"}]}
        dangling = {**base, "relationships": [{"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-Missing"}]}
        with self.assertRaisesRegex(ValueError, "dangling local"):
            module.validate_component_graph(dangling, set())
        valid = {**base, "relationships": [{"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "OTHER", "relatedSpdxElement": "NONE"}]}
        module.validate_component_graph(valid, set())
        invalid = {**base, "relationships": [{"spdxElementId": "NOASSERTION", "relationshipType": "OTHER", "relatedSpdxElement": "SPDXRef-Package"}]}
        with self.assertRaisesRegex(ValueError, "dangling local"):
            module.validate_component_graph(invalid, set())

    def test_external_duplicate_and_dangling_references_fail(self):
        ref = {"externalDocumentId": "DocumentRef-component", "spdxDocument": "urn:component", "checksum": {"algorithm": "SHA256", "checksumValue": "a" * 64}}
        target = {"DocumentRef-component": {"namespace": "urn:component", "sha256": "a" * 64, "ids": {"SPDXRef-DOCUMENT"}}}
        document = {"SPDXID": "SPDXRef-DOCUMENT", "documentNamespace": "urn:aggregate", "packages": [{"SPDXID": "SPDXRef-Package"}], "externalDocumentRefs": [ref, dict(ref)], "relationships": []}
        with self.assertRaisesRegex(ValueError, "duplicate external"):
            module.validate_component_graph(document, set(), target)
        document["externalDocumentRefs"] = [ref]
        document["relationships"] = [{"spdxElementId": "SPDXRef-Package", "relationshipType": "CONTAINS", "relatedSpdxElement": "DocumentRef-component:SPDXRef-Missing"}]
        with self.assertRaisesRegex(ValueError, "dangling external"):
            module.validate_component_graph(document, set(), target)
        document["externalDocumentRefs"][0]["spdxDocument"] = "urn:wrong"
        with self.assertRaisesRegex(ValueError, "identity differs"):
            module.validate_component_graph(document, set(), target)
        document["externalDocumentRefs"][0]["spdxDocument"] = "urn:component"
        document["externalDocumentRefs"][0]["checksum"]["checksumValue"] = "b" * 64
        with self.assertRaisesRegex(ValueError, "identity differs"):
            module.validate_component_graph(document, set(), target)
        document["externalDocumentRefs"][0]["checksum"] = {"algorithm": "SHA1", "checksumValue": "a" * 64}
        with self.assertRaisesRegex(ValueError, "invalid external"):
            module.validate_component_graph(document, set(), target)

    def test_archive_identifier_collision_fails_before_output_creation(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture = make_fixture(root); output = root / "collision"
            with patch.object(module, "archive_identifier", return_value="f" * 64):
                with self.assertRaisesRegex(ValueError, "identifier collision"):
                    generate(fixture, output)
            self.assertFalse(output.exists())

    def test_provenance_timestamps_use_rfc3339_utc_suffix(self):
        self.assertRegex(module.utc_timestamp(), r"^\d{4}-\d\d-\d\dT.*Z$")

    def test_scanner_bounds_stderr_and_direct_sbom_output(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); previous = module.SCANNER_OUTPUT_LIMIT; module.SCANNER_OUTPUT_LIMIT = 4096
            try:
                for code, label in (("import sys;sys.stderr.write('x'*100000)", "stderr"), ("import sys;open(sys.argv[-1],'w').write('x'*100000)", "sbom")):
                    with self.subTest(label=label), self.assertRaisesRegex(ValueError, "output limit"):
                        output_path = root / (label + ".sbom")
                        module.run_scanner([os.sys.executable, "-c", code, str(output_path)], os.environ.copy(), root / (label + ".stdout"), root / (label + ".stderr"), output_path)
            finally:
                module.SCANNER_OUTPUT_LIMIT = previous

    def test_scanner_bounds_stdout_and_preserves_bounded_failure_tail(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); previous = module.SCANNER_OUTPUT_LIMIT; module.SCANNER_OUTPUT_LIMIT = 4096
            try:
                with self.assertRaisesRegex(ValueError, "output limit"):
                    module.run_scanner([os.sys.executable, "-c", "import sys;sys.stdout.write('x'*100000)"], os.environ.copy(), root / "out", root / "err", root / "sbom")
                with self.assertRaisesRegex(ValueError, "stderr tail") as caught:
                    module.run_scanner([os.sys.executable, "-c", "import sys;sys.stderr.write('end-marker');sys.exit(23)"], os.environ.copy(), root / "out2", root / "err2", root / "sbom2")
                self.assertIn("end-marker", str(caught.exception))
                self.assertLess(len(str(caught.exception)), 4200)
            finally:
                module.SCANNER_OUTPUT_LIMIT = previous

    def test_scanner_timeout_kills_sigterm_ignoring_process(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); previous = module.SCANNER_TIMEOUT_SECONDS; module.SCANNER_TIMEOUT_SECONDS = 0.15
            try:
                code = "import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(10)"
                with self.assertRaisesRegex(ValueError, "timed out"):
                    module.run_scanner([os.sys.executable, "-c", code], os.environ.copy(), root / "out", root / "err", root / "sbom")
            finally:
                module.SCANNER_TIMEOUT_SECONDS = previous

    @unittest.skipUnless(hasattr(os, "fork"), "requires POSIX process groups")
    def test_exited_parent_cannot_leave_sbom_writer_running(self):
        import time
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); output = root / "sbom"
            code = "import os,sys,time;pid=os.fork();(time.sleep(.4),open(sys.argv[1],'w').write('late')) if pid==0 else os._exit(0)"
            with self.assertRaisesRegex(ValueError, "child process"):
                module.run_scanner([os.sys.executable, "-c", code, str(output)], os.environ.copy(), root / "out", root / "err", output)
            size_after = output.stat().st_size if output.exists() else 0
            time.sleep(.5)
            self.assertEqual(output.stat().st_size if output.exists() else 0, size_after)

    def test_keyboard_interrupt_removes_fresh_output_and_preserves_existing(self):
        original = module._generate
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fresh = root / "fresh"
            def interrupt(*args, **kwargs):
                fresh.mkdir(); (fresh / "partial").write_text("partial"); raise KeyboardInterrupt
            module._generate = interrupt
            try:
                with self.assertRaises(KeyboardInterrupt): module.generate(root / "src", fresh, "commit")
                self.assertFalse(fresh.exists())
                existing = root / "existing"; existing.mkdir(); (existing / "keep").write_text("keep")
                with self.assertRaises(KeyboardInterrupt): module.generate(root / "src", existing, "commit")
                self.assertEqual((existing / "keep").read_text(), "keep")
            finally:
                module._generate = original

if __name__ == "__main__": unittest.main()
