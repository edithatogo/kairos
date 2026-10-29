# ADR: Preserve CareOps trajectories while backporting required APIs

Status: candidate for local CareOps R10.3 acceptance; 29 October 2026.

## Evidence and decision

CareOps at `87439c3` consumes SplitMix64 seeded trajectories. Current Kairos
main `8eafd77` provides the reviewed scheduler preview and experimental staged
allocation APIs, but also adopts ChaCha8. The existing CareOps workspace gate
fails five independent seeded-vector tests with that broad pin. The parent
R20/R21 RNG adoption remains separately scheduled after Polars.

Backport the exact reviewed core/DES API source and existing tests/fixtures from
`8eafd77` onto accepted Track 70 `5692e6a`. Only 28 additive scheduler lines and
the new staged runtime/export are added to existing production source. RNG,
state/types, dependencies and lock remain byte-identical to the Track 70 base.
This branch preserves the old generator; it introduces no second generator or
new legacy backend. The later intentional RNG migration supersedes this bridge.

## Compatibility and review

The API addition follows merged Track 106 and Track 107 review, including
`docs/adr/adr-track107-careops-staged-flow.md` and its API review at `8eafd77`.
CareOps must explicitly migrate FlowRuntime calls to StagedResourceRuntime;
the removed broad FlowRuntime contract is not restored. Both APIs remain
experimental. The copied fixture JSON is retained for source provenance; no
new manifest registration or broad cross-language fixture claim is made.

Objection: passing new API tests can conceal a changed RNG or dropped oracle.
Response: compare source blob identities, retain every consumer assertion, run
all parent workspace tests and compare seeded benchmark summary hashes with
the frozen R10.2 baseline. No oracle is rebased or skipped.

Objection: an older baseline may carry unrelated dependency advisories.
Response: audit the actual parent lock/feature graph; no Arrow/Polars dependency
is linked by the current consumer. This is not full workspace security, Polars
runtime migration, package publication or a security-qualified release. Later
R11-R14 dependency closure must be qualified before activating those paths.

Rollback is the parent gitlink/lock/test revert in a fresh output directory.
Preserve historical outputs. Hosted checks, upstream publication and broader
CareOps history integration remain separate from local compatibility evidence.
