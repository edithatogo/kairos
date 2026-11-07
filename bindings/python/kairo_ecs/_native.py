"""Explicit C ABI v1 bindings. All scheduling and state transitions run in Rust."""

from __future__ import annotations

import ctypes
import os
import sys
from pathlib import Path
from threading import RLock
from typing import Any, Self

from ._errors import NativeBackendUnavailable, NativeExecutionError


class _Stats(ctypes.Structure):
    _fields_ = [
        ("current_time_ticks", ctypes.c_uint64),
        ("scheduled_events", ctypes.c_uint64),
        ("dispatched_events", ctypes.c_uint64),
        ("cancelled_events", ctypes.c_uint64),
        ("pending_events", ctypes.c_uint64),
    ]


def _integer(value: int, bits: int, name: str, *, signed: bool = False) -> int:
    lower = -(1 << (bits - 1)) if signed else 0
    upper = (1 << (bits - int(signed))) - 1
    if type(value) is not int or not lower <= value <= upper:
        raise ValueError(f"{name} must be an integer in [{lower}, {upper}]")
    return value


class NativeSchedulerBridge:
    """Load an explicitly selected Rust library and validate its required engine ABI v1."""

    def __init__(self, lib_path: str | None = None) -> None:
        selected = lib_path or os.environ.get("KAIRO_ECS_FFI_LIBRARY")
        if not selected:
            raise NativeBackendUnavailable(
                "Rust native execution requires an absolute KAIRO_ECS_FFI_LIBRARY path; "
                "no host-language fallback is supported."
            )
        path = Path(selected)
        if not path.is_absolute() or not path.is_file():
            raise NativeBackendUnavailable(
                f"Rust native library must be an existing absolute file: {selected}"
            )
        self.lib_path = str(path.resolve(strict=True))
        try:
            library = ctypes.CDLL(self.lib_path)
            u64, i32, u32 = ctypes.c_uint64, ctypes.c_int32, ctypes.c_uint32
            signatures: dict[str, tuple[list[Any], Any]] = {
                "ffi_version": ([], u32),
                "engine_new": ([], u64),
                "engine_free": ([u64], ctypes.c_int),
                "engine_reset": ([u64], ctypes.c_int),
                "engine_current_time": ([u64], u64),
                "schedule_at": ([u64, u64, i32, u32], u64),
                "schedule_after": ([u64, u64, i32, u32], u64),
                "cancel_event": ([u64, u64], ctypes.c_int),
                "step": ([u64], ctypes.c_int),
                "run_for": ([u64, u64], ctypes.c_int),
                "run_until": ([u64, u64], ctypes.c_int),
                "run_until_or_for": ([u64, u64, u64], ctypes.c_int),
                "stats": ([u64], _Stats),
                "last_error_message": ([], ctypes.c_char_p),
            }
            for name, (argtypes, restype) in signatures.items():
                function = getattr(library, f"kairo_ecs_{name}")
                function.argtypes = argtypes
                function.restype = restype
            abi = int(library.kairo_ecs_ffi_version())
        except (OSError, AttributeError) as error:
            raise NativeBackendUnavailable(
                f"Unable to load the required Rust ABI from {self.lib_path}: {error}"
            ) from error
        if abi != 1:
            raise NativeBackendUnavailable(f"Unsupported Rust ABI version {abi}; expected 1")
        self._lib = library
        self.available = True

    def get_version(self) -> int:
        """Read the actual native ABI version; never synthesize one."""
        return int(self._lib.kairo_ecs_ffi_version())


