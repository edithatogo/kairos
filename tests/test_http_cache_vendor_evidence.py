"""Focused reproducibility and schema checks for the private vendor evidence."""
from __future__ import annotations

import copy
import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from generate_http_cache_vendor_evidence import (
    EVIDENCE_MEMBERS,
    INPUT_ZIP,
    PACKAGE_TARBALL,
    ROOT,
    SBOM_PATH,
    PROVENANCE_PATH,
    build_evidence_zip,
    build_outputs,
    build_package_archive,
    derived_slsa_cue,
    json_bytes,
    read_evidence_members,
    read_package_payload,
    validate_normative_profile,
    cyclonedx_validator,
)


class VendorEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.outputs = build_outputs()
        cls.members = read_evidence_members((ROOT / INPUT_ZIP.relative_to(ROOT)).read_bytes())

    def test_four_file_npm_archive_recreates_committed_archive(self) -> None:
        archive = build_package_archive(read_package_payload())
        self.assertEqual(archive, PACKAGE_TARBALL.read_bytes())

    def test_evidence_zip_rebuild_is_byte_identical_and_pinned(self) -> None:
        rebuilt = build_evidence_zip(self.members)
        self.assertEqual(rebuilt, INPUT_ZIP.read_bytes())
        self.assertEqual(set(self.members), set(EVIDENCE_MEMBERS))

    def test_evidence_zip_rejects_missing_extra_and_tampered_inputs(self) -> None:
        missing = dict(self.members)
        missing.pop(next(iter(missing)))
        with self.assertRaisesRegex(ValueError, "exactly the 13"):
            build_evidence_zip(missing)
        extra = dict(self.members, **{"unexpected.txt": b"extra"})
        with self.assertRaisesRegex(ValueError, "exactly the 13"):
            build_evidence_zip(extra)
        changed = dict(self.members)
        name = "advisories/GHSA-ch52-4w7c-c8xp.json"
        changed[name] = changed[name][:-1] + bytes([changed[name][-1] ^ 1])
        with self.assertRaisesRegex(ValueError, "input hash mismatch"):
            build_evidence_zip(changed)

    def test_output_documents_are_repeatable(self) -> None:
        self.assertEqual(build_outputs(), self.outputs)
        self.assertEqual(self.outputs["sbom"], SBOM_PATH.read_bytes())
        self.assertEqual(self.outputs["provenance"], PROVENANCE_PATH.read_bytes())
        provenance = json.loads(self.outputs["provenance"])
        internal = provenance["predicate"]["buildDefinition"]["internalParameters"]
        self.assertEqual(internal["generatorSha256"], hashlib.sha256((ROOT / "tools/generate_http_cache_vendor_evidence.py").read_bytes()).hexdigest())
        self.assertEqual(internal["sbomSha256"], hashlib.sha256(self.outputs["sbom"]).hexdigest())
        metadata = provenance["predicate"]["runDetails"]["metadata"]
        self.assertNotIn("startedOn", metadata)
        self.assertNotIn("finishedOn", metadata)
        definition = provenance["predicate"]["buildDefinition"]
        self.assertEqual(definition["buildType"], "urn:careops:build-type:private-npm-vendor-evidence:v1")
        self.assertEqual(definition["externalParameters"]["buildTypeStatus"], "local-undocumented")
        self.assertIn("No external standard", definition["externalParameters"]["buildTypeMapping"])
        evidence = [item for item in definition["resolvedDependencies"] if item.get("name") == "frozen evidence source archive"]
        self.assertEqual(len(evidence), 1)
        self.assertEqual(evidence[0]["uri"], "file:vendor/http-cache-semantics-kairos-prototype/vendor-evidence-inputs.zip")
        self.assertEqual(evidence[0]["digest"]["sha256"], hashlib.sha256(INPUT_ZIP.read_bytes()).hexdigest())

    def test_cyclonedx_17_full_schema_and_negative_fixture(self) -> None:
        bom = json.loads(self.outputs["sbom"])
        validator = cyclonedx_validator(self.members)
        self.assertEqual(list(validator.iter_errors(bom)), [])
        invalid = copy.deepcopy(bom)
        invalid["metadata"]["component"]["type"] = "not-a-cyclonedx-component-type"
        self.assertTrue(list(validator.iter_errors(invalid)))

    def test_cyclonedx_preserves_private_fork_pedigree_and_advisory(self) -> None:
        bom = json.loads(self.outputs["sbom"])
        component = bom["metadata"]["component"]
        self.assertEqual(component["name"], "@careops/http-cache-semantics-kairos-prototype")
        self.assertEqual(component["version"], "0.1.0")
        pedigree = component["pedigree"]
        self.assertEqual(pedigree["ancestors"][0]["name"], "http-cache-semantics")
        self.assertEqual(pedigree["ancestors"][0]["version"], "4.2.0")
        self.assertEqual(len(pedigree["commits"]), 2)
        self.assertEqual(len(pedigree["patches"]), 2)
        props = {item["name"]: item["value"] for item in component["properties"]}
        self.assertEqual(props["careops:upstream-advisory"], "GHSA-ch52-4w7c-c8xp")
        self.assertEqual(props["careops:upstream-cve"], "CVE-2026-93748")
        self.assertEqual(props["careops:public-advisory-closure"], "not claimed")

    def test_slsa_normative_profile_accepts_provenance_and_rejects_required_field_gaps(self) -> None:
        statement = json.loads(self.outputs["provenance"])
        validate_normative_profile(statement)
        bad_values = []
        for path in [
            ("predicate", "buildDefinition"),
            ("predicate", "runDetails"),
        ]:
            item = copy.deepcopy(statement)
            item["predicate"].pop(path[1])
            bad_values.append(item)
        for field in ["buildType", "externalParameters"]:
            item = copy.deepcopy(statement)
            item["predicate"]["buildDefinition"].pop(field)
            bad_values.append(item)
        item = copy.deepcopy(statement)
        item["predicate"]["runDetails"]["builder"].pop("id")
        bad_values.append(item)
        item = copy.deepcopy(statement)
        item["subject"][0].pop("digest")
        bad_values.append(item)
        item = copy.deepcopy(statement)
        item["predicate"]["buildDefinition"]["resolvedDependencies"].append({"name": "empty"})
        bad_values.append(item)
        for group in [
            ("predicate", "runDetails", "builder", "builderDependencies"),
            ("predicate", "runDetails", "byproducts"),
        ]:
            item = copy.deepcopy(statement)
            target = item
            for key in group[:-1]:
                target = target[key]
            target[group[-1]] = [{"name": "empty"}]
            bad_values.append(item)
        for field, value in [("startedOn", "2026-02-30T01:02:03Z"), ("finishedOn", "2026-10-03T12:00:00")]:
            item = copy.deepcopy(statement)
            item["predicate"]["runDetails"]["metadata"][field] = value
            bad_values.append(item)
        item = copy.deepcopy(statement)
        metadata = item["predicate"]["runDetails"]["metadata"]
        metadata["startedOn"] = "2026-10-03T22:14:08.223574Z"
        metadata["finishedOn"] = "2026-10-03T22:14:08.442402+00:00"
        validate_normative_profile(item)
        metadata.pop("startedOn")
        metadata.pop("finishedOn")
        item["predicate"]["runDetails"]["builder"].pop("builderDependencies")
        item["predicate"]["runDetails"].pop("byproducts")
        item["predicate"]["buildDefinition"].pop("resolvedDependencies")
        validate_normative_profile(item)
        for invalid in bad_values:
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    validate_normative_profile(invalid)

    def test_resource_descriptor_content_uses_strict_canonical_base64(self) -> None:
        base = json.loads(self.outputs["provenance"])
        for value, accepted in [("", True), ("YQ==", True), ("!!!", False), ("Zg", False), ("Zh==", False)]:
            candidate = copy.deepcopy(base)
            candidate["predicate"]["buildDefinition"]["resolvedDependencies"] = [{"content": value}]
            if accepted:
                validate_normative_profile(candidate)
            else:
                with self.assertRaises(ValueError):
                    validate_normative_profile(candidate)

    def test_derived_slsa_cue_matches_frozen_json_compatibility_adapter(self) -> None:
        official = self.members["schemas/slsa/1.2/provenance.cue"]
        derived = derived_slsa_cue(official)
        self.assertEqual(hashlib.sha256(derived).hexdigest(), "4d4a43d17e5f80c238f4a8a6f22bc459b611eaf646a45235fa83d3b159a4aff2")

    def test_cue_v0171_validates_statement_and_rejects_negative_fixtures(self) -> None:
        cue = os.environ.get("CUE_BIN")
        self.assertTrue(cue, "CUE_BIN must point to the source-pinned CUE v0.17.1 executable")
        version = subprocess.run([cue, "version"], check=True, text=True, capture_output=True).stdout
        self.assertIn("v0.17.1", version)
        statement = json.loads(self.outputs["provenance"])
        schema = derived_slsa_cue(self.members["schemas/slsa/1.2/provenance.cue"])
        invalids = []
        for path, replacement in [
            (("_type",), "wrong"),
            (("predicateType",), "https://slsa.dev/provenance/v0.2"),
            (("predicate", "runDetails", "builder", "version", "generator"), 17),
            (("predicate", "runDetails", "builder", "version", "generator"), ["v"]),
            (("predicate", "runDetails", "builder", "version", "generator"), None),
            (("predicate", "buildDefinition", "externalParameters"), "not-an-object"),
            (("predicate", "buildDefinition", "internalParameters"), []),
            (("predicate", "buildDefinition", "resolvedDependencies", 0, "uri"), 17),
            (("predicate", "buildDefinition", "resolvedDependencies", 0, "digest", "sha256"), 17),
            (("predicate", "buildDefinition", "resolvedDependencies", 0, "content"), []),
            (("predicate", "buildDefinition", "resolvedDependencies", 0, "annotations"), []),
            (("predicate", "runDetails", "metadata", "startedOn"), 17),
        ]:
            candidate = copy.deepcopy(statement)
            target = candidate
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = replacement
            invalids.append(candidate)
        with tempfile.TemporaryDirectory(prefix="kairos-cue-vendor-evidence-") as tmp:
            temp = Path(tmp)
            schema_path = temp / "provenance.cue"
            good_path = temp / "good.json"
            schema_path.write_bytes(schema)
            good_path.write_bytes(json_bytes(statement))
            good = subprocess.run([cue, "vet", "-c", str(schema_path), str(good_path)], cwd=temp, text=True, capture_output=True)
            self.assertEqual(good.returncode, 0, good.stderr)
            minimal = copy.deepcopy(statement)
            for descriptor in minimal["predicate"]["buildDefinition"]["resolvedDependencies"] + minimal["predicate"]["runDetails"]["byproducts"]:
                descriptor.clear()
                descriptor.update({"uri": "https://example.invalid/resource", "digest": {"sha256": "0123456789abcdef"}})
            validate_normative_profile(minimal)
            minimal_path = temp / "minimal-descriptors.json"
            minimal_path.write_bytes(json_bytes(minimal))
            minimal_result = subprocess.run([cue, "vet", "-c", str(schema_path), str(minimal_path)], cwd=temp, text=True, capture_output=True)
            self.assertEqual(minimal_result.returncode, 0, minimal_result.stderr)
            for index, invalid in enumerate(invalids):
                data_path = temp / f"invalid-{index}.json"
                data_path.write_bytes(json_bytes(invalid))
                result = subprocess.run([cue, "vet", "-c", str(schema_path), str(data_path)], cwd=temp, text=True, capture_output=True)
                self.assertNotEqual(result.returncode, 0, f"CUE unexpectedly accepted negative fixture {index}")


if __name__ == "__main__":
    unittest.main()
