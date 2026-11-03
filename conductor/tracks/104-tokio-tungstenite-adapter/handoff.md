# Handoff

Updated: 3 November 2026. Maturity: Phase 0 deferral integrated; adoption and formal phase acceptance remain pending.

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

Reopen the recorded deferral only with an accepted consumer and transport contract. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

The bounded `$conductor-review` and accepted fixes were integrated by PR #360, merge commit SHA `615b2e17cf97f39d1090b591061f193b2faf4cb3`; the historical pushed ref was `codex/track104-async-adapter-decision`. The next-phase decision is deferral. Formal phase and shared-registry acceptance remain pending. Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.

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

## Current readback — 3 November 2026

Chat `01a19bed-7060-7910-af22-38cb31f51446` owns this bounded handoff and
baseline readback in `/Users/doughnut/.codex/worktrees/kairos-track103-104-review/kairos`,
branch `codex/track103-104-qualification-20261103`, based on `d7bdcd3`.
The integrator retains shared registry and phase acceptance; no production
transport, native service, dependency or downstream pin was changed. The
preceding instruction to integrate is historical: [PR #360](https://github.com/edithatogo/kairos/pull/360)
merged as `615b2e17cf97f39d1090b591061f193b2faf4cb3`.

Track 103 remains deferred. The Tokio framed-TCP source and tests have changed
since the prior receipt. At `d7bdcd3`, from this worktree on macOS arm64 with
Homebrew Rust/Cargo 1.99.1, `cargo test --locked -p kairo-ecs-grpc --test
 tokio_transport_oracles` exited zero with seven passes and zero ignored. Build
settings were `CARGO_TARGET_DIR=/Volumes/PortableSSD/codex-builds/kairos-track103-104-20261103`,
`CARGO_BUILD_JOBS=3` and `CARGO_PROFILE_TEST_DEBUG=0`. An earlier build was
interrupted before tests; the resumed run produced the successful result.

The resumed log `/Volumes/PortableSSD/codex-builds/kairos-track103-104-20261103/tokio-resume.log`
has SHA-256 `da8c873c9e9b629b0843fc08ae3f9743bdd29182903bebe589d308d86be11be5`;
the interrupted log has `f12b4c7cfbbc7d6a9106222c645ae9edc1e1009ff0069d7654992bae7da1cc41`.
`Cargo.lock` has `6f0517d02ffb3555b410ff2ae97892b8c639341785c65b5ae3864dbb6374bde7`
and `crates/kairo-ecs-grpc/src/socket.rs` has
`15854719de031f4f41548df7a8abc8538667b78b99441e4a83dd3106df624edf`.
These are framed-TCP tests, not RFC 6455, browser, WebSocket backpressure or
command-idempotency acceptance. The async adapter decision remains deferred.