class NativeEngine:
    """Owned Rust engine; no host scheduler, RNG, state-transition or model fallback.

    ABI v1 exposes u64 ticks/handles, i32 priorities and u32 event kinds. Full
    dispatched records and a public RNG API are not exposed by this ABI. This
    class does not invent missing operations in Python.
    """

    def __init__(self, lib_path: str | None = None) -> None:
        self._handle = 0
        self._lock = RLock()
        self._bridge = NativeSchedulerBridge(lib_path)
        self._lib = self._bridge._lib
        handle = int(self._lib.kairo_ecs_engine_new())
        if handle == 0:
            raise self._error("engine_new")
        self._handle = handle

    @property
    def backend(self) -> str:
        return "rust-c-abi-v1"

    @property
    def library_path(self) -> str:
        return self._bridge.lib_path

    def _error(self, operation: str) -> NativeExecutionError:
        detail = self._lib.kairo_ecs_last_error_message()
        text = detail.decode("utf-8", errors="replace") if detail else "unknown native error"
        return NativeExecutionError(f"Rust {operation} failed: {text}")

    def _open_handle(self) -> int:
        if self._handle == 0:
            raise NativeExecutionError("Rust engine is closed")
        return self._handle

    def _check_status(self, status: int, operation: str) -> None:
        if status != 0:
            raise self._error(operation)

    def stats(self) -> dict[str, int]:
        with self._lock:
            stats = self._lib.kairo_ecs_stats(self._open_handle())
            return {name: int(getattr(stats, name)) for name, _ in _Stats._fields_}

    @property
    def current_time_ticks(self) -> int:
        return self.stats()["current_time_ticks"]

    def schedule_at(self, at_ticks: int, *, priority: int = 0, kind: int = 0) -> int:
        return self._schedule("schedule_at", at_ticks, priority, kind)

    def schedule_after(self, after_ticks: int, *, priority: int = 0, kind: int = 0) -> int:
        return self._schedule("schedule_after", after_ticks, priority, kind)

    def _schedule(self, operation: str, ticks: int, priority: int, kind: int) -> int:
        _integer(ticks, 64, "ticks")
        _integer(priority, 32, "priority", signed=True)
        _integer(kind, 32, "kind")
        with self._lock:
            result = int(getattr(self._lib, f"kairo_ecs_{operation}")(
                self._open_handle(), ticks, priority, kind,
            ))
            if result == 0:
                raise self._error(operation)
            return result

    def cancel(self, event_handle: int) -> bool:
        _integer(event_handle, 64, "event_handle")
        with self._lock:
            status = int(self._lib.kairo_ecs_cancel_event(self._open_handle(), event_handle))
            if status == 2:  # KAIRO_ECS_ERR_NOT_FOUND, including already dispatched events.
                return False
            self._check_status(status, "cancel_event")
            return True

    def _advance(self, operation: str, *arguments: int) -> int:
        with self._lock:
            before = self.stats()["dispatched_events"]
            status = int(getattr(self._lib, f"kairo_ecs_{operation}")(
                self._open_handle(), *arguments,
            ))
            self._check_status(status, operation)
            return self.stats()["dispatched_events"] - before

    def step(self) -> bool:
        return self._advance("step") == 1

    def run_for(self, max_events: int) -> int:
        return self._advance("run_for", _integer(max_events, 64, "max_events"))

    def run_until(self, time_limit_ticks: int) -> int:
        return self._advance("run_until", _integer(time_limit_ticks, 64, "time_limit_ticks"))

    def run_until_or_for(self, time_limit_ticks: int, max_events: int) -> int:
        return self._advance(
            "run_until_or_for", _integer(time_limit_ticks, 64, "time_limit_ticks"),
            _integer(max_events, 64, "max_events"),
        )

    def reset(self) -> None:
        with self._lock:
            self._check_status(self._lib.kairo_ecs_engine_reset(self._open_handle()), "engine_reset")

    def close(self) -> None:
        with self._lock:
            if self._handle:
                handle, self._handle = self._handle, 0
                self._check_status(self._lib.kairo_ecs_engine_free(handle), "engine_free")

    def __enter__(self) -> Self:
        self._open_handle()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __del__(self) -> None:
        # Prefer explicit close/context-manager use. Never retry a failed free.
        if getattr(self, "_handle", 0):
            handle, self._handle = self._handle, 0
            self._lib.kairo_ecs_engine_free(handle)


def native_extension_status() -> dict[str, Any]:
    """Report discovery only; file existence never proves native execution."""
    suffix = ".dylib" if sys.platform == "darwin" else ".dll" if sys.platform == "win32" else ".so"
    prefix = "" if sys.platform == "win32" else "lib"
    selected = os.environ.get("KAIRO_ECS_FFI_LIBRARY")
    return {
        "native_library": selected or f"{prefix}kairo_ecs_ffi{suffix}",
        "artifact_present": bool(selected and Path(selected).is_file()),
        "compiled": False,
        "status": "not_loaded" if selected else "not_configured",
        "bridge": "NativeEngine",
        "fallback_supported": False,
    }
