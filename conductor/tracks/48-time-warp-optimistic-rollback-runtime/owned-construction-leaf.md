# Bounded owned-construction and live-peer foundation

Status: precise proposed dispatch contract, based on qualified architecture review at90dfbc5 and independent liveness objections on4 October2026; exact-document independent acceptance remains required. Native baseline is8150585e765ac69a701cfdbd4ed9dfd1ccbf6eff; actual dispatch binds the later contract commit with unchanged native files. This is an intermediate implementation leaf toward full Track48/49, not an enabled-owned-execution or final PR acceptance.

## Ownership and implementation boundary

Implementation owns only crates/kairo-ecs-pdes/src/optimistic.rs, crates/kairo-ecs-pdes/src/optimistic/owned.rs (new), crates/kairo-ecs-pdes/src/lib.rs and crates/kairo-ecs-pdes/tests/optimistic_owned_construction.rs (new). Independent author owns only tests/optimistic_owned_construction_heldout.rs in that crate. Coordinator owns this contract and dispatch/evidence ledger. No manifest, dependency, lockfile, CI, parent pin, legacy fixture or task-status changes.

Store an optional private owned-state descriptor in the real OptimisticRuntime. The new constructor creates process states, initial snapshots, token epochs and emitter counters only for actual owned LPs. It must not construct all-LP mirrors or substitute copies of published_messages. The owned state retains complete immutable global configuration and bounded options for subsequent routing/index/retirement leaves. Existing new initializes owned mode absent and retains every existing legacy behavior. Existing delivery indexes remain legacy-only until their exact-authority propagation leaf; owned scheduling/execution are explicitly gated here.

## Exact API

Export under time-warp:

```rust
#[derive(Clone, Debug)]
pub struct OptimisticOwnedOptions {
    pub simulation_namespace: u128,
    pub current_authorities: BTreeMap<LpId, OptimisticAuthority>,
    pub emission_epochs: BTreeMap<LpId, u64>,
    pub local_limits: OptimisticLimits,
    pub max_global_lps: usize,
    pub max_outbox_entries: usize,
    pub max_transition_entries: usize,
    pub max_receipt_entries: usize,
}

OptimisticRuntime::new_owned(
    partition: PartitionPlan,
    topology: BTreeMap<LpId, Vec<LpId>>,
    owned_processes: BTreeMap<LpId, P>,
    options: OptimisticOwnedOptions,
) -> Result<Self, OptimisticError>;

native_accounting_authority(&self) -> Result<NativeAccountingAuthority, OptimisticError>;
register_native_peer(&mut self, peer: NativeAccountingAuthority) -> Result<(), OptimisticError>;
seal_native_peers(&mut self) -> Result<(), OptimisticError>;
close_initial_inputs(&mut self) -> Result<(), OptimisticError>;
native_peers_sealed(&self) -> Result<bool, OptimisticError>;
initial_inputs_closed(&self) -> bool;
accounting_revision(&self) -> Result<u64, OptimisticError>;
```

NativeAccountingAuthority is opaque/private-fields, Clone and bounded Debug, no public constructor, deserialize, Eq/Ord/Hash or conversion from raw metadata. Immutable getters: runtime_id()->u64, recovery_generation()->u64, simulation_namespace()->u128, owned_lps()->&[LpId] sorted, is_live()->bool. Additional immutable authority getters: global_partition()->&PartitionPlan, global_topology()->&BTreeMap<LpId, Vec<LpId>>, current_authorities()->&BTreeMap<LpId, OptimisticAuthority>, emission_epochs()->&BTreeMap<LpId,u64> (issuer-owned map). These remain inspectable after issuer Drop and do not certify liveness.

Recovery generation is zero in this static leaf; runtime ID uses the existing checked unique instance allocator. Configuration equality is validated privately against complete PartitionPlan (including entities and lookahead), canonical directed topology and current_authorities. A hash alone is insufficient; per-owner emission epoch maps and limits may differ as their respective validation allows.

