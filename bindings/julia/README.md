# Julia Binding

Track 08 owns this binding surface.

This package currently provides a pure-Julia preview slice:

- deterministic event ordering by `(time_ticks, priority, sequence)`;
- a facade for the Track 70 Schema v2 `kairo_ecs.event_log.v2` Arrow schema
  fields (generational-ID index/generation columns, `time_ticks_hi`/`lo`
  limbs);
- an event-log smoke-byte roundtrip gate that preserves the Track 70 v2 field
  order (14 columns, decimal integer limbs, null-together entity columns)
  without requiring Arrow.jl at package load time;
- conformance fixture bridge helpers that distinguish Track 08 ready and
  planned fixture coverage without running native FFI;
- explicit native FFI status reporting.

**Documented copies and unsupported targets (Track 70 Phase 2, Task 2.3):**

- The smoke-byte codec performs a **value copy** per record into an escaped
  tab-separated projection of the Schema v2 columns; it is a deterministic
  dev-roundtrip and conformance gate, not a zero-copy Arrow buffer hand-off.
- Native Arrow.jl IPC reading/writing the populated FFI export is
  **deferred** and reported as unsupported until the Julia tooling lane is
  available; the binding does not consume `libkairo_ecs_ffi` yet
  (`ffi_status().configured == false`), so populated Arrow C Data/Stream
  exports are unsupported targets here.
- Entity projections are null-together per the v2 contract; malformed
  half-null rows are rejected explicitly on decode.

Native FFI is intentionally reported as not configured until Track 02 provides
a safe artifact layout for Julia package loading.

Local validation from this directory:

```bash
julia --project=. -e 'using Pkg; Pkg.test()'
julia --project=. -e 'include("test/test_arrow.jl")'
```

The fixture bridge accepts `ConformanceFixture` values, named tuples, or
dictionary-like manifest rows and reports only the fixtures that list Track 08
as a consumer. Planned fixtures stay visible as planned; they are not promoted
to passing coverage by this package slice.
