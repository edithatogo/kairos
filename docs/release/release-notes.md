# Release Notes

## Unreleased: Rust-only execution and evidence corrections

- **Breaking:** Rust is the only supported simulation core. Independent
  host-language preview schedulers and synthetic model execution are unsupported
  immediately. Native/Wasm adapters must fail explicitly when unavailable.
- Python's native engine wrapper and TypeScript's generated-Wasm loader expose
  bounded Rust operations. Other surfaces remain unsupported until they pass
  native acceptance; a package import does not imply simulation support.
- The homepage, installation guides and paper draft now distinguish implemented
  capabilities, native execution, scientific validation and release acceptance.
- The [HPC scaling report](../benchmarks/scaling-certification-report.md)
  corrects generated placeholder efficiency/speedup figures. The separately
  measured PDES benchmark remains historical performance evidence.
- The [delivery programme](../../conductor/frontier-programme-20261107.md)
  defines native qualification, scientific-method completion, measured
  performance and beta/stable acceptance work.

See [ADR 0010](../adr/0010-rust-only-execution.md),
[compatibility](compatibility.md) and [migration](migration.md) before upgrading.

## v0.4.0-alpha.1 — 18 October 2026

The [GitHub prerelease](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1)
provides the alpha source snapshot and release artefacts. This notice does not
assert that every package registry, native wheel or platform is published or
qualified. Default-branch changes after the tag require their own verification.

## Release note requirements

- List user-visible changes and breaking API, ABI, schema or package changes.
- Link relevant ADRs, migration instructions and conformance fixtures.
- Record registry reservations, dry-runs, published artefacts and archive metadata separately.
- Match support claims to the capability register, release gates and exact candidate evidence.
