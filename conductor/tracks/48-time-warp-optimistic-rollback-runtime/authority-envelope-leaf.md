# Bound native authority envelope leaf

Status: proposed exact source `1afe522239087d2a7a92caa352ffd19a4fd1b254`, awaiting joint contract acceptance. Implementation ownership: `crates/kairo-ecs-pdes/src/optimistic.rs`, `crates/kairo-ecs-pdes/src/lib.rs`, and `crates/kairo-ecs-pdes/tests/optimistic_authority_envelope.rs`. Independent held-out ownership: a distinct test file `optimistic_authority_envelope_heldout.rs`. Coordinator owns contract/evidence/docs; no shared manifest/lock/CI/parent-pin changes.

## Exact initial API and behavior

Feature gate remains time-warp. Export Copy/Debug/Eq/PartialEq `OptimisticAuthority::{LocalPreview, Scoped { simulation_namespace: u128, ownership_epoch: u64 }}`. Do not derive Ord or Hash merely to make authority an execution tie-breaker. Add private immutable authority to OptimisticMessage; existing new/try_from_parts/internal emission assign LocalPreview. Add `authority(&self) -> OptimisticAuthority` and `try_from_authority_parts(event: RemoteEvent, logical_id: LogicalEventId, authority: OptimisticAuthority, incarnation: u64, kind: OptimisticMessageKind) -> Result<Self, OptimisticError>` using the existing complete-ancestry checked constructor. Retain exact bytes/ranges. as_anti and clone retain authority. Order keys are identical across authority/kind/incarnation differences.

Add typed `OptimisticError::ScopedAuthorityRequiresOwnedRuntime`. Existing validate_message rejects Scoped before any mutation; no scoped traffic reaches receive's allocation observation, queue/history/tombstone/replay membership. Public reconstruction of scoped metadata remains possible but all-local execution rejects it. Do not change DeliveryIdentity/EventQueue/replay/GVT/RNG/scheduler, public OptimisticLimits, or existing legacy errors beyond this explicit new variant. No owned/retirement method is implemented in this leaf.

## Behavioral proof

- Existing generated roots/outputs/replay and codec constructors remain LocalPreview; accepted legacy receive/rollback behavior remains.
- Full native u128 namespace/tick and u64 epoch/incarnation including zero/MAX roundtrip for both kinds. Complete nested multi-emitter ancestry and payload/destination retained.
- Clone/as_anti preserve authority and all other fields; order key equal across differing authority/kind/incarnation. Invalid ancestry still rejects using existing errors.
- Real all-local runtime receives scoped positive AND scoped anti and rejects typed unchanged-state error, including cases colliding with known legacy work. Compare report, pending queues, process/RNG snapshots, tokens and GVT; execute a subsequent valid legacy event to prove healthy state. Future emitted legacy incarnation is not advanced by rejected scoped MAX input.
- Independently authored held-outs cannot be changed by the implementation writer. Original missing-API RED remains distinct from successful final runs.

## Reviewed command binding at dispatch

Use matching absolute Cargo/rustc/rustdoc1.98.1 and1.76.0 with disjoint fresh CARGO_TARGET_DIR. Record actual versions/compiler-cache/source/lock hashes, deterministic fixture seeds, each command's real exit and logs. Unset compiler wrappers and environment flags. Focused crate test is `cargo test -p kairo-ecs-pdes --features pdes,time-warp --locked`; formatting is `cargo fmt --check`; Clippy is `cargo clippy -p kairo-ecs-pdes --all-targets --features pdes,time-warp --locked -- -D warnings`. Coordinator runs held-out tests on integrated source and combined just ci before full delivery. Existing hosted/PR/normal-merge gates remain.

One writer per claimed path, clean isolated worktrees, small hashed context, prewrite/precommit scope checks, one commit per claim and immediate release. Source hashes and contract status/base must be rebound after acceptance and at actual dispatch; this proposal is not execution authority.
