"""Native FFI extension loader and scheduler bridge for Python binding."""

import ctypes
import sys
from pathlib import Path
from typing import Any


class NativeSchedulerBridge:
    """Thin ctypes/PyO3 loader bridge for the KairoECS C ABI shared library."""

    def __init__(self, lib_path: str | None = None) -> None:
        self.lib_path = lib_path
        self.available = False
        self._lib: ctypes.CDLL | None = None
        if lib_path and Path(lib_path).exists():
            try:
                self._lib = ctypes.CDLL(lib_path)
            except OSError:
                self._lib = None
            else:
                self.available = True

    def get_version(self) -> int:
        """Return the native FFI ABI version or 1 when running in fallback mode."""

        if self._lib is not None and hasattr(self._lib, "kairo_ecs_ffi_version"):
            try:
                return int(self._lib.kairo_ecs_ffi_version())
            except OSError:
                return 1
        return 1


def native_extension_status() -> dict[str, Any]:
    """Return discovery metadata for the native KairoECS FFI shared library."""

    if sys.platform == "darwin":
        ext_suffix = ".dylib"
    elif sys.platform == "win32":
        ext_suffix = ".dll"
    else:
        ext_suffix = ".so"
    lib_name = f"libkairo_ecs_ffi{ext_suffix}"
    found = Path(lib_name).exists()
    return {
        "native_library": lib_name,
        "compiled": found,
        "bridge": "NativeSchedulerBridge",
    }
