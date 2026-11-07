---
title: C#
description: C# native ABI declarations and unsupported managed-scheduler boundaries.
---

C# retains raw `NativeMethods` declarations for the Rust C ABI. The managed scheduler preview is unsupported. Compiling the managed assembly does not load, execute or qualify a native engine. Native support requires the matching library, actual execution and handle/ownership tests on each advertised platform.

## Rust-only execution

Rust is the only supported simulation core. Host languages may prepare inputs,
orchestrate experiments and analyse results. They must fail explicitly for
missing native simulation operations and never substitute a host-language core.

Use the [binding README](https://github.com/edithatogo/kairos/blob/main/bindings/csharp/README.md) for the actual current
surface and tooling. The [C# tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/csharp-getting-started.md)
describes package checks within that boundary. Use the
[Rust quickstart](rust.md) when a required native adapter operation is unavailable.

## Support and migration

- [Installation](https://github.com/edithatogo/kairos/blob/main/docs/install.md).
- [Immediate Rust-only decision](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md) and [migration](https://github.com/edithatogo/kairos/blob/main/docs/release/migration.md).
- [Capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json) and [release gates](https://github.com/edithatogo/kairos/blob/main/conductor/release-gates.md).

The alpha support matrix must name tested operations and installed native
artefacts. It must not infer full model support from metadata or schema helpers.
