from __future__ import annotations

import os
import tempfile
import pytest

import kairo_ecs


def test_isolated_tempdir_publication_lifecycle() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(500, priority=0, kind=1)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None

    record = kairo_ecs.EventLogRecord.dispatched("run-tempfile-py", event)
    batch = kairo_ecs.EventLogBatch([record])

    dir_path = ""
    with tempfile.TemporaryDirectory(prefix="kairo-test-tempfile-") as tmpdir:
        dir_path = tmpdir
        assert os.path.isdir(dir_path)

        out_file = os.path.join(tmpdir, "events.smoke")
        with open(out_file, "wb") as f:
            f.write(batch.to_smoke_bytes())

        assert os.path.isfile(out_file)
        assert os.path.getsize(out_file) > 0

    # Ensure hermetic cleanup on exit
    assert not os.path.exists(dir_path), "TemporaryDirectory must be cleaned up on context manager exit"


def test_tempfile_isolation_preserves_deterministic_content() -> None:
    scheduler1 = kairo_ecs.Scheduler()
    scheduler1.schedule_at(100, priority=0, kind=2)
    _, event1 = scheduler1.step()

    scheduler2 = kairo_ecs.Scheduler()
    scheduler2.schedule_at(100, priority=0, kind=2)
    _, event2 = scheduler2.step()

    record1 = kairo_ecs.EventLogRecord.dispatched("run-det-1", event1)
    record2 = kairo_ecs.EventLogRecord.dispatched("run-det-1", event2)

    bytes1 = kairo_ecs.EventLogBatch([record1]).to_smoke_bytes()
    bytes2 = kairo_ecs.EventLogBatch([record2]).to_smoke_bytes()

    # The payloads must be bit-for-bit identical regardless of different temporary environments
    assert bytes1 == bytes2
