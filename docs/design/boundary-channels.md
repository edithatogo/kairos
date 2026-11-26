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

### 6. Coordinated boundary wakeups

The experimental `BoundaryChannel::next_event_time` previews the earliest
pending arrival or expiry. A timeout includes its deadline tick. Expiry is due
at the next representable tick, so a maximum-tick deadline adds no expiry.
An arrived transfer can still expire until acknowledgement.

Use the preview to schedule a native DES event. Recompute the wakeup when a
channel operation changes, and replace a pending wakeup if its tick changes.
If the due tick has passed, handle the overdue work at the current
coordinated tick.
At dispatch, call `check_expiries` and then `collect_arrivals` with the emitted
tick. These calls apply expiry before arrivals due at that same tick. Expiry
batches report IDs in ascending order. When collecting arrivals across
several due ticks, the channel returns the earliest scheduled arrivals
first and preserves departure order at equal ticks. The model can schedule
receiving ABM events in that order at the current coordinated tick. The
envelopes retain their scheduled arrival ticks.
[Executable DES and ABM case](../../crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs)

### 7. Arrival tick overflow

Departure accepts an arrival at the maximum representable tick. A sampled
delay that would put the arrival beyond that tick returns
`BoundaryError::ArrivalOverflow`. The operation, free channel capacity,
pending wakeups and caller's random stream are preserved on this error.
The model can cancel the transfer or route it through a channel with a
feasible delay.

For the next prerelease built from this source, Rust callers matching
every experimental `kairo-ecs-des::BoundaryError` variant must handle
`ArrivalOverflow`. Successful departures use the same delay distribution
and seeded samples.
[Transit fixture](../../crates/kairo-ecs-des/tests/boundary_channel_transit_delay.rs)
