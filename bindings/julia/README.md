# Julia Binding

Track 08 owns this binding surface.

This package currently provides a pure-Julia preview slice:

- deterministic event ordering by `(time_ticks, priority, sequence)`;
- a facade for the `kairo_ecs.event_log.v1` Arrow schema fields;
- conformance fixture bridge helpers that distinguish Track 08 ready and
  planned fixture coverage without running native FFI;
- explicit native FFI status reporting.

Native FFI is intentionally reported as not configured until Track 02 provides
a safe artifact layout for Julia package loading.

Local validation from this directory:

```bash
julia --project=. -e 'using Pkg; Pkg.test()'
```

The fixture bridge accepts `ConformanceFixture` values, named tuples, or
dictionary-like manifest rows and reports only the fixtures that list Track 08
as a consumer. Planned fixtures stay visible as planned; they are not promoted
to passing coverage by this package slice.
