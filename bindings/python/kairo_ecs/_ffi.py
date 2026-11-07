"""Native configuration discovery; this function never loads executable code."""

import os
from dataclasses import dataclass


@dataclass(frozen=True)
class FfiStatus:
    configured: bool
    status: str
    detail: str
    library_path: str | None = None


def ffi_status() -> dict[str, str | bool | None]:
    """Return native FFI configuration without implicit dynamic loading."""

    library_path = os.environ.get("KAIRO_ECS_FFI_LIBRARY")
    if library_path:
        return {
            "configured": False,
            "status": "not_loaded",
            "detail": "NativeEngine validates and loads this explicit path; execution is not yet verified",
            "library_path": library_path,
        }
    return {
        "configured": False,
        "status": "not_configured",
        "detail": "set an absolute KAIRO_ECS_FFI_LIBRARY path; no host-language fallback is supported",
        "library_path": None,
    }
