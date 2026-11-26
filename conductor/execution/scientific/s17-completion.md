# S17 completion design

## Boundary arrival horizon

Track 110 integrates this slice through the existing Track 68 boundary
channel. The implementation is in the experimental Rust API of
`kairo-ecs-des` 0.1.0. The DES and ABM coordinator requires monotonic
event ticks, and a departure queues an arrival only when its scheduled
tick fits in `SimTime`. Validate the sampled delay against the remaining
tick range before committing the transfer or advancing the caller's stream.
Reuse the existing typed tick addition and preserve valid RNG sequences.

For seed 17, consume one RNG word before drawing a delay of one or two
ticks. Subtract the delay from the maximum tick to choose a departure that
arrives exactly at the maximum. Move departure one tick later for the
overflow control. Overflow must preserve the reserved transfer, free slot,
empty wakeup preview and next draw from the original stream position. The
original unchecked addition fails this control.
[Transit fixture](../../../crates/kairo-ecs-des/tests/boundary_channel_transit_delay.rs)

## Remaining design

The workpack records the arrival-horizon leaf separately from the closure
of S17. Its oracle covers composition at the upper tick bound. The wider
contract still needs explicit scope and oracles for hierarchy lifetime,
access-aware movement, and hybrid model equivalence. Model-specific empirical
validation needs a separate named model and reference.
[Scientific register](../../scientific-completion.md#s17-resource-and-boundary-qualification)
