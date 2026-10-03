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
        "artifact_id": "fixture-artifact-1",
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
            release_manifest = json.loads((output / "release-artifact-manifest.json").read_text())
            expected_subjects = {(row["path"], row["sha256"]) for row in release_manifest["artifacts"]}
            actual_subjects = {(row["name"], row["digest"]["sha256"]) for row in provenance["subject"]}
            self.assertEqual(actual_subjects, expected_subjects)
            dependencies = provenance["predicate"]["buildDefinition"]["resolvedDependencies"]
            dependency_uris = {row["uri"] for row in dependencies}
            self.assertTrue({"build-inputs/ARCHIVE-INDEX.json", "build-inputs/BUILD-RECEIPT.json", "build-inputs/acquisition.json", "packaging/scripts/build_archive_supply_chain.py", "packaging/scripts/build_archive_release_manifest.py", "packaging/scripts/build_package_archive_bundle.py", "tool:syft", "schema:spdx-2.3"}.issubset(dependency_uris))
            self.assertTrue(any(f"/actions/runs/{RUN_ID}" in uri for uri in dependency_uris))
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
            real_run = module.subprocess.run
            changed = False
            def scan_then_mutate(command, **kwargs):
                nonlocal changed
                result = real_run(command, **kwargs)
                if not changed:
                    source_name = command[command.index("--source-name") + 1]
                    copied = output / "archives" / source_name
                    raw = copied.read_bytes()
                    copied.write_bytes(bytes([raw[0] ^ 1]) + raw[1:])
                    changed = True
                return result
            with patch.object(module.subprocess, "run", side_effect=scan_then_mutate):
                with self.assertRaises(ValueError): generate(fixture, output)
            self.assertTrue(changed)
            self.assertFalse(output.exists())


if __name__ == "__main__": unittest.main()
