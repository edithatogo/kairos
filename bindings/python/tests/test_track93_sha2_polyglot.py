from __future__ import annotations

import hashlib
import pytest

import kairo_ecs


def test_standard_sha256_nist_vectors() -> None:
    # NIST CAVP 1: Empty string
    assert (
        hashlib.sha256(b"").hexdigest()
        == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    )

    # NIST CAVP 2: "abc"
    assert (
        hashlib.sha256(b"abc").hexdigest()
        == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    )

    # NIST CAVP 3: 56-byte vector
    vec_56 = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    assert (
        hashlib.sha256(vec_56).hexdigest()
        == "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    )


def test_event_log_batch_bytes_sha256_polyglot_parity() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(1000, priority=-1, kind=3)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None

    record = kairo_ecs.EventLogRecord.dispatched("run-sha2", event)
    batch = kairo_ecs.EventLogBatch([record])
    smoke = batch.to_smoke_bytes()

    # Verify hashlib.sha256 computes a valid 32-byte digest
    digest = hashlib.sha256(smoke).digest()
    assert len(digest) == 32

    # Verify single-bit corruption produces avalanche difference
    corrupted = bytearray(smoke)
    corrupted[0] ^= 0x01
    corrupted_digest = hashlib.sha256(corrupted).digest()
    assert digest != corrupted_digest
    # Avalanche effect: at least 30% of bits differ
    diff_bits = sum(bin(b1 ^ b2).count("1") for b1, b2 in zip(digest, corrupted_digest))
    assert diff_bits >= 80, f"Expected avalanche effect, got {diff_bits} different bits"
