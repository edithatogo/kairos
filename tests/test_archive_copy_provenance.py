"""Regression tests for the stdlib unsigned archive-copy provenance validator."""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "packaging" / "scripts"))

import validate_archive_copy_provenance as validator


RUN_ID = 37273717088
SOURCE_COMMIT = "ddfb65da90dc041cea20fa73bfb59d3560746eaf"
ARTIFACT_ID = 11329331832
INDEX = {
    "source_commit": SOURCE_COMMIT,
    "artifacts": [
        {"path": "python/sample.whl", "sha256": "1" * 64, "builder": {"source_commit": SOURCE_COMMIT}},
        {"path": "rust/sample.crate", "sha256": "2" * 64, "builder": {"source_commit": SOURCE_COMMIT}},
    ]
}
INDEX_BYTES = json.dumps(INDEX, sort_keys=True, separators=(",", ":")).encode()
INDEX_SHA256 = hashlib.sha256(INDEX_BYTES).hexdigest()
EXPECTED = {
    "archive_index_sha256": INDEX_SHA256,
    "source_commit": SOURCE_COMMIT,
    "original_run_id": RUN_ID,
    "acquisition_artifact_id": ARTIFACT_ID,
    "dependencies": [
        {"id": f"https://github.com/edithatogo/kairos/actions/runs/{RUN_ID}", "sha256": "a" * 64},
        {"id": "ARCHIVE-INDEX.json", "sha256": INDEX_SHA256},
        {"id": "build-inputs/ARCHIVE-INDEX.json", "sha256": INDEX_SHA256},
        {"id": "packaging/scripts/build_archive_supply_chain.py", "sha256": "b" * 64},
    ],
}


def make_statement() -> dict:
    deps = []
    for item in EXPECTED["dependencies"]:
        uri = validator.canonical_dependency_uri(item["id"], RUN_ID)
        deps.append({"name": item["id"], "uri": uri, "digest": {"sha256": item["sha256"]}})
    return {
        "_type": validator.STATEMENT_TYPE,
        "subject": [
            {"name": "archives/" + item["path"], "digest": {"sha256": item["sha256"]}}
            for item in INDEX["artifacts"]
        ],
        "predicateType": validator.PREDICATE_TYPE,
        "predicate": {
            "buildDefinition": {
                "buildType": validator.BUILD_TYPE,
                "externalParameters": {"source_commit": SOURCE_COMMIT, "original_run_id": RUN_ID},
                "internalParameters": {"acquisition_artifact_id": ARTIFACT_ID, "runtime": {"python": "3.14.8"}},
                "resolvedDependencies": deps,
            },
            "runDetails": {
                "builder": {"id": validator.BUILDER_ID},
                "metadata": {"startedOn": "2026-10-05T08:44:48.514073Z", "finishedOn": "2026-10-05T08:44:53.608681Z"},
            },
        },
    }


def codes(statement=None, index=INDEX, expected=EXPECTED, index_sha=INDEX_SHA256):
    return {issue.code for issue in validator.validate_provenance(
        make_statement() if statement is None else statement, index, expected, index_sha
    )}