The runtime exclusively owns an Arc live witness; capabilities retain only Weak. Dropping owned state invalidates the flag and destroys the owner witness. Cloning a capability does not keep an issuer alive. No Clone implementation for runtime. Revalidate every registered peer's liveness before each ownership-dependent mutation, including repeated seal/close. is_live and registration checks establish point-in-time liveness; this flag does not exclude concurrent peer drop or certify durable accounting. Enabled admission must freeze its concurrency linearization later. The complete-group fossil join borrows every actual runtime exclusively and cannot use an independently sampled flag as a cut.

## Constructor validation and typed errors

Validate all configuration before model snapshot capture or issuer publication. Apply existing validate_limits to local_limits; reject zero global/outbox/transition/receipt capacities using InvalidLimits. Global LP count beyond max_global_lps returns GlobalLpLimitExceeded{actual,limit}. Owned process count beyond local_limits.max_lps returns InvalidLimits. Empty owned set returns EmptyOwnedProcessSet. Unexpected owned LPs return ProcessSetMismatch{missing:vec![],unexpected:sorted}. Missing remote processes are permitted and never instantiated.

Preserve existing complete global topology checks: missing entries, unknown source/destination and duplicate neighbor/self-loop reject using existing errors. Sort validated neighbor vectors for canonical comparison. Preserve the entire global partition rather than rebuilding a subset partition.

current_authorities keys equal global LP keys exactly; mismatch returns AuthoritySetMismatch{missing,unexpected}, sorted. Every value must be Scoped: otherwise AuthorityModeMismatch(lp). Namespace mismatch returns AuthorityNamespaceMismatch{lp_id,expected,actual}. Zero/MAX namespace and ownership epochs are valid. emission_epochs keys exactly equal owned keys: EmissionEpochSetMismatch{missing,unexpected}; values equal configured owned epochs: EmissionEpochMismatch{lp_id,expected,actual}. No numeric authority winner or receiver-emitter advancement.

New exact OptimisticError variants:

```rust
OwnedModeRequired,
OwnedRuntimeJoinIncomplete,
EmptyOwnedProcessSet,
GlobalLpLimitExceeded { actual: usize, limit: usize },
AuthoritySetMismatch { missing: Vec<LpId>, unexpected: Vec<LpId> },
EmissionEpochSetMismatch { missing: Vec<LpId>, unexpected: Vec<LpId> },
AuthorityModeMismatch(LpId),
AuthorityNamespaceMismatch { lp_id: LpId, expected: u128, actual: u128 },
EmissionEpochMismatch { lp_id: LpId, expected: u64, actual: u64 },
UnownedLogicalProcess(LpId),
NativePeersNotSealed,
NativePeerRegistrationClosed,
NativePeerCoverageIncomplete { missing: Vec<LpId> },
NativePeerConfigurationMismatch,
NativePeerOwnershipOverlap(LpId),
StaleNativeAccountingAuthority { runtime_id: u64, recovery_generation: u64 },
AccountingRevisionExhausted,
VerifiedNativeAdmissionRequired,
NativeGroupCutRequired,
```

These intentionally expand the alpha exhaustive error enum; downstream exhaustive matches may need edits. No non_exhaustive or unrelated legacy error hides the expansion.

## Registration, sealing, closure and observable state

Self registers during construction at accounting revision0. A registry has at most one live issuer per owned scope and is bounded by global LP count. Registration validates health, owned mode, incoming liveness and existing registered liveness; compare complete config, then exact issuer retry. Exact repeated live registration succeeds unchanged, including after seal. Different registration after seal returns NativePeerRegistrationClosed. Before seal, overlapping ownership returns NativePeerOwnershipOverlap(first sorted overlapping LP). Failed validation never changes peers/revision/process state/tokens. New registration checks next revision before insertion and increments once.

