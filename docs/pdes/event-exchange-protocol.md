# Event Exchange Protocol

Remote events use the tuple:

```text
(source_lp, dest_lp, tick, event_payload)
```

The protocol is conservative. If an LP is at local time `T` and declares a
minimum lookahead `L`, it may only schedule a remote event with timestamp
`tick >= T + L`. This prevents a receiver from later seeing an event that should
have arrived before its current safe processing horizon.

The Chandy-Misra-Bryant null-message rule is the deadlock-avoidance mechanism.
Whenever an LP advances, it sends each neighbor a null message containing
`local_time + lookahead`. The receiver treats that value as a lower bound on
future events from the sender.

The Track 34 scaffold implements `RemoteEvent`, `NullMessage`, and `PdesMessage`
and emits null messages from `PdesScheduler::step_until`.

Validation command:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```
