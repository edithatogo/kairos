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

**Defer Tokio-Tungstenite.** At Kairos main
`c22172ccbf2d155a28f481a656ff3f7b6d9592ba`, Track 82 has supplied an async
runtime for a different wire protocol. Track 103 deferred Tungstenite because
the inspected dashboard consumes supplied JSON and returns command envelopes;
its live connection to Kairos has not been demonstrated. No owner has supplied
a concurrent WebSocket workload or an accepted simpler WebSocket transport
against which an async adapter could be measured.
[Track 82 merge](https://github.com/edithatogo/kairos/pull/325) ·
[Track 103 decision](https://github.com/edithatogo/kairos/pull/359)

Review traced both normal dependency graphs. The streaming crate with its
websocket feature has no normal dependencies and exposes an in-memory alias.
The gRPC crate uses Tokio 1.54.1; its GrpcSocketTransport uses custom framed
TCP with a bounded inbound semaphore, shutdown notification and read deadlines.
Its six existing Tokio oracles passed, covering cancellation, saturation,
stalled reads, oversized frames, reconnect and deterministic handoff under
jitter. These checks establish the existing runtime baseline only. They do
not exercise RFC 6455 frames, browser interoperability, command idempotency
or exactly-once delivery on reconnect.

Reproduce at the pinned source with Rust/Cargo 1.99.1 (tested on Darwin arm64):

```sh
cargo tree --locked --offline -e normal -p kairo-ecs-streaming --features websocket
cargo test --locked --offline -p kairo-ecs-grpc --test tokio_transport_oracles
```

The first command reported only kairo-ecs-streaming. The second exited zero
with six passes and zero ignored. Track 103's retained tests cover the
streaming alias; they were reused unchanged. Crates.io reported tokio-tungstenite
0.30.0 as stable and non-yanked, MIT with declared Rust 1.85. Package metadata
is inventory only. No new crate, features or runtime were installed for this
entry decision. CareOps Sim's 104-tokio-tungstenite-adapter handoff remains
unresolved with impact review pending.

Reopen when an application owner identifies the executable, endpoint, concurrent
connection workload and accepted Track 103 protocol. Compare against the
simpler transport under the same load; test queue saturation, cancellation
and shutdown while writes are pending, slow consumers, gap recovery and
duplicate command suppression before promotion. Runtime configuration and
healthcare session policy remain with the application owner.

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns the track directory in
`~/.codex/worktrees/track48-rollback/kairos` on codex/track104-async-adapter-decision.
Native scope was local testing and read-only repository, consumer and package
readback. Shared registry and phase acceptance remain with the integrator.
Next operation is protected integration of the decision.
