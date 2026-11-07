"""Retirement controls; native engine behavior is tested in native_tests/."""

from __future__ import annotations

import unittest

import kairo_ecs


class RustOnlyExecutionTests(unittest.TestCase):
    def test_legacy_scheduler_fails_immediately(self) -> None:
        with self.assertRaisesRegex(kairo_ecs.UnsupportedNativeOperation, "Rust core"):
            kairo_ecs.Scheduler()

    def test_synthetic_environments_are_not_available(self) -> None:
        for constructor, args in (
            (kairo_ecs.ParallelVecEnv, (2,)),
            (kairo_ecs.PettingZooParallelAdapter, (["p0", "p1"],)),
        ):
            with (
                self.subTest(constructor=constructor.__name__),
                self.assertRaises(kairo_ecs.UnsupportedNativeOperation),
            ):
                constructor(*args)
