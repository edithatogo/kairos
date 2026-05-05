# Risk Register — 06 Python Binding 3.10-3.14

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Free-threaded/no-GIL Python 3.14 ABI break | 4 | 5 | 20 | Gate 3.14 CI on free-threaded builds; add `@gil_disabled` test suite; pin PyO3 version with known free-threaded support | python-agent | Free-threaded CI lane fails |
| PyO3 version lockstep with upstream Rust | 3 | 4 | 12 | Track PyO3 minimum-supported-Rust-version (MSRV) in CI matrix; use `[patch.crates-io]` override only during upstream gap windows | python-agent | PyO3 MSRV exceeds project MSRV for >1 week |
| Wheel ABI target policy mismatch | 3 | 4 | 12 | Lock `abi3-py310` as primary strategy; run `cibuildwheel` against full matrix on each PR; validate wheel tags with `auditwheel repair` + `delocate` + `delvewheel` | python-agent | `cibuildwheel` fails on any target platform |
| pyarrow version compatibility drift | 3 | 4 | 12 | Pin minimum/maximum pyarrow versions; add CI lane with `pyarrow>=MIN,<MAX` resolved; run C data roundtrip smoke test | python-agent | pyarrow C Data Interface smoke test fails |
| Pip/PyPI index propagation delay | 4 | 2 | 8 | Run `twine check` and `pip install --index-url https://test.pypi.org/simple/ kairo_ecs` as dry-run before production upload | python-agent | Test PyPI dry-run fails |
