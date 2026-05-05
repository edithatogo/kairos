# Python Style Guide

- Target CPython 3.10, 3.11, 3.12, 3.13, and 3.14.
- Include a Python 3.14 free-threaded smoke lane where runner support exists.
- Use `ruff` for lint/format and `pytest` + `hypothesis` for tests.
- Keep per-event host callbacks out of hot loops unless explicitly labelled as slow/prototyping mode.
- Prefer Arrow/pyarrow batch outputs over Python object lists for telemetry.
- Generate or maintain type stubs for public APIs.