class ArchiveCopyProvenanceTests(unittest.TestCase):
    def test_valid_bound_fixture_passes(self):
        self.assertEqual([], validator.validate_provenance(make_statement(), INDEX, EXPECTED, INDEX_SHA256))

    def test_subject_count_is_derived_from_index(self):
        index = copy.deepcopy(INDEX)
        index["artifacts"].append({"path": "go/sample.tgz", "sha256": "3" * 64, "builder": {"source_commit": SOURCE_COMMIT}})
        index_sha = hashlib.sha256(json.dumps(index, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        expected = copy.deepcopy(EXPECTED)
        for dep in expected["dependencies"]:
            if dep["id"] in ("ARCHIVE-INDEX.json", "build-inputs/ARCHIVE-INDEX.json"):
                dep["sha256"] = index_sha
        statement = make_statement()
        statement["subject"].append({"name": "archives/go/sample.tgz", "digest": {"sha256": "3" * 64}})
        for dep in statement["predicate"]["buildDefinition"]["resolvedDependencies"]:
            if dep["uri"] in {
                validator.canonical_dependency_uri("ARCHIVE-INDEX.json", RUN_ID),
                validator.canonical_dependency_uri("build-inputs/ARCHIVE-INDEX.json", RUN_ID),
            }:
                dep["digest"]["sha256"] = index_sha
        self.assertEqual([], validator.validate_provenance(statement, index, expected | {"archive_index_sha256": index_sha}, index_sha))

    def test_subject_cardinality_one_is_supported(self):
        index = {"source_commit": SOURCE_COMMIT, "artifacts": INDEX["artifacts"][:1]}
        index_sha = hashlib.sha256(json.dumps(index, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        expected = copy.deepcopy(EXPECTED)
        expected["archive_index_sha256"] = index_sha
        expected["dependencies"] = [
            {"id": dep["id"], "sha256": index_sha if dep["id"] in ("ARCHIVE-INDEX.json", "build-inputs/ARCHIVE-INDEX.json") else dep["sha256"]}
            for dep in expected["dependencies"]
        ]
        statement = make_statement()
        statement["subject"] = statement["subject"][:1]
        for dep in statement["predicate"]["buildDefinition"]["resolvedDependencies"]:
            if dep["uri"] in {validator.canonical_dependency_uri("ARCHIVE-INDEX.json", RUN_ID), validator.canonical_dependency_uri("build-inputs/ARCHIVE-INDEX.json", RUN_ID)}:
                dep["digest"]["sha256"] = index_sha
        self.assertEqual([], validator.validate_provenance(statement, index, expected, index_sha))

    def test_subject_cardinality_eight_is_supported_without_fixed_count(self):
        index = {"source_commit": SOURCE_COMMIT, "artifacts": [
            {"path": f"pkg/{number}.tgz", "sha256": f"{number:x}" * 64, "builder": {"source_commit": SOURCE_COMMIT}}
            for number in range(1, 9)
        ]}
        index_sha = hashlib.sha256(json.dumps(index, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        expected = copy.deepcopy(EXPECTED)
        expected["archive_index_sha256"] = index_sha
        expected["dependencies"] = [
            {"id": dep["id"], "sha256": index_sha if dep["id"] in ("ARCHIVE-INDEX.json", "build-inputs/ARCHIVE-INDEX.json") else dep["sha256"]}
            for dep in expected["dependencies"]
        ]
        statement = make_statement()
        statement["subject"] = [
            {"name": "archives/" + item["path"], "digest": {"sha256": item["sha256"]}}
            for item in index["artifacts"]
        ]
        for dep in statement["predicate"]["buildDefinition"]["resolvedDependencies"]:
            if dep["uri"] in {validator.canonical_dependency_uri("ARCHIVE-INDEX.json", RUN_ID), validator.canonical_dependency_uri("build-inputs/ARCHIVE-INDEX.json", RUN_ID)}:
                dep["digest"]["sha256"] = index_sha
        self.assertEqual([], validator.validate_provenance(statement, index, expected, index_sha))

    def test_canonical_local_uri_mapping_preserves_identifier(self):
        self.assertEqual(
            "urn:careops:archive-copy:v1:run:37273717088:dependency:packaging%2Fscripts%2Fvalidate.py",
            validator.canonical_dependency_uri("packaging/scripts/validate.py", RUN_ID),
        )
        self.assertEqual(
            "https://example.test/run/4",
            validator.canonical_dependency_uri("https://example.test/run/4", RUN_ID),
        )
        self.assertNotEqual(
            validator.canonical_dependency_uri("ARCHIVE-INDEX.json", RUN_ID),
            validator.canonical_dependency_uri("build-inputs/ARCHIVE-INDEX.json", RUN_ID),
        )

    def test_wrong_statement_type_is_rejected(self):
        value = make_statement(); value["_type"] = "other"
        self.assertIn("statement_type_id", codes(value))

    def test_wrong_predicate_type_is_rejected(self):
        value = make_statement(); value["predicateType"] = "other"
        self.assertIn("predicate_type", codes(value))

    def test_empty_subject_array_is_rejected(self):
        value = make_statement(); value["subject"] = []
        self.assertIn("subject_array", codes(value))

    def test_missing_subject_digest_is_rejected(self):
        value = make_statement(); del value["subject"][0]["digest"]
        self.assertIn("subject_digest", codes(value))

    def test_malformed_subject_digest_is_rejected(self):
        value = make_statement(); value["subject"][0]["digest"]["sha256"] = "xyz"
        self.assertIn("subject_digest", codes(value))

    def test_wrong_subject_digest_is_rejected(self):
        value = make_statement(); value["subject"][0]["digest"]["sha256"] = "f" * 64
        self.assertIn("subject_digest_mismatch", codes(value))

    def test_missing_index_subject_is_rejected(self):
        value = make_statement(); value["subject"].pop()
        self.assertIn("subject_set", codes(value))

    def test_duplicate_subject_name_is_rejected(self):
        value = make_statement(); value["subject"][1]["name"] = value["subject"][0]["name"]
        self.assertIn("subject_duplicate_name", codes(value))

    def test_wrong_build_type_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["buildType"] = "urn:other"
        self.assertIn("build_type", codes(value))

    def test_unexpected_external_parameter_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["externalParameters"]["extra"] = True
        self.assertIn("external_parameters", codes(value))

    def test_wrong_source_commit_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["externalParameters"]["source_commit"] = "0" * 40
        self.assertIn("external_parameters", codes(value))

    def test_wrong_builder_id_is_rejected(self):
        value = make_statement(); value["predicate"]["runDetails"]["builder"]["id"] = "urn:trusted:builder"
        self.assertIn("builder_id", codes(value))

    def test_missing_builder_id_is_rejected(self):
        value = make_statement(); value["predicate"]["runDetails"]["builder"] = {}
        self.assertIn("builder_id", codes(value))

    def test_missing_expected_dependency_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["resolvedDependencies"].pop()
        self.assertIn("dependency_set", codes(value))

    def test_duplicate_dependency_uri_is_rejected(self):
        value = make_statement(); dep = copy.deepcopy(value["predicate"]["buildDefinition"]["resolvedDependencies"][0]); value["predicate"]["buildDefinition"]["resolvedDependencies"].append(dep)
        self.assertIn("dependency_duplicate_uri", codes(value))

    def test_relative_dependency_identifier_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["resolvedDependencies"][1]["uri"] = "ARCHIVE-INDEX.json"
        self.assertIn("dependency_uri", codes(value))

    def test_bad_dependency_digest_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["resolvedDependencies"][0]["digest"]["sha256"] = "z" * 64
        self.assertIn("dependency_digest", codes(value))

    def test_substituted_dependency_digest_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["resolvedDependencies"][0]["digest"]["sha256"] = "c" * 64
        self.assertIn("dependency_digest_mismatch", codes(value))

    def test_dependency_name_must_preserve_original_identifier(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["resolvedDependencies"][1]["name"] = "different"
        self.assertIn("dependency_name_mismatch", codes(value))

    def test_dependency_name_is_required(self):
        value = make_statement(); del value["predicate"]["buildDefinition"]["resolvedDependencies"][1]["name"]
        self.assertIn("dependency_name", codes(value))

    def test_offset_timestamp_is_rejected(self):
        value = make_statement(); value["predicate"]["runDetails"]["metadata"]["startedOn"] = "2026-10-05T08:44:48.514073+00:00"
        self.assertIn("timestamp_utc_z", codes(value))

    def test_malformed_timestamp_is_rejected(self):
        value = make_statement(); value["predicate"]["runDetails"]["metadata"]["finishedOn"] = "yesterday"
        self.assertIn("timestamp_utc_z", codes(value))

    def test_reversed_timestamps_are_rejected(self):
        value = make_statement(); meta = value["predicate"]["runDetails"]["metadata"]; meta["startedOn"], meta["finishedOn"] = meta["finishedOn"], meta["startedOn"]
        self.assertIn("timestamp_order", codes(value))

    def test_unknown_top_level_extension_is_ignored(self):
        value = make_statement(); value["futureExtension"] = {"note": "allowed"}
        self.assertEqual([], validator.validate_provenance(value, INDEX, EXPECTED, INDEX_SHA256))

    def test_index_byte_hash_must_match_independent_pin(self):
        self.assertIn("archive_index_hash_mismatch", codes(index_sha="0" * 64))

    def test_expected_index_dependencies_must_bind_index_bytes(self):
        expected = copy.deepcopy(EXPECTED)
        expected["dependencies"][1]["sha256"] = "f" * 64
        self.assertIn("expected_index_dependency", codes(expected=expected))

    def test_boolean_run_id_is_not_accepted_as_integer(self):
        expected = copy.deepcopy(EXPECTED); expected["original_run_id"] = True
        self.assertIn("expected_integer", codes(expected=expected))

    def test_duplicate_json_keys_are_rejected(self):
        with self.assertRaises(validator.InputError):
            validator.load_json_bytes(b'{"a":1,"a":2}', "duplicate-key fixture")

    def test_document_size_limit_is_enforced(self):
        with self.assertRaises(validator.InputError):
            validator.load_json_bytes(b'{"long":"value"}', "size fixture", max_bytes=4)

    def test_file_reader_stops_at_size_limit(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "large.json"
            path.write_bytes(b" " * 20)
            with self.assertRaises(validator.InputError):
                validator.load_json_file(path, "large fixture", max_bytes=4)

    def test_file_reader_rejects_nonregular_files(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(validator.InputError):
                validator.load_json_file(Path(folder), "directory fixture")

    def test_non_json_nan_constant_is_rejected(self):
        for constant in (b"NaN", b"Infinity", b"-Infinity"):
            with self.subTest(constant=constant):
                with self.assertRaises(validator.InputError):
                    validator.load_json_bytes(b'{"x":' + constant + b"}", "non-finite fixture")

    def test_deeply_nested_json_is_reported_as_input_error(self):
        nested = b"[" * (validator.MAX_JSON_DEPTH + 1) + b"0" + b"]" * (validator.MAX_JSON_DEPTH + 1)
        with self.assertRaises(validator.InputError):
            validator.load_json_bytes(nested, "nested fixture")

    def test_json_at_nesting_limit_is_accepted(self):
        depth = validator.MAX_JSON_DEPTH
        nested = b"[" * depth + b"0" + b"]" * depth
        self.assertIsNotNone(validator.load_json_bytes(nested, "depth boundary fixture"))

    def test_decoder_recursion_error_is_reported_as_input_error(self):
        with mock.patch.object(validator.json, "loads", side_effect=RecursionError("decoder stack")):
            with self.assertRaises(validator.InputError):
                validator.load_json_bytes(b"{}", "mock recursion fixture")

    def test_uri_uppercase_scheme_is_rejected(self):
        self.assertFalse(validator.is_resource_uri("HTTPS://example.test/a"))

    def test_uri_forbidden_backslash_is_rejected(self):
        self.assertFalse(validator.is_resource_uri("https://example.test/a\\b"))

    def test_index_source_commit_must_match_external_pin(self):
        index = copy.deepcopy(INDEX); index["source_commit"] = "0" * 40
        self.assertIn("archive_index_source_commit", codes(index=index))

    def test_each_index_artifact_source_commit_must_match_external_pin(self):
        index = copy.deepcopy(INDEX); index["artifacts"][0]["builder"]["source_commit"] = "0" * 40
        self.assertIn("archive_index_artifact_source_commit", codes(index=index))

    def test_float_run_id_does_not_equal_expected_integer(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["externalParameters"]["original_run_id"] = float(RUN_ID)
        self.assertIn("external_parameters", codes(value))

    def test_boolean_run_id_does_not_equal_expected_integer(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["externalParameters"]["original_run_id"] = True
        self.assertIn("external_parameters", codes(value))

    def test_float_acquisition_artifact_id_is_rejected(self):
        value = make_statement(); value["predicate"]["buildDefinition"]["internalParameters"]["acquisition_artifact_id"] = float(ARTIFACT_ID)
        self.assertIn("acquisition_artifact_id", codes(value))

    def test_timestamp_fraction_precision_is_compared_without_truncation(self):
        value = make_statement(); meta = value["predicate"]["runDetails"]["metadata"]
        meta["startedOn"] = "2026-10-05T08:44:48.0000009Z"
        meta["finishedOn"] = "2026-10-05T08:44:48.0000008Z"
        self.assertIn("timestamp_order", codes(value))

    def test_old_actual_output_has_separate_timestamp_and_uri_failures(self):
        fixture = ROOT / "tests/fixtures/archive-supply-chain/legacy-actual-provenance"
        oracle = fixture / "provenance.json"
        index_path = fixture / "archive-index.json"
        expected_path = fixture / "expected-inputs.json"
        retained_hashes = {
            oracle: "88423a5c84bdfcfce9c00c0ccdd908686445c3832d968e9fadb250f2f8283c2a",
            index_path: "ff82daf20fe3161040fe87dcfdc1b7607dc3d65e3a176d3713945896c86fce11",
            expected_path: "bb39f3e431101c2a8b68c830ccf9cc251dcc558f9f494d402c20776978ccf510",
        }
        for path, expected_sha256 in retained_hashes.items():
            self.assertTrue(path.is_file(), f"missing checked-in actual-output oracle: {path}")
            self.assertEqual(expected_sha256, hashlib.sha256(path.read_bytes()).hexdigest())
        actual = json.loads(oracle.read_bytes())
        index_bytes = index_path.read_bytes()
        index = json.loads(index_bytes)
        expected = json.loads(expected_path.read_bytes())
        index_sha = hashlib.sha256(index_bytes).hexdigest()
        result = {issue.code for issue in validator.validate_provenance(actual, index, expected, index_sha)}
        self.assertIn("timestamp_utc_z", result)
        self.assertIn("dependency_uri", result)
        self.assertIn("dependency_name", result)
        self.assertIn("dependency_set", result)

        time_only = copy.deepcopy(actual)
        time_build = time_only["predicate"]["buildDefinition"]
        time_build["resolvedDependencies"] = [
            {"name": item["id"], "uri": validator.canonical_dependency_uri(item["id"], expected["original_run_id"]),
             "digest": {"sha256": item["sha256"]}}
            for item in expected["dependencies"]
        ]
        time_codes = {issue.code for issue in validator.validate_provenance(time_only, index, expected, index_sha)}
        self.assertIn("timestamp_utc_z", time_codes)
        self.assertNotIn("dependency_uri", time_codes)

        uri_only = copy.deepcopy(actual)
        uri_metadata = uri_only["predicate"]["runDetails"]["metadata"]
        for field in ("startedOn", "finishedOn"):
            uri_metadata[field] = uri_metadata[field].replace("+00:00", "Z")
        uri_codes = {issue.code for issue in validator.validate_provenance(uri_only, index, expected, index_sha)}
        self.assertNotIn("timestamp_utc_z", uri_codes)
        self.assertIn("dependency_uri", uri_codes)

        normalized = copy.deepcopy(actual)
        normalized["predicate"]["buildDefinition"]["resolvedDependencies"] = time_build["resolvedDependencies"]
        normalized_metadata = normalized["predicate"]["runDetails"]["metadata"]
        for field in ("startedOn", "finishedOn"):
            normalized_metadata[field] = normalized_metadata[field].replace("+00:00", "Z")
        self.assertEqual([], validator.validate_provenance(normalized, index, expected, index_sha))


if __name__ == "__main__":
    unittest.main()
