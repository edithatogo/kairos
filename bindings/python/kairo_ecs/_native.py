"""Native FFI extension loader and scheduler bridge for Python binding."""

import ctypes
import os
import sys
from typing import Any

class NativeSchedulerBridge:
    def __init__(self, lib_path: str | None = None) -> None:
        self.lib_path = lib_path
        self.available = False
        self._lib = None
        if lib_path and os.path.exists(lib_path):
            try:
                self._lib = ctypes.CDLL(lib_path)
                self.available = True
            except Exception:
                pass

    def get_version(self) -> int:
        if self._lib and hasattr(self._lib, "kairo_ecs_ffi_version"):
            try:
                return self._lib.kairo_ecs_ffi_version()
            except Exception:
                pass
        return 1

def native_extension_status() -> dict[str, Any]:
    ext_suffix = ".dylib" if sys.platform == "darwin" else (".dll" if sys.platform == "win32" else ".so")
    lib_name = f"libkairo_ecs_ffi{ext_suffix}"
    found = os.path.exists(lib_name)
    return {
        "native_library": lib_name,
        "compiled": found,
        "bridge": "NativeSchedulerBridge",
    }
