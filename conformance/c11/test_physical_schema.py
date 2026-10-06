"""Tests for the proposed C1 Arrow schema and logical/physical codecs."""
import unittest

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as pq

from physical_schema import (
    I128_MAX, I128_MIN, TRACE_EVENT_SCHEMA, TRACE_EXCLUSION_SCHEMA, OUTCOME_SCHEMA,
    U128_MAX, _bytes, _utc_nanos, decode_row, encode_row, table_from_rows,
)


def lineage():
    return {"status": "observed", "mapping_version": "map-1", "evidence_ref": None, "derivation": None}


def time_value(ticks="0"):
    return {"raw": "2024-01-01T00:00:00Z", "utc": "2024-01-01T00:00:00Z",
            "relative_ticks": ticks, "tick_resolution": "1ns", "source_precision": "second", "source_offset_or_zone": "Z",
            "lineage": lineage()}


def trace_event():
    return {"record_type": "trace_event.v1", "schema_version": "calibration-v1", "dataset_id": "d1",
            "mapping_version": "map-1", "case_key": "c1", "source_event_key": "s1", "occurrence": 0,
            "event_kind": "arrive", "relative_ticks": str(U128_MAX), "source_order": 2**64 - 1,
            "event_kind_rank": 10**100, "occurrence_time": time_value(str(U128_MAX)),
            "source_recorded_time": None, "message_created_time": None,
            "time_lineage": {"occurrence": lineage(), "source_recorded": lineage(), "message_created": lineage()},
            "resource_key": None, "actor_key": "a1", "location_key": None, "quality_flags": [],
            "raw_event": {"source_family": "synthetic", "source_event_type": "arrive",
                          "source_record_id": None, "source_fields": {"z": [2, 1], "a": True}},
            "disposition": "realized", "knowledge_availability": {"status": "known", "available_at": None}}


