# Digital Twin Deployment Model

The `digital-twin` feature defines the first runtime boundary between FMU variables and live telemetry.

## Data flow

1. KairoECS steps an FMU at a simulation tick boundary.
2. Selected FMU output variables are read by value reference.
3. `DigitalTwinConnector` compares values with the previous publication.
4. Values whose absolute delta exceeds `epsilon` are published to deterministic topic names.
5. `TwinStateSnapshot` captures the ECS/twin state at the same tick boundary.
6. `TwinStateDiff` transports only changed or removed entries.

## Topic convention

The current scaffold uses:

```text
<topic-prefix>/fmi/<value-reference>
```

Track 36 owns the final streaming wire contract. This track must adapt to that contract instead of inventing a parallel telemetry protocol.

## State synchronization

Snapshots are sorted by key before checksumming so the same state has the same checksum regardless of collection order. The checksum is a drift-detection guard, not a cryptographic integrity mechanism.
