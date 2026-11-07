"""Missing/invalid Rust artifact controls; no substitute engine is used."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import kairo_ecs
from kairo_ecs._native import _integer


class NativeBoundaryTests(unittest.TestCase):
    def test_missing_native_artifact_raises(self) -> None:
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(kairo_ecs.NativeBackendUnavailable):
                kairo_ecs.NativeEngine()
            with self.assertRaises(kairo_ecs.NativeBackendUnavailable):
                kairo_ecs.NativeSchedulerBridge()

    def test_relative_path_is_not_loaded_from_working_directory(self) -> None:
        with self.assertRaises(kairo_ecs.NativeBackendUnavailable):
            kairo_ecs.NativeEngine("libkairo_ecs_ffi.so")

    def test_file_presence_does_not_report_a_working_backend(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "not-a-library.so"
            path.write_text("not a native library", encoding="utf-8")
            with patch.dict(os.environ, {"KAIRO_ECS_FFI_LIBRARY": str(path)}):
                status = kairo_ecs.native_extension_status()
                self.assertTrue(status["artifact_present"])
                self.assertFalse(status["compiled"])
                self.assertEqual(status["status"], "not_loaded")
                with self.assertRaises(kairo_ecs.NativeBackendUnavailable):
                    kairo_ecs.NativeEngine()

    def test_abi_integer_boundaries_never_truncate(self) -> None:
        self.assertEqual(_integer((1 << 64) - 1, 64, "tick"), (1 << 64) - 1)
        for value in (-1, 1 << 64, True, 1.0):
            with self.subTest(value=value), self.assertRaises(ValueError):
                _integer(value, 64, "tick")
        self.assertEqual(_integer(-(1 << 31), 32, "priority", signed=True), -(1 << 31))
        with self.assertRaises(ValueError):
            _integer(1 << 31, 32, "priority", signed=True)

    def test_metadata_self_check_does_not_claim_execution(self) -> None:
        payload = kairo_ecs.self_check()
        self.assertEqual(payload["status"], "metadata_only")
        self.assertEqual(payload["execution"], "rust_native_required")
        self.assertFalse(payload["fallback_supported"])
