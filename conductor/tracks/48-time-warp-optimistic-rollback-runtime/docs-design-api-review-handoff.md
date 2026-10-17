# Handoff request: Track 25 update of `docs/design/track48-api-review.md`

Raised by: Track 48 closeout reconciliation, 17 October 2026
Addressed to: Track 25 (`api-governance-agent`), owner of `docs/design/`

## Why this is a handoff and not a Track 48 edit

`conductor/tracks.yaml` lists `docs/design/` as an owned path of Track 25 only.
Track 48's owned paths are `crates/kairo-ecs-pdes/`, `docs/pdes/`,
`benches/pdes/` and `conductor/tracks/48-time-warp-optimistic-rollback-runtime/`.
The Track 48 plan's open review-fix item asks for the API assessment to appear in
both `review-report.md` and `docs/design/track48-api-review.md`, so Track 48
completes its half and requests the Track 25 half rather than writing across the
ownership boundary.

This is a status/documentation boundary only. No API, ABI, schema, binding or
policy change is requested, and no compatibility waiver is implied.

## Current gap

`docs/design/track48-api-review.md` was last extended with the "Owned constructor
and root-routing local acceptance — 4 October 2026" section. It does not describe
the handler-owned execution surface that is now merged on `main` via
`55fd635` (#207), which introduced reviewed-cut native source `35914c3`.

## Content Track 25 should add

1. **Affected surface.** Same Rust root `crates/kairo-ecs-pdes`, still outside the
   protected-surface inventory, still an opt-in `time-warp` alpha preview. State
   plainly that no protected root, C ABI, Arrow schema or binding root changes.
2. **Compatibility classification.** Additive opt-in alpha surface with additional
   public error variants, so exhaustive downstream `OptimisticError` matches may
   need updates. No migration adapter or package pin change. No `non_exhaustive`
   attribute is added to mask this.
3. **Owned execution and retirement APIs.** `run_owned_until_with_budget`,
   `ready_native_sends`, `admit_native`, `acknowledge_native_admission`,
   `pending_native_retirement_requests`, `receive_native_retirement`,
   `applied_native_retirements`, `acknowledge_native_retirement`,
   `fossil_collect_native_group` and `accounting_snapshot`. The readable source is
   `docs/pdes/owned-optimistic-runtime.md` plus
   `owned-handler-retirement-api.md` in this track directory.
4. **Reporting behavior change worth calling out.** A blocked local replacement now
   consumes capacity and is included in public pending-event *and* pending-positive
   totals, and a failed native group cut reports every already-poisoned participant
   in sorted deduplicated order while preserving the originally selected failing
   runtime. Both were found by review and are consumer-visible in reports.
5. **Maturity and limits.** All native tickets, acknowledgements, retirement proofs
   and cut participants bind actual **process-local** actors. They are not
   wire-authenticated and cannot be serialized into remote proof. Durable restart,
   authenticated external accounting, MPI/gRPC acceptance, migration and
   distributed GVT remain separate implementation and evidence gates. No release
   or publication claim follows from this preview.
6. **Track status.** Track 48 remains In Progress pending live distributed
   acceptance. Do not record Done, a stable promotion, a protected-root enrollment,
   or a release-manager signoff.

## Supporting evidence already in the repository

- Merge commits `786f50b` (#199), `1afe522` (#204), `55fd635` (#207).
- `owned-native-acceptance-20261004.md` for the accepted native leaf, compiler
  binding and failed-attempt distinctions.
- `owned-root-routing-evidence.md` and `codec-bridge-evidence/` for the earlier
  leaves.
- `benches/pdes/evidence/track48-owned-9bf478e/`,
  `benches/pdes/evidence/track48-hosted-fixes-8b56c94/` and
  `benches/pdes/evidence/track48-reviewed-cut/` for hash-bound raw receipts.

No new code is required for this handoff, and Track 48 takes no dependency on when
it is actioned.