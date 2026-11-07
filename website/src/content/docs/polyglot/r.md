---
title: R
description: R package and data helpers with an explicit native-execution boundary.
---

The R package exposes metadata and data helpers. The earlier R scheduler is unsupported; a native runtime must be implemented and qualified before scheduling is advertised. Package installation or `kairoecs_surface_ready()` does not establish Rust model execution.

## Rust-only execution

Rust is the only supported simulation core. Host languages may prepare inputs,
orchestrate experiments and analyse results. They must fail explicitly for
missing native simulation operations and never substitute a host-language core.

Use the [binding README](https://github.com/edithatogo/kairos/blob/main/bindings/r/README.md) for the actual current
surface and tooling. The [R tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/r-getting-started.md)
describes package checks within that boundary. Use the
[Rust quickstart](rust.md) when a required native adapter operation is unavailable.

## Support and migration

- [Installation](https://github.com/edithatogo/kairos/blob/main/docs/install.md).
- [Immediate Rust-only decision](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md) and [migration](https://github.com/edithatogo/kairos/blob/main/docs/release/migration.md).
- [Capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json) and [release gates](https://github.com/edithatogo/kairos/blob/main/conductor/release-gates.md).

The alpha support matrix must name tested operations and installed native
artefacts. It must not infer full model support from metadata or schema helpers.
