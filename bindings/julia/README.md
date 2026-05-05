# Julia Binding

Track 08 owns this binding surface.

This package currently provides a pure-Julia preview slice:

- deterministic event ordering by `(time_ticks, priority, sequence)`;
- a facade for the `kairo_ecs.event_log.v1` Arrow schema fields;
- explicit native FFI status reporting.

Native FFI is intentionally reported as not configured until Track 02 provides
a safe artifact layout for Julia package loading.

Local validation from this directory:

```bash
julia --project=. -e 'using Pkg; Pkg.test()'
```
