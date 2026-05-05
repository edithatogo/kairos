from __future__ import annotations

import kairo_ecs


def test_import_exposes_package_identity() -> None:
    assert kairo_ecs.__version__ == "0.1.0"


def test_self_check_returns_ok_payload() -> None:
    payload = kairo_ecs.self_check()

    assert payload["package"] == "kairo_ecs"
    assert payload["version"] == "0.1.0"
    assert payload["status"] == "ok"
    assert payload["ffi"] == {
        "configured": False,
        "status": "not_configured",
        "detail": "set KAIRO_ECS_FFI_LIBRARY after native wheels are packaged",
        "library_path": None,
    }
