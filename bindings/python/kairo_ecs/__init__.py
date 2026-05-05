"""Public Python package surface for KairoECS."""

from __future__ import annotations

__all__ = ["__version__", "self_check"]

__version__ = "0.1.0"


def self_check() -> dict[str, str]:
    """Return a small import-time health payload for smoke tests."""

    return {
        "package": "kairo_ecs",
        "version": __version__,
        "status": "ok",
    }
