# Handoff

Updated: 2026-10-22. Maturity: planned; adoption decision pending.

## Summary

Separate dependency track registered. No runtime change, installation or accepted implementation phase.

## Files changed

Planning folder and coordinated registry/policy surfaces only.

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

Prospective replacement boundaries only; no runtime API changed.

## Tests added

Independent future tests specified in test-matrix.md; planning validators only this pass.

## Known risks

Value and behavioural gates remain unresolved. Preserve published contracts and explicit owner boundaries; see the dependency-specific specification.

## Follow-up issues

Record exact existing consumer, baseline and adopt/no-go decision in Phase 0. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Pending: `$conductor-review`, accepted fixes, rejected findings, command/cwd/commit SHA/toolchain/seed/input hashes, exit codes/artifacts, pushed ref or blocker and next-phase decision. Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.

## Phase 0 decision — 1 November 2026

**Defer Tungstenite adoption.** Track 78 merged in [PR #317](https://github.com/edithatogo/kairos/pull/317).
At Kairos `a4506b65243fc4687556444ee366c0e6911fa663`, the streaming crate
declares `websocket = []`. Its `WebSocketBridge` type alias resolves to
`InMemoryStream`. The native library tests and feature matrix exercised the
stream-message contract with `cargo test --locked --offline -p kairo-ecs-streaming
--features websocket --lib --test feature_matrix`: 10 library and 8 integration
tests passed on Darwin arm64 with Rust/Cargo 1.99.1. This baseline performs no
WebSocket handshake or frame transmission.

Review checked the exported alias directly because its feature smoke only
asserts that a new stream is empty. This caller linked the feature-enabled
library at the same source revision:

```rust
use kairo_ecs_streaming::{websocket::WebSocketBridge, EventSink, EventSource, StreamMessage};
fn main() {
    let mut bridge = WebSocketBridge::default();
    for sequence in [1, 3] {
        bridge.publish(StreamMessage::event_log("fixture", 10, sequence)).unwrap();
    }
    assert!(bridge.publish(StreamMessage::event_log("fixture", 10, 3)).is_err());
    assert_eq!(bridge.len(), 2);
    for sequence in [1, 3] {
        assert_eq!(bridge.next_event().unwrap().unwrap().sequence, sequence);
    }
    assert!(bridge.next_event().unwrap().is_none());
    println!("bridge accepted 1,3; duplicate rejected; replay 1,3; then empty");
}
```

The observed sequence was 1,3 and the duplicate 3 was rejected. The gap was
accepted. These results establish process-local queue semantics only.
The type alias supplies no socket listener, payload-size bound, reconnect
synchronisation, or command acknowledgement.

### Downstream evidence and entry decision

CareOps Sim main `c8ac4171c8e9ff48852ccbcf8e7b77f0cdbaccba` exports
`SimulationWebSocketClient` from the dashboard. Its constructor stores a URL.
`sendCommand` returns an envelope; `handleIncomingMessage` processes a string
supplied by its caller. A sequence gap in a delta with a base tick can mark
the state `resync_required`. These code paths establish a protocol consumer,
but provide no socket-open, network send or completed snapshot resync. No
accepted live Kairos endpoint or deployment packet was found in the reviewed
source and downstream register. [Dashboard handler](https://github.com/edithatogo/careops-sim/blob/c8ac4171c8e9ff48852ccbcf8e7b77f0cdbaccba/apps/dashboard/src/live/websocket_client.ts) ·
[Consumer register](https://github.com/edithatogo/careops-sim/blob/c8ac4171c8e9ff48852ccbcf8e7b77f0cdbaccba/conductor/dependency-handoffs.json)

Crates.io listed Tungstenite 0.30.0 as latest stable and non-yanked, dual
MIT/Apache-2.0 with declared Rust 1.85. Phase 0 stopped before installation.
No features, protocol security, runtime interoperability or performance costs
were qualified for this candidate. No runtime dependency or listener was added.
The downstream Track 103 handoff remains unresolved/pending. Track 104's
asynchronous adapter still requires an accepted transport and an async consumer.

Reopen when an executable owner identifies the client and endpoint, selects the
wire schema and defines bounded queues, message-size limits, authentication,
origin/TLS boundary and replay/command-idempotency semantics. Then run real
browser/native loopback tests covering fragmentation, invalid frames, control
frames, slow readers, disconnect/reconnect and duplicate arrival, plus
renderer-disabled simulation parity.

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns the track directory in
/Users/doughnut/.codex/worktrees/track48-rollback/kairos on codex/track103-websocket-decision.
Native scope was local stream execution and read-only repository/package
inspection. The integrator retains shared phase and registry ownership.
Next operation is protected integration of this decision.
