"""Offline negative tests for queue_simpy_v1; intentionally stdlib-only."""
import copy
import unittest

import queue_simpy_v1 as comparator


class ComparatorValidationTests(unittest.TestCase):
    def setUp(self):
        self.kairos = {
            "fixture": comparator.FIXTURE,
            "version": comparator.FIXTURE_VERSION,
            "engine": "kairos",
            "engine_version": comparator.KAIROS_VERSION,
            "events": copy.deepcopy(comparator.EXPECTED_EVENTS),
        }
        self.simpy = {
            "fixture": comparator.FIXTURE,
            "version": comparator.FIXTURE_VERSION,
            "engine": "simpy",
            "engine_version": comparator.SIMPY_VERSION,
            "events": copy.deepcopy(comparator.EXPECTED_EVENTS),
        }

    def test_hand_derived_rows_match(self):
        result = comparator.compare_traces(self.kairos, self.simpy)
        self.assertEqual(result["events"], comparator.EXPECTED_EVENTS)
        self.assertEqual(len(result["events_sha256"]), 64)

    def test_missing_event_fails(self):
        self.simpy["events"].pop()
        with self.assertRaisesRegex(ValueError, "hand-derived"):
            comparator.compare_traces(self.kairos, self.simpy)

    def test_altered_event_fails(self):
        self.simpy["events"][0]["at"] = 1
        with self.assertRaisesRegex(ValueError, "hand-derived"):
            comparator.compare_traces(self.kairos, self.simpy)

    def test_wrong_kairos_version_fails(self):
        self.kairos["engine_version"] = "0.1.1"
        with self.assertRaisesRegex(ValueError, "wrong engine or version"):
            comparator.compare_traces(self.kairos, self.simpy)

    def test_bool_is_not_fixture_integer_version(self):
        self.kairos["version"] = True
        with self.assertRaisesRegex(ValueError, "fixture identity/version"):
            comparator.compare_traces(self.kairos, self.simpy)

    def test_wrong_simpy_version_fails(self):
        self.simpy["engine_version"] = "4.1.1"
        with self.assertRaisesRegex(ValueError, "wrong engine or version"):
            comparator.compare_traces(self.kairos, self.simpy)

    def test_duplicate_event_fails(self):
        self.simpy["events"].insert(1, copy.deepcopy(self.simpy["events"][0]))
        with self.assertRaisesRegex(ValueError, "hand-derived"):
            comparator.compare_traces(self.kairos, self.simpy)

    def test_unknown_case_or_operation_fails(self):
        self.simpy["events"][0]["op"] = "preempt"
        with self.assertRaisesRegex(ValueError, "invalid types or operation"):
            comparator.compare_traces(self.kairos, self.simpy)


if __name__ == "__main__":
    unittest.main()
