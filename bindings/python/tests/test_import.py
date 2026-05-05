from __future__ import annotations

import kairo_ecs


def test_import_exposes_package_identity() -> None:
    assert kairo_ecs.__version__ == "0.1.0"


def test_self_check_returns_ok_payload() -> None:
    assert kairo_ecs.self_check() == {
        "package": "kairo_ecs",
        "version": "0.1.0",
        "status": "ok",
    }