class PhysicalSchemaTests(unittest.TestCase):
    def test_raw_time_map_has_exact_parquet_and_ipc_schema(self):
        row = {"record_type": "trace_exclusion.v1", "schema_version": "calibration-v1",
               "dataset_id": "d1", "mapping_version": "map-1", "source_event_key": "s2",
               "raw_event": {"source_family": "synthetic", "source_event_type": "bad_time"},
               "raw_time_values": {"occurred": None, "message": "unchanged raw"},
               "exclusion_reason": "missing_timezone", "lineage": lineage()}
        table = table_from_rows("trace_exclusion.v1", [row])
        sink = pa.BufferOutputStream()
        pq.write_table(table, sink, compression="NONE", use_compliant_nested_type=True)
        restored = pq.read_table(pa.BufferReader(sink.getvalue()))
        self.assertTrue(restored.schema.equals(TRACE_EXCLUSION_SCHEMA, check_metadata=True))
        self.assertEqual(decode_row(restored.to_pylist()[0], "trace_exclusion.v1"), row)
        for factory, reader in ((ipc.new_file, ipc.open_file), (ipc.new_stream, ipc.open_stream)):
            sink = pa.BufferOutputStream()
            with factory(sink, table.schema) as writer:
                writer.write_table(table)
            restored = reader(pa.BufferReader(sink.getvalue())).read_all()
            self.assertTrue(restored.schema.equals(TRACE_EXCLUSION_SCHEMA, check_metadata=True))
            self.assertEqual(decode_row(restored.to_pylist()[0], "trace_exclusion.v1"), row)

    def test_available_at_lineage_status_has_its_own_enum(self):
        for status in ("known", "not_yet_known", "unknown"):
            for clock_status in ("observed", "derived", "unknown"):
                row = trace_event()
                clock = time_value()
                clock["lineage"]["status"] = clock_status
                row["knowledge_availability"] = {"status": status, "available_at": clock}
                self.assertEqual(decode_row(encode_row(row), "trace_event.v1"), row)

    def test_availability_and_lineage_status_domains_cannot_cross(self):
        row = trace_event()
        row["knowledge_availability"]["status"] = "observed"
        with self.assertRaises(ValueError):
            encode_row(row)
        row = trace_event()
        row["knowledge_availability"]["available_at"] = time_value()
        row["knowledge_availability"]["available_at"]["lineage"]["status"] = "known"
        with self.assertRaises(ValueError):
            encode_row(row)

    def test_exact_table_metadata_and_clock_types(self):
        for schema, record_type in ((TRACE_EVENT_SCHEMA, "trace_event.v1"),
                                    (TRACE_EXCLUSION_SCHEMA, "trace_exclusion.v1"),
                                    (OUTCOME_SCHEMA, "outcome_observation.v1")):
            self.assertEqual(schema.metadata[b"format"], b"careops.calibration.physical")
            self.assertEqual(schema.metadata[b"physical_version"], b"2")
            self.assertEqual(schema.metadata[b"record_type"], record_type.encode())
            self.assertEqual(schema.field("relative_ticks").type, pa.binary(16)) if record_type == "trace_event.v1" else None
        self.assertEqual(TRACE_EVENT_SCHEMA.field("source_order").type, pa.uint64())
        self.assertEqual(TRACE_EVENT_SCHEMA.field("occurrence").type, pa.uint32())
        self.assertEqual(TRACE_EVENT_SCHEMA.field("event_kind_rank").type, pa.string())
        self.assertEqual(TRACE_EVENT_SCHEMA.field("presence_fields").type, pa.list_(pa.field("element", pa.string(), nullable=False)))

    def test_full_width_ticks_and_arbitrary_rank_round_trip(self):
        row = trace_event()
        physical = encode_row(row)
        self.assertEqual(physical["relative_ticks"], U128_MAX.to_bytes(16, "little"))
        self.assertEqual(physical["occurrence_time"]["relative_ticks"], U128_MAX.to_bytes(16, "little"))
        self.assertEqual(physical["event_kind_rank"], str(10**100))
        self.assertEqual(decode_row(physical, "trace_event.v1"), row)
        row["event_kind_rank"] = 10**5000
        self.assertEqual(decode_row(encode_row(row), "trace_event.v1"), row)

    def test_presence_distinguishes_absent_from_null_and_empty(self):
        row = trace_event()
        del row["resource_key"]
        row["location_key"] = None
        del row["quality_flags"]
        row["event_kind_rank"] = 0
        del row["occurrence_time"]["source_offset_or_zone"]
        physical = encode_row(row)
        self.assertIn("resource_key", physical["presence_fields"])
        self.assertIn("quality_flags", physical["presence_fields"])
        self.assertIn("occurrence_time.source_offset_or_zone", physical["presence_fields"])
        decoded = decode_row(physical, "trace_event.v1")
        self.assertNotIn("resource_key", decoded)
        self.assertIsNone(decoded["location_key"])
        self.assertNotIn("quality_flags", decoded)
        self.assertEqual(decoded["event_kind_rank"], 0)
        self.assertNotIn("source_offset_or_zone", decoded["occurrence_time"])

    def test_utc_i128_parity_and_rejection(self):
        row = trace_event()
        row["occurrence_time"]["utc"] = "1969-12-31T23:59:59.999999Z"
        encoded = encode_row(row)
        self.assertEqual(int.from_bytes(encoded["occurrence_time"]["utc_i128_le"], "little", signed=True), -1000)
        encoded["occurrence_time"]["utc_i128_le"] = I128_MIN.to_bytes(16, "little", signed=True)
        with self.assertRaisesRegex(ValueError, "disagree"):
            decode_row(encoded, "trace_event.v1")
        row["occurrence_time"]["utc"] = "2024-01-01T00:00:00"
        with self.assertRaisesRegex(ValueError, "RFC3339"):
            encode_row(row)
        with self.assertRaisesRegex(ValueError, "unknown -00:00"):
            _utc_nanos("2024-01-01T00:00:00-00:00")
        self.assertEqual(_utc_nanos("1969-12-31T23:59:59.999999999000Z"), -1)
        self.assertEqual(_bytes(I128_MIN, True), I128_MIN.to_bytes(16, "little", signed=True))
        self.assertEqual(_bytes(I128_MAX, True), I128_MAX.to_bytes(16, "little", signed=True))

    def test_invalid_clock_and_presence_encodings_reject(self):
        row = trace_event()
        row["relative_ticks"] = str(U128_MAX + 1)
        with self.assertRaises(ValueError):
            encode_row(row)
        physical = encode_row(trace_event())
        physical["presence_fields"] = ["not_a_field"]
        with self.assertRaisesRegex(ValueError, "unknown"):
            decode_row(physical, "trace_event.v1")
        physical = encode_row(trace_event())
        physical["presence_fields"] = ["record_type"]
        with self.assertRaises(ValueError):
            decode_row(physical, "trace_event.v1")
        physical = encode_row(trace_event())
        physical["raw_event"]["source_family"] = None
        physical["presence_fields"] = ["raw_event.source_family"]
        with self.assertRaises(ValueError):
            decode_row(physical, "trace_event.v1")

    def test_source_fields_are_canonical_json_only(self):
        physical = encode_row(trace_event())
        self.assertEqual(physical["raw_event"]["source_fields_json"], '{"a":true,"z":[2,1]}')
        self.assertEqual(decode_row(physical, "trace_event.v1")["raw_event"]["source_fields"],
                         {"a": True, "z": [2, 1]})
        physical["raw_event"]["source_fields_json"] = '{ "a": true, "z": [2, 1] }'
        with self.assertRaisesRegex(ValueError, "canonical"):
            decode_row(physical, "trace_event.v1")

    def test_fixed_schema_table_construction(self):
        table = table_from_rows("trace_event.v1", [trace_event()])
        self.assertEqual(table.schema, TRACE_EVENT_SCHEMA)
        self.assertEqual(table.num_rows, 1)

    def test_exclusion_and_outcome_tables_keep_all_declared_fields(self):
        exclusion = {"record_type": "trace_exclusion.v1", "schema_version": "calibration-v1",
                     "dataset_id": "d1", "mapping_version": "map-1", "source_event_key": "s2",
                     "raw_event": {"source_family": "synthetic", "source_event_type": "bad_time"},
                     "raw_time_values": {"occurred": None}, "exclusion_reason": "missing_timezone",
                     "lineage": lineage(), "detail": None}
        outcome = {"record_type": "outcome_observation.v1", "schema_version": "calibration-v1",
                   "dataset_id": "d1", "case_key": "c1", "endpoint": "death", "risk_start": None,
                   "last_observed": time_value(), "event_observed": False, "event_time": None, "event_cause": None,
                   "censor_status": "right", "censor_reason": "end_of_followup", "cluster_ids": [],
                   "lineage": lineage()}
        for row in (exclusion, outcome):
            physical = encode_row(row)
            self.assertEqual(decode_row(physical, row["record_type"]), row)
        self.assertEqual(TRACE_EXCLUSION_SCHEMA.field("raw_time_values").type, pa.list_(pa.field("element", pa.struct([pa.field("key", pa.string(), nullable=False), pa.field("value", pa.string())]), nullable=False)))

    def test_rejects_required_null_unknown_and_hidden_presence(self):
        row = trace_event()
        row["dataset_id"] = None
        with self.assertRaises(ValueError):
            encode_row(row)
        row = trace_event()
        row["extra"] = 1
        with self.assertRaises(ValueError):
            encode_row(row)
        row = trace_event()
        row["quality_flags"] = [None]
        with self.assertRaises(ValueError):
            encode_row(row)
        physical = encode_row(trace_event())
        physical["presence_fields"] = ["source_recorded_time.lineage.evidence_ref"]
        with self.assertRaises(ValueError):
            decode_row(physical, "trace_event.v1")

    def test_map_duplicate_keys_reject(self):
        row = {"record_type": "trace_exclusion.v1", "schema_version": "calibration-v1",
               "dataset_id": "d1", "mapping_version": "map-1", "source_event_key": "s2",
               "raw_event": {"source_family": "synthetic", "source_event_type": "bad_time"},
               "raw_time_values": {"occurred": None}, "exclusion_reason": "missing_timezone",
               "lineage": lineage()}
        physical = encode_row(row)
        physical["raw_time_values"] = [{"key": "occurred", "value": None}, {"key": "occurred", "value": "duplicate"}]
        with self.assertRaisesRegex(ValueError, "sorted and unique"):
            decode_row(physical, "trace_exclusion.v1")


if __name__ == "__main__":
    unittest.main()
