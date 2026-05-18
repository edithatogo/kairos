# Entity Migration Protocol

Entity migration transfers ownership of an entity and its serialized component
state between LPs.

Message shape:

```text
(entity_id, component_data: Vec<(ComponentTypeId, Vec<u8>)>, source_lp, dest_lp, migration_id)
```

Required validation before transport send:

- `migration_id` is present and non-blank.
- `source_lp` and `dest_lp` are different LPs.
- at least one component blob is present.
- each component blob has a non-blank component type identifier.
- each component blob has non-empty serialized payload bytes.

Handshake:

1. source LP sends a migration request;
2. coordinator LP 0 acknowledges and records `migration_id`;
3. destination LP receives and applies component bytes;
4. destination LP acknowledges successful apply;
5. source LP deletes its local entity copy.

Consistency rules:

- `migration_id` provides deduplication and at-most-once apply.
- source deletion happens only after destination acknowledgment.
- migration occurs during an exchange phase, not while the LP is processing
  local events.
- duplicate `migration_id` values are acknowledged without reapplying payload
  bytes at the destination.

Current local smoke coverage:

- `MpiMigrationRequest::validate` checks the dependency-free MPI migration
  envelope before a real `rsmpi` backend is wired.
- `GrpcMigrationRequest::validate` checks the dependency-free gRPC migration
  envelope that mirrors `proto/simulation.proto`.
- Local tests include accepted complete migrations plus rejected self-migration
  and empty-component cases.
- `local_two_rank_contract_proof` and `local_two_node_contract_proof` now include
  one validated migration request each, proving the byte-preserving envelope
  shape is exercised in the local two-node emulator path. This is still not a
  real MPI or gRPC migration runtime.

## Protocol contract placeholders

- `MpiProtocol` contract identities are represented in Rust as:
  - `MpiContractMessage::Event`, `Null`, `Migration`, `Telemetry`
  - `MpiContractEnvelope` in `crates/kairo-ecs-mpi/src/lib.rs`
- `GrpcProtocol` contract identities are represented in Rust as:
  - `GrpcContractMessage::ExchangeEvents`, `MigrationRequest`,
    `StreamTelemetry`, `GvtProposal`
  - `GrpcContractEnvelope` in `crates/kairo-ecs-grpc/src/lib.rs`
- Both are local compile-time proof points only; they do not serialize or transport
  bytes yet.

## Validation command (scaffold)

```powershell
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi migration_validator
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc migration_validator
```

Expected output:

```text
test result: ok. ...
```

`--test`/`--features` execution remains optional runtime-proof status; on hosts with
linker blockers it is expected to fail before executable runtime validation can run.
