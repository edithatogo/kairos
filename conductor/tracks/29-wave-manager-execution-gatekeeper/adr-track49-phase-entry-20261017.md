# ADR: renewed conditional Track 49 phase entry

Decision date: 17 October 2026 (Australia/Brisbane).
Status: approved by the human sole maintainer for a **bounded first packet only**.
Supersedes: the scheduling disposition in `adr-track49-phase-entry-20261004.md`,
which expired when bound interface and local semantics changed at `55fd635` (#207).

## Authority and scope

The sole maintainer was presented with four explicit decisions and answered "Yes"
to dispatching Track 49 real MPI/gRPC work, alongside authorizing the #255 merge,
the cross-owner `docs/design` write and the global status/phase update. This ADR
records the Track 49 scheduling decision only. It is separate from any EXC-199
security/release decision and from the #255 merge decision.

Approval is human authority, not an AI review and not a gate pass. This ADR does
not change any dependency, owner, wave or track status, and it does **not** make
any raw validator pass. The wave-progression and dependency-closure failure for
`Track49 -> Track48 not Done` remains exactly as it was.

## What changed since the expired disposition

The 4 October disposition was bound to interface and local semantics that have
since moved:

- `786f50b` (#199) merged the preview event-owned optimistic runtime.
- `1afe522` (#204) merged the checked optimistic message codec bridge, which was
  packet 1 of the Track 49 preparation order.
- `55fd635` (#207) merged handler-owned execution, retirement and group-cut
  reporting.

The codec bridge prerequisite named in the 4 October preparation note is therefore
satisfied and merged. What remains unmet is the dependency-closure condition,
because Track 48 is still `In Progress` pending live distributed acceptance.

## Scope of this authorization: packet 2 only

`first-production-leaf-preparation-20261004.md` defines a four-packet order. This
ADR authorizes **packet 2 only**:

> A Track 49-owner contract packet defines the additive versioned envelope,
> authority-epoch ownership and durable-admission boundary. Freeze limits and
> rejection/receipt oracles; retain all old generic protobuf contracts.

Packet 2 deliberately introduces **no new third-party dependency**. The empty
`mpi = []` and `grpc = []` marker features, and the absence of `rsmpi`, `tonic`
and `prost` lock entries, are unchanged by this ADR.

Explicitly **not** authorized by this ADR:

- Packet 3, the codec/conversion leaf, which requires production
  `protobuf`/`tonic`/`prost` selection with fresh version and MSRV review and
  exclusive manifest/lockfile ownership.
- Packet 4, real gRPC service/client/process delivery and MPI rank/GVT exchange.
- Any enablement of the `mpi` or `grpc` features in CI.
- Any claim of distributed rollback, cross-participant GVT, migration or durable
  admission.
- Any Track 48 or Track 49 status advancement.

## Conditions still mandatory for later packets

1. **Supply-chain and advisory review.** `AGENTS.md` requires reading `SECURITY.md`
   before changing dependencies. Any new transport crate must clear `cargo-deny`,
   `cargo-audit` and Track 20 review before CI enablement. The repository carries
   an open advisory and audit-exception posture on `main`; EXC-193 is PR #193-only
   and EXC-199 remains an unapproved proposal. Neither extends to a new PR.
2. **MSRV review.** Workspace `rust-version` is `1.99`. Note that this supersedes
   the historical Track 48 MSRV 1.76 evidence, which described wrapper-labelled
   receipts rather than the current workspace floor. Any transport crate must be
   checked against 1.99, not against 1.76.
3. **Real evidence.** Local toolchain is available and is not the blocker:
   `mpiexec`, `mpicc` and `ompi_info` report Open MPI 5.0.11 and `protoc` reports
   libprotoc 36.2. The blocker is that `kairo-ecs-mpi` and `kairo-ecs-grpc` have no
   real backend and no integration-test directories, so the `multirank_smoke` and
   `two_process_smoke` targets in Track 49's own test matrix do not exist. Recorded
   in Track 48's `distributed-blocker-readback-20261017.md`.
4. **Dual review.** PDES and distributed roles must both review any interface
   change before it is frozen.
5. **Normal delivery.** Exact-head push, hosted checks, and normal merge for every
   packet. No protection bypass, and no planned command may be recorded as
   executed.

## Explicit anti-claims

This ADR does not make the raw Track 49 gate result pass, does not rewrite a
failed gate as a pass, does not waive distributed acceptance, and does not grant
release, publication or protected-branch authority. Raw dependency gates and
full-track acceptance are unchanged.