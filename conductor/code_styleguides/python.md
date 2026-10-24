# Python Style Guide

- Use the qualified CPython 3.15.1 default and verify the actual repo-local interpreter.
- Prefer latest suitable qualified versions; necessary alternate profiles are permitted by the [dependency policy](../dependency-value-policy.md), with concrete scope/reason and equivalent qualification.
- Do not imply older or free-threaded support from a planning entry; qualify native bindings and CI before promising that lane.
- Use `ruff` for lint/format and `pytest` + `hypothesis` for tests.
- Keep per-event host callbacks out of hot loops unless explicitly labelled as slow/prototyping mode.
- Prefer Arrow/pyarrow batch outputs over Python object lists for telemetry.
- Generate or maintain type stubs for public APIs.
