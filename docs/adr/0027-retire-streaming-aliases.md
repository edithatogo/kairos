# ADR 0027 Retire broker-named in-memory aliases

The next source revision of `kairo-ecs-streaming` removes the experimental
`kafka`, `nats`, `websocket` and `arrow-flight` Cargo features and
`KafkaStream`, `NatsStream`, `WebSocketBridge` and `ArrowFlightEndpoint`.
They were shipped in the `v0.4.0-alpha.1` source as aliases for the same
in-process stream, not native adapters. This is a source-breaking upgrade.
Keeping them would make an application compile without providing the transport
it selected. An unsupported network selection must not fall back to memory.

## Migration and scope

For in-process tests, drop the old feature from the Cargo dependency and
import `kairo_ecs_streaming::adapters::InMemoryStream` directly. Code needing
a network connection must supply a separately qualified transport; the
in-memory adapter cannot replace one. The event stream contract, pacing logic
and core beta profile are unchanged. The typed TCP and Tonic transports and
their qualification remain separate. The published alpha source and earlier
track evidence remain reproducible at their original revisions.

The crates.io index had no entry for this crate on 25 November 2026. Read-only
checks of the verified default branches of CareOps, CareOps Sim, Innovate,
Paignia, Tobac, Dapane and Voiage found no dependency on the features or alias
symbols. The CareOps Sim hit only inventories the existing manifest path;
its gitlink and file path are unchanged. Other Git and source consumers
must apply the migration on upgrade if they used the experimental surface.

## Verification

The [Track 36 test matrix](../../conductor/tracks/36-streaming-real-time-processing/test-matrix.md)
keeps the complete event-log round trip and the ordering, snapshot and pacing
oracles. Retiring the feature-presence checks does not qualify or waive any
live transport acceptance.
