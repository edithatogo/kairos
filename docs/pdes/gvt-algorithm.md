# Global Virtual Time Algorithm

Global virtual time is defined as:

```text
GVT = min(local_time_of_all_LPs, min_timestamp_of_all_inflight_messages)
```

Track 34 starts with a conservative scaffold:

- each LP reports its local time;
- the transport provides `all_reduce_min(timestamp)`;
- the scheduler advances every LP to the computed minimum.

The in-process transport uses a deterministic in-memory reduction for unit
tests. Track 35 replaces this with MPI `Allreduce(MIN)` or a gRPC coordinator
aggregation while preserving the same scheduler boundary.

Validation command:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```