Sealing validates live peers and exact complete disjoint global coverage. Missing LPs returns NativePeerCoverageIncomplete{missing:sorted}; initial missing registration is not a permanent failure. First successful seal increments revision once; repeated valid seal is a no-op. Dropped peer prevents sealing and later ownership-dependent operations with StaleNativeAccountingAuthority. A fresh issuer has a different runtime ID and cannot impersonate a registered issuer. No replacement or recovery activation after seal in this leaf.

close_initial_inputs requires healthy owned mode, sealed complete live peers. First close sets initial_open false and increments revision once; repeated valid close is a no-op. initial_inputs_closed reports !initial_open in both modes. Other inspections require owned mode; authority issuance additionally requires healthy state. Revision is checked u64 and never wraps; every mutation preflights before publication. Inspectors allocate no revision and do not change model state.

## Intermediate entry-point guards

Preserve ensure_healthy/Poisoned precedence on existing mutating methods. Owned schedule_initial checks initial_open first (existing InitialSchedulingClosed if closed), complete live sealed peers, owned event source (UnownedLogicalProcess for unowned), global destination/route/GVT/ancestry as applicable, then returns OwnedRuntimeJoinIncomplete without allocating incarnation, changing tokens/report/model or queuing/outboxing work. Remote destination is a valid global endpoint; it is not treated as a missing local process. This leaf does not claim successful owned scheduling yet.

Owned run_until_with_budget checks complete live sealed peers and horizon regression, then returns OwnedRuntimeJoinIncomplete before changing last_horizon, closing inputs or running handlers, even for budget0. Owned raw receive rejects LocalPreview with AuthorityModeMismatch(source LP), and Scoped with VerifiedNativeAdmissionRequired after health; it never authenticates a message from its public metadata. Legacy Scoped still returns ScopedAuthorityRequiresOwnedRuntime unchanged. Owned raw fossil_collect returns NativeGroupCutRequired unchanged after health. No complete-group implementation or native send/admission/retirement API is introduced in this leaf.

process_at/pending_events for remote unowned LP return None; state_token returns UnownedLogicalProcess for a known global unowned LP in owned mode. Owned state tokens remain valid through registry/seal/closure mutations because no process epoch changes. Snapshots include exactly actual owned model instances.

## Oracles and verification

Independent public fixtures use at least two genuinely disjoint real runtimes and separately retained model witnesses. Verify owned-only snapshot invocation and absence of remote models/tokens, full-width namespace/epoch preservation, every map/topology/capacity rejection, canonical topology acceptance, mismatched partition membership/lookahead refusal, incomplete then complete sealing, overlap rejection, issuer identity/liveness, drop before/after seal, exact retry idempotence and checked revision accounting. Compare report/process snapshots/queues/tokens/revision across rejected or guarded calls. Legacy issuer returns OwnedModeRequired. Closed scheduling returns InitialSchedulingClosed. Routing-success, exact authority indexes, rollback/retirement and complete-group GVT tests are mandatory later joins, not claimed by this guarded foundation.

Implementation verifies matching absolute Rust1.98.1 cargo test -p kairo-ecs-pdes --features pdes,time-warp --locked; cargo fmt --all --check; cargo clippy -p kairo-ecs-pdes --all-targets --features pdes,time-warp --locked -- -D warnings. Coordinator verifies independent fixture on matching absolute1.98.1 and1.76.0 with fresh targets and records source/tool/cache/log hashes and actual exits. Initial RED at accepted contract baseline fails exclusively absent new APIs; preserve it separately. Whole joined runtime just ci/hosted/normal-merge gates remain before final Track48 delivery.

Claim exact owned paths plus private artifact/target outputs, bound small hashed context, progressively read oversized native source with full file hash, prewrite/precommit checks, one commit per claim and immediate release. Future writer packet binds actual source/contract hashes and command paths at dispatch. This contract is not itself a worker launch.
