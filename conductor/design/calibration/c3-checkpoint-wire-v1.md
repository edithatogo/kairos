# C3 checkpoint transport v1

Private implementation protocol under ADR-0021. This wire transports a C3
observed-ledger frontier and the complete admitted probe inventory. Pending
records carry real model runtime images; completed/failed/censored/infeasible
records carry terminal results and never restart a runtime.

The caller supplies the trusted run binding and admitted (spec, historical
snapshot) inventory reconstructed from the same pinned normalized trace and
model configuration. Wire rows bind the entire spec and snapshot with a
canonical SHA-256 digest, including the full C2 seed key, logical provenance,
claims, visible events, assumptions and budget. The bytes cannot select code,
seed/configuration, or a different historical snapshot. This is input-referenced
recovery: the retained normalized input/configuration remains required, exactly
as model codecs and graphs remain required by the C2 image protocol.

All integers are unsigned little-endian. Header: 8-byte magic `KC3RUN01`,
u32 version 1, 32-byte trusted run binding, u64 ledger frontier, u64 record count.
Each ID-sorted row: u64 ID length, UTF-8 ID, 32-byte spec/snapshot digest,
u64 consumed events, u128 last tick, u8 state tag, u64 payload length, payload.
Tags: 1 pending native image; 2 completed u128 tick; 3 missing empty; 4 resource
infeasible UTF-8 reason; 5 tick-censored empty; 6 event-censored empty; 7 failed
UTF-8 reason. A final SHA-256 digest covers all preceding bytes. Digests detect
corruption and mismatch, not authenticity.

Limits apply to complete wire bytes, record count, ID bytes, native image bytes
and reason bytes. Encode measures before allocation. Decode checks complete
framing/count minimum widths, all trusted metadata, state tags, UTF-8, canonical
ordering, snapshot frontier <= ledger frontier and payload lengths before
cloning any pending image. The typed pool restore then validates semantic state
and restores all worlds privately before returning a usable pool. A wire decode
alone is not evidence of runtime recovery or C-03 acceptance.

Disk publication uses the reviewed no-clobber checkpoint file helper in the
orchestration layer; this module is a byte codec and performs no filesystem I/O.

## Bounded source reconstruction

`frontier_hint` checks file cap, checksum, version, trusted run binding, probe-count framing and the caller's source-event cap before returning a frontier. It does not authorize a runtime restore. The runner rebuilds only the trusted observed prefix, derives the exact admitted inventory, then calls full `decode` and staged pool restore. No prediction prefix is replayed and no native image is decoded by the hint.

C4 physical v1 retains its historical `fidelity=ShadowAnchored` replay-policy spelling. The C3 output join carries actual `execution_mode=Macro|Micro` and `replay_role=ShadowAnchored` independently in metric strata and raw residual groups in the join manifest; the physical schema is unchanged.
