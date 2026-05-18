# Distributed Transport Trait

Track 35 consumes the Track 34 `PdesTransport` boundary:

```rust
fn knows_lp(&self, lp_id: LpId) -> bool;
fn send(&mut self, dest: LpId, message: PdesMessage) -> Result<(), TransportError>;
fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError>;
fn barrier(&mut self);
fn all_reduce_min(&mut self, timestamp: Tick) -> Tick;
```

The single-node reference is `ThreadChannelTransport` in `kairo-ecs-pdes`.
Distributed implementations must preserve the same message semantics:

- `MpiTransport`: rank-aware send/receive, barrier, and `MPI_Allreduce(MIN)`.
- `GrpcTransport`: peer RPC exchange, coordinator barrier, and coordinator
  minimum aggregation.

The current Track 35 crates intentionally compile without system MPI or tonic
dependencies. Their transports are dependency-free protocol emulators: they
round-trip messages locally, count barriers, and reduce GVT candidates so
transport semantics can be checked before real MPI/gRPC runtimes are wired in.
The emulators also include queued event timestamps in each `all_reduce_min`
round, matching Track 34's GVT rule that pending remote events bound the next
safe global time.

## Local proof scope

- **Protocol contracts are concrete and checked in compile-time unit tests**.
  `MpiTransport` and `GrpcTransport` now expose message-envelope placeholders
  (`MpiContractEnvelope`, `GrpcContractEnvelope`) that pin tag and message-kind
  identities for Event, Null, Migration, Telemetry, and GVT/GVT-sync flows.
- **Runtime transport wiring is still deferred.**
  No MPI launch, gRPC endpoint wiring, or cross-node binary orchestration is
  enabled in this track slice.
- **Error-path checks are validated locally.**
  Both emulated transports return transport errors for unknown LP ids and now
  verify contract consistency before accepting message envelopes.
- **Two-node local contract proof is dependency-free.**
  `local_two_rank_contract_proof` and `local_two_node_contract_proof` exercise
  event exchange and GVT floors through the placeholder transports. These
  helpers are local emulator evidence only; they do not start MPI ranks or gRPC
  services.

Validation commands:

```powershell
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --tests
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --tests
```

Expected output:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in ...
```
