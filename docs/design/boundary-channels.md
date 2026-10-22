# Inter-Unit Boundary Channels & Asynchronous Handover Sinks

This document specifies the discrete-event simulation (DES) architecture for asynchronous cross-boundary entity transfers, stochastic transit delays, receiver capacity backpressure, and boarding holds in KairoECS.

## Architecture Overview

In complex modular discrete-event simulations (such as emergency departments, hospital networks, supply chains, or multi-depot logistics networks), entities transition between decoupled simulation units across boundaries. Coupling these units with synchronous shared memory or direct pointer-passing introduces fragile state management, race conditions, and scheduling deadlocks.

```
+------------------+         +--------------------+         +------------------+
|   Origin Unit    |         |  Boundary Channel  |         | Destination Sink |
|                  |         |                    |         |                  |
| [Sender Holding] | ====>   | [Stochastic Delay] | ====>   | [Reserved / Occ] |
| (Bed/Bay/Escort) |         | [Bounded Capacity] |         |  [Boarding Hold] |
+------------------+         +--------------------+         +------------------+
```

### 1. Operation Identity and Handover Envelopes

Every inter-unit transfer is tracked via a unique `HandoverOperationId(u64)` and encapsulated within a `HandoverEnvelope<T>`:
- `operation_id`: Stable operation identifier providing idempotency.
- `entity`: Transferred `EntityId`.
- `payload`: Generic domain payload (e.g. clinical acuity, cargo manifest, batch descriptor).
- `sender_unit` & `receiver_unit`: Distinct unit names.
- `state`: Explicit lifecycle phase (`TransferState`).
- Timestamps: `requested_at`, `depart_at`, `arrive_at`, `acknowledged_at`, and optional `timeout`.

### 2. Handover Lifecycle State Machine

```
      +-------------+
      |  Requested  |
      +------+------+
             |
             v
      +-------------+        Capacity Available
      |   Offered   | --------------------------------+
      +------+------+                                 |
             |                                        v
             | Capacity Full                   +-------------+
             +-------------------------------> |  Reserved   |
               (Boarding Hold at Sender)       +------+------+
                                                      |
                                                      v
      +-------------+     Arrival Time Met     +-------------+
      |   Arrived   | <----------------------- |  InTransit  |
      +------+------+                          +-------------+
             |
             v
      +-------------+
      |Acknowledged | (Physical custody transfers; Sender resource released)
      +-------------+
```

**Compensation & Failure Transitions**:
- `Rejected`: Receiver rejects entity due to sudden capacity loss or protocol failure.
- `Cancelled`: Sender cancels transfer before physical custody transfer.
- `Expired`: Reservation expires due to timeout.
- In all failure transitions, destination reservations are cleared, sender boarding holds are resolved, and waiting transfers are promoted.

### 3. Stochastic Transit Delay Distributions

`TransitDelayDistribution` models inter-unit travel and transmission times:
- `Fixed(SimTime)`: Constant delay.
- `Uniform { min: SimTime, max: SimTime }`: Uniform bounded delay sampled via `DeterministicStream`.
- `Exponential { mean_ticks: u128 }`: Exponential service or travel time via deterministic inverse-transform sampling.

### 4. Egress Sink Backpressure & Boarding Holds

`EgressSink<T>` maintains strict distinction between:
- **`reserved_count`**: Capacity reserved for in-transit entities that have not yet physically arrived.
- **`occupied_count`**: Entities physically present at destination.
- **`total_committed() = reserved_count + occupied_count <= capacity`**.

When destination capacity is exhausted (`total_committed == capacity`), new intake requests enter **Boarding Hold** (`TransferState::Offered`). The sender unit physically holds the entity (e.g., patient stays in ED bed, truck waits in loading bay) until destination occupancy is freed and the hold is promoted to `Reserved`.

### 5. Entity Conservation Accounting

Single-scheduler two-depot accounting guarantees:
$$\text{Origin Entities} + \text{In-Transit Envelopes} + \text{Destination Entities} = \text{Total Entities}$$
at every tick. Duplicate messages or retry attempts with the same `HandoverOperationId` produce no duplicate allocations or entity losses.
