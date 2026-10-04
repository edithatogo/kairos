"""Draft fixture oracle only: no wire decoding, execution, persistence or GVT protocol."""
import json
import re

MAX_BYTES = 65536
MAX_PAYLOAD = 4096
U64 = (1 << 64) - 1
U32 = (1 << 32) - 1


def fields(value, names):
    if type(value) is not dict or set(value) != set(names):
        raise ValueError("unexpected shape")


def integer(value, maximum):
    if type(value) is not int or not 0 <= value <= maximum:
        raise ValueError("integer range/type")
    return value


def decimal(value):
    if type(value) is not str or not re.fullmatch(r"0|[1-9][0-9]{0,19}", value):
        raise ValueError("noncanonical u64")
    return integer(int(value), U64)


def identity(node):
    # Iterative bounded preflight prevents recursion/serialization preceding validation.
    parents = []
    while True:
        if type(node) is not dict:
            raise ValueError("ancestry type")
        if node.get("kind") == "root":
            fields(node, ("kind", "source_lp", "sequence"))
            result = (0, integer(node["source_lp"], U32), decimal(node["sequence"]))
            break
        if len(parents) >= 128:
            raise ValueError("ancestry bound")
        fields(node, ("kind", "parent", "ordinal"))
        if node["kind"] != "output":
            raise ValueError("unknown identity")
        parent = node["parent"]
        fields(parent, ("tick", "source_lp", "logical_id"))
        parents.append((decimal(parent["tick"]), integer(parent["source_lp"], U32), integer(node["ordinal"], U32)))
        node = parent["logical_id"]
    for tick, source, ordinal in reversed(parents):
        if result[0] == 0 and result[1] != source:
            raise ValueError("parent root source mismatch")
        if result[0] == 1 and tick <= result[1][0]:
            raise ValueError("nonfuture ancestor")
        result = (1, (tick, source, result), ordinal)
    return result


def envelope(message):
    fields(message, ("kind", "source_lp", "dest_lp", "tick", "logical_id", "authority_epoch", "incarnation", "payload_hex"))
    if message["kind"] not in ("positive", "anti"):
        raise ValueError("kind")
    source = integer(message["source_lp"], U32)
    dest = integer(message["dest_lp"], U32)
    tick = decimal(message["tick"])
    logical = identity(message["logical_id"])
    if logical[0] == 0 and logical[1] != source:
        raise ValueError("root source mismatch")
    if logical[0] == 1 and tick <= logical[1][0]:
        raise ValueError("output must be future")
    epoch = decimal(message["authority_epoch"])
    incarnation = decimal(message["incarnation"])
    payload = message["payload_hex"]
    if type(payload) is not str or len(payload) > 2 * MAX_PAYLOAD or not re.fullmatch(r"(?:[0-9a-f]{2})*", payload):
        raise ValueError("payload bytes")
    # Bound shapes/scalars/depth before serialization of a typed fixture.
    if len(json.dumps(message, separators=(",", ":")).encode()) > MAX_BYTES:
        raise ValueError("fixture byte limit")
    return (source, epoch, logical, incarnation), (dest, tick, bytes.fromhex(payload))


def order(message):
    key, metadata = envelope(message)
    return metadata[1], key[0], key[2]


class Ledger:
    """Membership-only oracle. Exact tombstones survive; no fossil mutation here."""
    def __init__(self):
        self.known = {}
        self.pending = set()
        self.tombstones = set()

    def receive(self, message):
        key, metadata = envelope(message)  # Whole validation before any mutation.
        if key in self.known and self.known[key] != metadata:
            raise ValueError("conflicting exact delivery metadata")
        if message["kind"] == "positive" and key in self.pending:
            raise ValueError("duplicate positive")
        self.known[key] = metadata
        if message["kind"] == "anti":
            self.pending.discard(key)
            self.tombstones.add(key)
            return "canceled"
        if key in self.tombstones:
            return "annihilated"
        self.pending.add(key)
        return "pending"


def validate_floor(previous, proposed, accounted):
    previous, proposed = decimal(previous), decimal(proposed)
    if proposed < previous:
        raise ValueError("regressing floor")
    categories = {"queued_positive", "queued_anti", "replay", "staged", "in_flight_positive", "in_flight_anti"}
    if type(accounted) is not dict or set(accounted) != categories:
        raise ValueError("incomplete accounting categories")
    ticks = []
    for values in accounted.values():
        if type(values) is not list:
            raise ValueError("invalid accounting list")
        ticks.extend(decimal(value) for value in values)
    if ticks and proposed > min(ticks):
        raise ValueError("unresolved work below floor")
    return proposed


def fossil_ticks(ticks, floor):
    floor = decimal(floor)
    return [value for value in ticks if decimal(value) >= floor]
