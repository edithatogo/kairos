#!/usr/bin/env python3
"""
Independent Protocol Buffers encoder and conformance fixture generator for Track 81.
Implements the canonical protobuf wire format according to `crates/kairo-ecs-grpc/proto/simulation.proto`
without depending on prost or the production Rust implementation.
"""

import json
import os
import sys

def encode_varint(value: int) -> bytes:
    """Encodes an unsigned integer into protobuf varint format."""
    out = bytearray()
    while value > 0x7F:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value & 0x7F)
    return bytes(out)

def decode_varint(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decodes a protobuf varint starting at offset. Returns (value, next_offset)."""
    res = 0
    shift = 0
    idx = offset
    while idx < len(data):
        b = data[idx]
        idx += 1
        res |= (b & 0x7F) << shift
        if not (b & 0x80):
            return res, idx
        shift += 7
    raise ValueError("Truncated varint")

def encode_tag(field_number: int, wire_type: int) -> bytes:
    return encode_varint((field_number << 3) | wire_type)

def encode_varint_field(field_number: int, value: int) -> bytes:
    # In proto3, default value (0) for numeric types can be omitted or included;
    # prost derives always serialize fields if non-default, but in message structs:
    if value == 0:
        return b""
    return encode_tag(field_number, 0) + encode_varint(value)

def encode_varint_field_always(field_number: int, value: int) -> bytes:
    return encode_tag(field_number, 0) + encode_varint(value)

def encode_bytes_field(field_number: int, data: bytes) -> bytes:
    if len(data) == 0:
        return b""
    return encode_tag(field_number, 2) + encode_varint(len(data)) + data

def encode_string_field(field_number: int, s: str) -> bytes:
    data = s.encode("utf-8")
    if len(data) == 0:
        return b""
    return encode_tag(field_number, 2) + encode_varint(len(data)) + data

def encode_embedded_message(field_number: int, msg_bytes: bytes) -> bytes:
    return encode_tag(field_number, 2) + encode_varint(len(msg_bytes)) + msg_bytes

def encode_u128_be(val: int) -> bytes:
    return val.to_bytes(16, byteorder="big")

def encode_root_frame(root_source_lp: int, root_sequence: int) -> bytes:
    out = bytearray()
    if root_source_lp != 0:
        out.extend(encode_varint_field(1, root_source_lp))
    if root_sequence != 0:
        out.extend(encode_varint_field(2, root_sequence))
    return bytes(out)

def encode_output_frame(parent_tick: int, parent_source_lp: int, ordinal: int) -> bytes:
    out = bytearray()
    out.extend(encode_bytes_field(1, encode_u128_be(parent_tick)))
    if parent_source_lp != 0:
        out.extend(encode_varint_field(2, parent_source_lp))
    if ordinal != 0:
        out.extend(encode_varint_field(3, ordinal))
    return bytes(out)

def encode_ancestry_frame(frame: dict) -> bytes:
    out = bytearray()
    if frame["type"] == "root":
        rf_bytes = encode_root_frame(frame["root_source_lp"], frame["root_sequence"])
        out.extend(encode_embedded_message(1, rf_bytes))
    elif frame["type"] == "output":
        of_bytes = encode_output_frame(frame["parent_tick"], frame["parent_source_lp"], frame["ordinal"])
        out.extend(encode_embedded_message(2, of_bytes))
    else:
        raise ValueError(f"Unknown frame type: {frame['type']}")
    return bytes(out)

def encode_optimistic_envelope(env: dict) -> bytes:
    """Encodes an envelope dictionary into protobuf binary bytes conforming to simulation.proto."""
    out = bytearray()

    # 1: envelope_schema_version = 1
    if env["envelope_schema_version"] != 0:
        out.extend(encode_varint_field(1, env["envelope_schema_version"]))

    # 2: protocol_id = 2
    if env["protocol_id"] != "":
        out.extend(encode_string_field(2, env["protocol_id"]))

    # 3: protocol_version = 3
    if env["protocol_version"] != 0:
        out.extend(encode_varint_field(3, env["protocol_version"]))

    # 4: authority_mode = 4 (enum: 0=LOCAL_PREVIEW, 1=SCOPED)
    if env["authority_mode"] != 0:
        out.extend(encode_varint_field(4, env["authority_mode"]))

    # 5: simulation_namespace = 5 (16 bytes)
    ns_bytes = encode_u128_be(env["simulation_namespace"])
    out.extend(encode_bytes_field(5, ns_bytes))

    # 6: ownership_epoch = 6
    if env["ownership_epoch"] != 0:
        out.extend(encode_varint_field(6, env["ownership_epoch"]))

    # 7: emission_epoch = 7
    if env["emission_epoch"] != 0:
        out.extend(encode_varint_field(7, env["emission_epoch"]))

    # 8: source_lp = 8
    if env["source_lp"] != 0:
        out.extend(encode_varint_field(8, env["source_lp"]))

    # 9: dest_lp = 9
    if env["dest_lp"] != 0:
        out.extend(encode_varint_field(9, env["dest_lp"]))

    # 10: tick = 10 (16 bytes)
    tick_bytes = encode_u128_be(env["tick"])
    out.extend(encode_bytes_field(10, tick_bytes))

    # 11: ancestry_frames = 11 (repeated)
    for frame in env["ancestry_frames"]:
        af_bytes = encode_ancestry_frame(frame)
        out.extend(encode_embedded_message(11, af_bytes))

    # 12: incarnation = 12
    if env["incarnation"] != 0:
        out.extend(encode_varint_field(12, env["incarnation"]))

    # 13: kind = 13 (enum: 0=POSITIVE, 1=ANTI)
    if env["kind"] != 0:
        out.extend(encode_varint_field(13, env["kind"]))

    # 14: event_payload = 14
    payload_bytes = env["event_payload_hex"]
    if isinstance(payload_bytes, str):
        payload_bytes = bytes.fromhex(payload_bytes)
    if len(payload_bytes) > 0:
        out.extend(encode_bytes_field(14, payload_bytes))

    return bytes(out)

def build_conformance_cases():
    cases = []

    # Case 1: Minimal root envelope (positive event, 128-bit tick exceeding u64, scoped authority)
    c1 = {
        "case_id": "root_positive_full_u128_tick",
        "envelope_schema_version": 2,
        "protocol_id": "kairo.ecs.distributed.grpc.v1",
        "protocol_version": 2,
        "authority_mode": 1, # SCOPED
        "simulation_namespace": 0xfeed_face_cafe_beef_0123_4567_89ab_cdef,
        "ownership_epoch": 101,
        "emission_epoch": 202,
        "source_lp": 3,
        "dest_lp": 7,
        "tick": (1 << 64) + 500,
        "ancestry_frames": [
            {
                "type": "root",
                "root_source_lp": 3,
                "root_sequence": 42
            }
        ],
        "incarnation": 5,
        "kind": 0, # POSITIVE
        "event_payload_hex": "70726f73742d776972652d7061796c6f61642d7631" # b"prost-wire-payload-v1".hex()
    }
    c1_wire = encode_optimistic_envelope(c1)
    c1["independent_protobuf_wire_hex"] = c1_wire.hex()
    cases.append(c1)

    # Case 2: Deep ancestry chain (root + output frame, antimessage kind)
    c2 = {
        "case_id": "deep_ancestry_anti_message",
        "envelope_schema_version": 2,
        "protocol_id": "kairo.ecs.distributed.grpc.v1",
        "protocol_version": 2,
        "authority_mode": 1, # SCOPED
        "simulation_namespace": 0x1234_5678_9abc_def0,
        "ownership_epoch": 42,
        "emission_epoch": 43,
        "source_lp": 10,
        "dest_lp": 20,
        "tick": 500_000,
        "ancestry_frames": [
            {
                "type": "root",
                "root_source_lp": 10,
                "root_sequence": 1
            },
            {
                "type": "output",
                "parent_tick": 200_000,
                "parent_source_lp": 10,
                "ordinal": 1
            },
            {
                "type": "output",
                "parent_tick": 350_000,
                "parent_source_lp": 15,
                "ordinal": 2
            }
        ],
        "incarnation": 2,
        "kind": 1, # ANTI
        "event_payload_hex": "616e74692d63616e63656c2d7061796c6f6164" # b"anti-cancel-payload".hex()
    }
    c2_wire = encode_optimistic_envelope(c2)
    c2["independent_protobuf_wire_hex"] = c2_wire.hex()
    cases.append(c2)

    return {
        "fixture": "optimistic_envelope_v2_conformance",
        "version": 1,
        "schema": "kairo.ecs.simulation.v1.OptimisticEnvelope",
        "spec_reference": "crates/kairo-ecs-grpc/proto/simulation.proto",
        "independent_generator": "scripts/generate_protobuf_envelope_conformance.py",
        "cases": cases
    }

def main():
    conformance_data = build_conformance_cases()
    out_path = os.path.join(os.path.dirname(__file__), "..", "conformance", "fixtures", "optimistic_envelope_v2_conformance.json")
    out_path = os.path.normpath(out_path)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(conformance_data, f, indent=2)
    print(f"Generated independent Protobuf conformance fixture at {out_path} ({len(conformance_data['cases'])} test cases)")

if __name__ == "__main__":
    main()
