# Q0.1 Flow API design disposition

Date: 2026-09-29. This is an owner-approved **preimplementation design
disposition**, not a completed Track 25 template review of concrete public
symbols and not a release approval.

## Owner-approved direction

The Kairos repository owner directed adoption of the Q0.1 architecture: add a
private DES-owned `FlowRuntime`; preserve legacy DES and ABM behavior; place the
Flow-specific behavior adapter in ABM with an ABM-to-DES dependency; keep
continuation context typed, owned, and in-memory; and defer portable checkpoint
serialization to Track 22. The Track 01 lifetime-cap and cleanup assumptions
are narrowed in the runtime contract. Q0.2/Q0.3 must still freeze detailed
queue, callback, event, RNG, and command semantics.

## Exact protected roots

| Root | Design disposition |
|---|---|
| `crates/kairo-ecs-des` | [DES root disposition](flow-runtime-des-q0.1.md) |
| `crates/kairo-ecs-abm` | [ABM adapter root disposition](flow-runtime-abm-q0.1.md) |

Both existing roots are registered as `experimental` in the protected-surface
inventory and aligned policy, matrix, and release compatibility note. The
records use the Track 25 review fields but explicitly defer concrete symbol
review and formal release signoff.

## Release boundary

**Release hold: yes.** Neither alpha inclusion nor release is authorized here.
After Q0.2/Q0.3 and implementation, Track 25 must review the exact symbols,
compatibility behavior, and test evidence for both roots. Release and red-team
review remain separate gates. The current owner direction does not waive those
gates or mark the full Track 25 workstream complete.

## Review basis

Independent read-only technical reviews assessed the current upstream source and
Track 01/03/25 contracts. Their findings are recorded in the parent CareOps
[`Q0.1 source and owner review`](../../../../conductor/evidence/q0.1-current-source-review-20260929.md).
Those technical reviews informed this disposition; they are not represented as
Track 25 release signoffs.
