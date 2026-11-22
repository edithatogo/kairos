"""Real Rust ABI smoke and lifecycle checks. Missing native configuration FAILS.

Run separately from host metadata/negative tests. This suite never substitutes a
mock library and contains no conditional skip. Set KAIRO_ECS_FFI_LIBRARY to the
built library's absolute path and PYTHONPATH=bindings/python.
"""

from __future__ import annotations

import copy
import pickle
import unittest

from kairo_ecs import NativeEngine, NativeExecutionError


class RustEngineTests(unittest.TestCase):
    def test_native_scheduling_cancellation_and_counts(self) -> None:
        with NativeEngine() as engine:
            self.assertEqual(engine.backend, "rust-c-abi-v1")
            self.assertFalse(engine.step())
            first = engine.schedule_at(5, priority=-1, kind=10)
            cancelled = engine.schedule_at(3, kind=20)
            engine.schedule_at(9, kind=30)
            self.assertTrue(engine.cancel(cancelled))
            self.assertFalse(engine.cancel(cancelled))
            self.assertTrue(engine.step())
            self.assertEqual(engine.current_time_ticks, 5)
            self.assertFalse(engine.cancel(first))
            self.assertEqual(engine.run_for(10), 1)
            self.assertEqual(engine.stats(), {
                "current_time_ticks": 9, "scheduled_events": 3,
                "dispatched_events": 2, "cancelled_events": 1, "pending_events": 0,
            })
            self.assertFalse(engine.step())

    def test_native_run_limits_and_reset(self) -> None:
        with NativeEngine() as engine:
            engine.schedule_after(4, kind=1)
            engine.schedule_at(8, kind=2)
            self.assertEqual(engine.run_until(3), 0)
            self.assertEqual(engine.run_until_or_for(8, 1), 1)
            self.assertEqual(engine.current_time_ticks, 4)
            self.assertEqual(engine.stats()["pending_events"], 1)
            engine.reset()
            self.assertEqual(engine.stats()["pending_events"], 0)
            self.assertEqual(engine.current_time_ticks, 0)

    def test_native_wide_ticks_are_exact_and_out_of_range_rejects(self) -> None:
        with NativeEngine() as engine:
            tick = (1 << 64) - 1
            engine.schedule_at(tick, priority=-(1 << 31), kind=(1 << 32) - 1)
            self.assertTrue(engine.step())
            self.assertEqual(engine.current_time_ticks, tick)
            with self.assertRaises(ValueError):
                engine.schedule_at(1 << 64)
            self.assertEqual(engine.stats()["scheduled_events"], 1)
            with self.assertRaises(NativeExecutionError):
                engine.schedule_after(1)

    def test_close_does_not_allow_further_execution(self) -> None:
        engine = NativeEngine()
        engine.close()
        engine.close()
        for operation in (engine.step, engine.stats, lambda: engine.schedule_at(1)):
            with self.assertRaises(NativeExecutionError):
                operation()

    def test_handles_belong_to_one_engine_and_one_live_run(self) -> None:
        with NativeEngine() as first, NativeEngine() as second:
            stale = first.schedule_at(5)
            original = second.schedule_at(7)
            self.assertFalse(second.cancel(stale))
            self.assertEqual(second.stats()["pending_events"], 1)
            first.close()
            second.reset()
            self.assertFalse(second.cancel(original))
            self.assertNotEqual(second.schedule_at(8), original)
            self.assertFalse(second.cancel(stale))
            self.assertTrue(second.step())
            with self.assertRaisesRegex(NativeExecutionError, "precedes"):
                second.schedule_at(7)
            self.assertEqual(second.current_time_ticks, 8)
            self.assertEqual(second.stats()["scheduled_events"], 1)

    def test_owned_engine_cannot_be_copied_or_serialized(self) -> None:
        with NativeEngine() as engine:
            engine.schedule_at(5)
            for operation in (copy.copy, copy.deepcopy, pickle.dumps):
                with self.assertRaisesRegex(TypeError, "owns a Rust handle"):
                    operation(engine)
            self.assertTrue(engine.step())
            self.assertEqual(engine.current_time_ticks, 5)
