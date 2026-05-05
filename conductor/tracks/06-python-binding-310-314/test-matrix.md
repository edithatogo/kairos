# Test Matrix — 06 Python Binding 3.10-3.14

## Required tests

- `python -m pytest -q` against the Python binding surface in `bindings/python/`.
- `python -m compileall kairo_ecs tests` to catch syntax drift across the supported interpreter range.
- `python -m pip check` after build to catch packaging metadata issues.
- `python -m build --sdist --wheel` against the package metadata in `bindings/python/pyproject.toml`.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` as the smoke import check.

## Future-surface controls

- Do not add PyPI publishing, release automation, or registry credentials here.
- Do not widen beyond the Python binding surface into core scheduler, conformance, or release-engineering work.
- Do not add language-bridge APIs for Julia, R, TypeScript, C#, or Go in this track.
- Stop at binding-level verification until Track 12 owns shared fixture parity and Track 15 owns packaging dry-runs.

## CI command

```bash
python -m pytest -q && python -m compileall kairo_ecs tests && python -c "import kairo_ecs; print(kairo_ecs.self_check())"
```

