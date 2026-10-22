# tungstenite: conditional

Planning only; adoption pending. [Value policy](../../dependency-value-policy.md), [sequencing](../../dependency-sequencing.md).

## Inputs

Accepted local dependencies [78]. Exact live consumer inventory and independent baseline.

## Outputs

Accepted improve/adopt/defer/reject decision and bounded implementation receipts if justified. Existing-tooling optimisation is not a new library installation success criterion.

## Owned paths

Lead streaming-agent: kairo-ecs-streaming transport adapter; live CareOps consumer via separate owner packet. Planning owns this folder; runtime/manifest/lock/CI changes require explicit current-owner handoff and claims.

## Blocked paths

Unrelated owners, cross-repository source, clinical policy, DES/resource semantics and protected settings.

## Dependencies

[78]. Conditional branches do not gate the core migration. No network/ODE work solely to satisfy a library roadmap.

## Contract

Implement actual WebSocket transport only for a demonstrated live consumer. Current WebSocketBridge aliases InMemoryStream; do not describe this as deletion of a mature custom WebSocket stack. Use protocol machinery rather than hand-built framing; benchmark wrapper/build cost. Browser native WebSocket remains available without another browser transport library. Schema/protocol and payload limits must be qualified before accepting messages. Generic transport stays Kairos-owned; CareOps owns healthcare commands, frontend session policy and deployment.

## Acceptance criteria

Real loopback browser/native interoperability; malformed/oversize/fragmented messages, close/ping/pong, slow-reader backpressure, reconnect/gaps, cancellation and no duplicate commands/arrivals. Explicit origin/auth/TLS boundary appropriate to deployment; no production service implied. Test renderer-off identical simulation outcomes; finite resource limits.

## Quality gates

`dependency-103-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Independent failure controls, full costs, exact version/maintenance/security/licence/MSRV/target qualification before adoption.

## CareOps handoff

Accepted immutable upstream receipts before a parent pin/API change. Parent owns its consumer and test packets. No schema or scientific policy invented upstream.

## Release implications

Public protocol/numerical changes require compatibility/version review; no hosted or operational acceptance from planning.

## Reference to requalify

https://docs.rs/tungstenite/latest/tungstenite/
