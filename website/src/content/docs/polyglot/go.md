---
title: Go
description: Go package helpers and the requirement for actual Rust-native execution.
---

The Go package has metadata and data helpers. The former Go scheduler is unsupported immediately. Setting `CGO_ENABLED=1` or passing host-only tests does not establish a native adapter; supported execution requires actual calls to the Rust library and installed-consumer acceptance.

## Rust-only execution

Rust is the only supported simulation core. Host languages may prepare inputs,
orchestrate experiments and analyse results. They must fail explicitly for
missing native simulation operations and never substitute a host-language core.

Use the [binding README](https://github.com/edithatogo/kairos/blob/main/bindings/go/README.md) for the actual current
surface and tooling. The [Go tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/go-getting-started.md)
describes package checks within that boundary. Use the
[Rust quickstart](rust.md) when a required native adapter operation is unavailable.

## Support and migration

- [Installation](https://github.com/edithatogo/kairos/blob/main/docs/install.md).
- [Immediate Rust-only decision](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md) and [migration](https://github.com/edithatogo/kairos/blob/main/docs/release/migration.md).
- [Capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json) and [release gates](https://github.com/edithatogo/kairos/blob/main/conductor/release-gates.md).

The alpha support matrix must name tested operations and installed native
artefacts. It must not infer full model support from metadata or schema helpers.
