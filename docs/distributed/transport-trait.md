# Distributed Transport Trait

Track 35 consumes the Track 34 `PdesTransport` boundary:

```rust
fn send(&mut self, dest: LpId, message: PdesMessage);
fn recv(&mut self, lp_id: LpId) -> Vec<PdesMessage>;
fn barrier(&mut self);
fn all_reduce_min(&mut self, timestamp: Tick) -> Tick;
```

The single-node reference is `ThreadChannelTransport` in `kairo-ecs-pdes`.
Distributed implementations must preserve the same message semantics:

- `MpiTransport`: rank-aware send/receive, barrier, and `MPI_Allreduce(MIN)`.
- `GrpcTransport`: peer RPC exchange, coordinator barrier, and coordinator
  minimum aggregation.

The current Track 35 crates intentionally compile without system MPI or tonic
dependencies. Their transport methods panic until the real backends are wired in.

Validation commands:

```powershell
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```
