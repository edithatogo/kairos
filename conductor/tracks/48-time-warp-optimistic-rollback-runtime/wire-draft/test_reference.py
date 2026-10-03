"""Behavioral checks for a draft oracle; no native/transport acceptance claim."""
import copy
import json
from pathlib import Path
import unittest
from reference import Ledger, envelope, order, validate_floor, fossil_ticks, U64

FIXTURES = json.loads(Path(__file__).with_name("fixtures.json").read_text())


class WireDraftTests(unittest.TestCase):
    def test_declared_delivery_vectors(self):
        for case in FIXTURES["delivery_cases"]:
            with self.subTest(case=case["name"]):
                ledger = Ledger()
                for name, expected in zip(case["steps"], case["outcomes"], strict=True):
                    before = copy.deepcopy(vars(ledger))
                    try:
                        actual = ledger.receive(FIXTURES["messages"][name])
                    except ValueError:
                        actual = "rejected"
                        self.assertEqual(vars(ledger), before)
                    self.assertEqual(actual, expected)
                self.assertEqual(ledger.pending, {envelope(FIXTURES["messages"][n])[0] for n in case["pending"]})

    def test_full_parent_key_and_actual_emitter_order(self):
        messages = FIXTURES["messages"]
        self.assertLess(order(messages["child10"]), order(messages["child20"]))
        self.assertEqual(envelope(messages["child10"])[0][0], 1)
        self.assertEqual(order(messages["old"]), order(messages["new_epoch"]))
        altered = copy.deepcopy(messages["old"])
        altered["incarnation"] = "2"
        self.assertEqual(order(messages["old"]), order(altered))
        root = copy.deepcopy(messages["child10"])
        root["logical_id"] = {"kind": "root", "source_lp": 1, "sequence": "999"}
        self.assertLess(order(root), order(messages["child10"]))

    def test_u64_and_payload_are_lossless(self):
        key, metadata = envelope(FIXTURES["messages"]["large_u64"])
        self.assertEqual((key[2][2], key[3], metadata[1]), (U64, U64, U64))
        self.assertEqual(metadata[2], b"\x00\xff\x80")

    def test_conflicting_metadata_and_malformed_reject_unchanged(self):
        changes = [{"dest_lp": 2}, {"payload_hex": "ff"}, {"tick": "11"}, {"source_lp": True}, {"dest_lp": -1}, {"dest_lp": 1 << 32}, {"tick": "01"}, {"tick": 10}, {"tick": str(U64 + 1)}, {"incarnation": "0"}, {"payload_hex": "f"}, {"payload_hex": "GG"}, {"payload_hex": "00" * 4097}, {"authority_epoch": "-1"}, {"unknown": 1}]
        for change in changes:
            with self.subTest(change=list(change)):
                ledger = Ledger()
                ledger.receive(FIXTURES["messages"]["old"])
                before = copy.deepcopy(vars(ledger))
                message = copy.deepcopy(FIXTURES["messages"]["old"])
                message.update(change)
                with self.assertRaises(ValueError):
                    ledger.receive(message)
                self.assertEqual(vars(ledger), before)
        message = copy.deepcopy(FIXTURES["messages"]["old"])
        message["logical_id"] = {"hash": "abc"}
        with self.assertRaises(ValueError):
            envelope(message)

    def test_ancestry_128_boundary_and_ordinal_overflow(self):
        message = copy.deepcopy(FIXTURES["messages"]["old"])
        node = message["logical_id"]
        for depth in range(1, 130):
            node = {"kind": "output", "parent": {"tick": str(depth), "source_lp": 0, "logical_id": node}, "ordinal": 0}
            message["logical_id"] = node
            message["tick"] = "200"
            if depth <= 128:
                envelope(message)
            else:
                with self.assertRaises(ValueError):
                    envelope(message)
        child = copy.deepcopy(FIXTURES["messages"]["child10"])
        child["logical_id"]["ordinal"] = 1 << 32
        with self.assertRaises(ValueError):
            envelope(child)

    def test_nested_nonfuture_and_deep_input_reject_without_mutation(self):
        message = copy.deepcopy(FIXTURES["messages"]["child20"])
        outer = copy.deepcopy(message["logical_id"])
        message["logical_id"] = {"kind": "output", "parent": {"tick": "20", "source_lp": 1, "logical_id": outer}, "ordinal": 0}
        ledger = Ledger()
        before = copy.deepcopy(vars(ledger))
        with self.assertRaises(ValueError):
            ledger.receive(message)
        self.assertEqual(vars(ledger), before)
        node = {"kind": "root", "source_lp": 0, "sequence": "1"}
        for depth in range(2000):
            node = {"kind": "output", "parent": {"tick": str(depth + 1), "source_lp": 0, "logical_id": node}, "ordinal": 0}
        message["logical_id"] = node
        message["source_lp"] = 0
        message["tick"] = "3000"
        with self.assertRaises(ValueError):
            ledger.receive(message)
        self.assertEqual(vars(ledger), before)

    def test_gvt_all_categories_and_equality_fossil(self):
        empty = {name: [] for name in FIXTURES["gvt_categories"]}
        for name in empty:
            accounted = copy.deepcopy(empty)
            accounted[name] = ["10"]
            with self.assertRaises(ValueError):
                validate_floor("9", "11", accounted)
            self.assertEqual(validate_floor("9", "10", accounted), 10)
        self.assertEqual(fossil_ticks(["9", "10", "11"], "10"), ["10", "11"])
        with self.assertRaises(ValueError):
            validate_floor("10", "9", empty)
        with self.assertRaises(ValueError):
            validate_floor("0", "1", {})

    def test_oversized_scalar_before_mutation(self):
        message = copy.deepcopy(FIXTURES["messages"]["old"])
        message["logical_id"] = {"kind": "root", "source_lp": 0, "sequence": "9" * 70000}
        ledger = Ledger()
        before = copy.deepcopy(vars(ledger))
        with self.assertRaises(ValueError):
            ledger.receive(message)
        self.assertEqual(vars(ledger), before)


if __name__ == "__main__":
    unittest.main()
