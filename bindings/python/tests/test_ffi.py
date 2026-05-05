from __future__ import annotations

import kairo_ecs


def test_ffi_status_is_explicitly_not_configured_by_default(monkeypatch) -> None:
    monkeypatch.delenv("KAIRO_ECS_FFI_LIBRARY", raising=False)

    assert kairo_ecs.ffi_status()["status"] == "not_configured"


def test_ffi_status_does_not_implicitly_load_env_library(monkeypatch) -> None:
    monkeypatch.setenv("KAIRO_ECS_FFI_LIBRARY", "native/kairo_ecs.dll")

    status = kairo_ecs.ffi_status()

    assert status["configured"] is False
    assert status["status"] == "not_loaded"
    assert status["library_path"] == "native/kairo_ecs.dll"
