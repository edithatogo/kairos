---
title: Julia
description: Julia data utilities and the pending native engine acceptance boundary.
---

Julia provides DTO and event-log helpers. These utilities are not a simulation engine. A successful self-check, a version string or the existence of a native library file does not establish Rust execution. Use an installed-consumer acceptance receipt for any future runtime claim.

## Rust-only execution

Rust is the only supported simulation core. Host languages may prepare inputs,
orchestrate experiments and analyse results. They must fail explicitly for
missing native simulation operations and never substitute a host-language core.

Use the [binding README](https://github.com/edithatogo/kairos/blob/main/bindings/julia/README.md) for the actual current
surface and tooling. The [Julia tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/julia-getting-started.md)
describes package checks within that boundary. Use the
[Rust quickstart](rust.md) when a required native adapter operation is unavailable.

## Support and migration

- [Installation](https://github.com/edithatogo/kairos/blob/main/docs/install.md).
- [Immediate Rust-only decision](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md) and [migration](https://github.com/edithatogo/kairos/blob/main/docs/release/migration.md).
- [Capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json) and [release gates](https://github.com/edithatogo/kairos/blob/main/conductor/release-gates.md).

The alpha support matrix must name tested operations and installed native
artefacts. It must not infer full model support from metadata or schema helpers.
