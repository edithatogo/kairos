# Phase 0 experiment

The candidate is the incomplete Tonic GVT reduction on Track 49 source
`6cac04f0549dbea7b3a2a55cbc9c9587742e3a00`. Silence and a valid zero
proposal can both yield tick zero. A useful diagnostic identifies the missing
LP and round within a caller-selected numeric run, without carrying endpoint
strings, payloads or arbitrary labels. Track 78 was merged in PR #317.

Before the probe, require identical GVT results, no installed global subscriber,
no RNG or clock input to the calculation, no diagnostic on a completed round,
and no change in virtual time. Test a deliberately slow 40 ms consumer: a hard
5 ms diagnostic-path budget rules out synchronous forwarding to an uncontrolled
sink. The budget concerns additional sink latency, not the transport timeout.
Keep the probe isolated from the workspace manifests and application logging.

## Results and decision

**Defer direct instrumentation.** In the exported candidate the silent peer
and the admitted zero proposal both reduced to zero with dispatch off and on.
Only the missing-proposal case emitted an event. The caller supplied run 97
through its scoped subscriber. The event fields were exactly worker 1, round
0 and missing peer 2. The oracle rejects unexpected field names or values and
non-numeric fields. This demonstrates GVT parity for the two fixtures; it
does not establish full simulation-output or RNG parity.

The subscriber confirmed that its callback ran on the same OS thread as the
reduction and spent at least 40 ms in that callback. This violates the proposed
5 ms bound even though the virtual result was unchanged. The slow sink was
deliberate. It measures sensitivity to application backpressure, not ordinary
event emission cost. Review moved both the enablement check and the event
callback outside the proposal mutex. A library cannot impose a latency bound
on an arbitrary application subscriber. Before reconsidering, the application
owner must supply an accepted bounded dispatch/drop policy and run-ID contract.
No global subscriber, log exporter, runtime dependency change or collector is
part of this packet. Track 100 remains separate.

Tracing 0.1.44 was the latest stable, non-yanked release at readback (MIT,
declared Rust 1.65.0; tokio-rs/tracing unarchived; last reported push 29
October 2026). The experiment requested only std without defaults. Other
consumers can unify features. Its checksum matches the existing workspace lock,
63e71662fa4b2a2c3a26f570f037eb95bb1f85397f3cd8076caed2f026a6d100.
The candidate added one direct dependency edge and no new resolved packages;
no runtime code was removed. Darwin arm64, Rust/Cargo 1.99.1: seven native
Tonic oracles and the new paired probe passed. Clippy and repository cargo-deny
advisory, ban, licence and source checks passed. The locked incremental test
command took 5.36 seconds and 277790720 bytes maximum RSS including Cargo and
compilation. Cross-platform cost, throughput under sustained saturation and
full simulation parity remain unqualified.

## Reproduction

Export the exact Track 49 commit above into an empty scratch directory with
`git archive`. In that directory, apply the adjacent `experiment.patch` with
`git apply`. Run:

```sh
cargo test --locked -p kairo-ecs-grpc --test track97_probe --test tonic_transport_oracles
cargo clippy --locked -p kairo-ecs-grpc --test track97_probe -- -D warnings
cargo deny --locked --manifest-path crates/kairo-ecs-grpc/Cargo.toml --config "$PWD/deny.toml" check
```

Raw receipts under /Volumes/PortableSSD/codex-builds/kairos-track97-Z6kYKd:

- `final.log` — 15902da9867b8fb55beddb639a11a22415ae512e46f9f5696c7c297344963531
- `deny.log` — f3465527ca1ea561190b13a30debfe4857cdf1ff9df8b1b3e7945877401de9dc
- `clippy.log` — 443abb4136043c29ebde7fa55edf92e7f19b8091c10a1694c1ebb0dc751ae3de
