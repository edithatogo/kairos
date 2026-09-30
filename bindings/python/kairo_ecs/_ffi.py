"""Native FFI status for the dependency-light Python package slice."""

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
            "detail": "native FFI loading is not enabled in this pure Python slice",
            "library_path": library_path,
        }
    return {
        "configured": False,
        "status": "not_configured",
        "detail": "set KAIRO_ECS_FFI_LIBRARY after native wheels are packaged",
        "library_path": None,
    }
