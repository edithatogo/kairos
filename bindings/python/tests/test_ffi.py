from __future__ import annotations

import kairo_ecs


def test_ffi_status_is_explicitly_not_configured_by_default(monkeypatch) -> None:
    monkeypatch.delenv("KAIRO_ECS_FFI_LIBRARY", raising=False)

    status = kairo_ecs.ffi_status()

    assert status["configured"] is False
    assert status["status"] == "not_configured"
    assert status["detail"] == "set KAIRO_ECS_FFI_LIBRARY after native wheels are packaged"
    assert status["library_path"] is None


def test_ffi_status_does_not_implicitly_load_env_library(monkeypatch) -> None:
    monkeypatch.setenv("KAIRO_ECS_FFI_LIBRARY", "native/kairo_ecs.dll")

    status = kairo_ecs.ffi_status()

    assert status["configured"] is False
    assert status["status"] == "not_loaded"
    assert status["detail"] == "native FFI loading is not enabled in this pure Python slice"
    assert status["library_path"] == "native/kairo_ecs.dll"

def test_ffi_status_empty_env_var(monkeypatch) -> None:
    monkeypatch.setenv("KAIRO_ECS_FFI_LIBRARY", "")

    status = kairo_ecs.ffi_status()

    assert status["configured"] is False
    assert status["status"] == "not_configured"
    assert status["detail"] == "set KAIRO_ECS_FFI_LIBRARY after native wheels are packaged"
    assert status["library_path"] is None
