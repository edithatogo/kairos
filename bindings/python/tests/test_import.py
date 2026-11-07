from __future__ import annotations

import kairo_ecs


def test_import_exposes_package_identity() -> None:
    assert kairo_ecs.__version__ == "0.1.0"


def test_self_check_returns_metadata_only_payload(monkeypatch) -> None:
    monkeypatch.delenv("KAIRO_ECS_FFI_LIBRARY", raising=False)
    payload = kairo_ecs.self_check()

    assert payload["package"] == "kairo_ecs"
    assert payload["version"] == "0.1.0"
    assert payload["status"] == "metadata_only"
    assert payload["ffi"] == {
        "configured": False,
        "status": "not_configured",
        "detail": "set an absolute KAIRO_ECS_FFI_LIBRARY path; no host-language fallback is supported",
        "library_path": None,
    }
