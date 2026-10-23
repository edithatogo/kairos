# Handoff

## Current state

Track registered to reconcile and finish existing candidate work; no phase accepted. The [dated qualification snapshot](prior-qualification.md) and [hash manifest](prior-evidence.json) preserve prior work. Kairos catalogue PR #295 is already merged independently. The active conversation goal is armed and covers full migration, not just this registration.

## Next step

Verify inventory and candidate equivalence against current main, then adopt qualified changes under fresh exact-path claims. Do not rerun unchanged expensive qualification solely for tracking. Do not advance the CareOps Kairos pin.

## Evidence boundary

Candidate local success is not current-head, hosted, main or environment-handover acceptance. Record actual blockers and preserve existing implementation owners. No emails or native service changes.

## Follow-up issues

Reconcile the retained candidate, remaining platform/native checks and current
implementation-ledger/Track 52 metadata failures without changing unrelated
acceptance records. Complete real current-main and terminal-environment proof.

## Integration notes

This registration is local planning; runtime integration remains pending. Use
required PR/checks, preserve the catalogue repair and keep the existing
Polars/FMI branches and CareOps runtime pin untouched.

## Phase closeout evidence

Every phase must record `$conductor-review`, accepted fixes, rejected or
cross-owner work, exact command/cwd/toolchain/input/output hashes and exit
statuses, commit SHA, pushed ref and next-phase decision. No phase is closed by
this registration. Run
`pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
after the applicable source is committed; retain any pre-existing ledger failure
as unresolved rather than accepting it silently.

## Registration review

Planning-only registration passes phase, DAG, no-skip and whitespace checks. See [exact-command receipt](registration-evidence/receipt.json). Previous qualification remains retained evidence, not current-main runtime acceptance.

## Candidate reconciliation

The original toolchain worktree remains byte-for-byte preserved. Its saved patch applied cleanly to the current-main-based branch; 63 changed source files match the qualified candidate exactly. Only `conductor/quality-gates.md` also contains the already merged catalogue repair. Three Python policy cases, unchanged no-skip catalogue validation and whitespace pass. See [reconciliation receipt](candidate-reconciliation/receipt.json). Earlier native/binding/security qualification remains retained with its original scope; repo-local workflow binding and current-main hosted/platform/handover proof remain open. No Polars/FMI/GPU/MPI implementation files or CareOps pin changed.

Raw prior logs retain original tool-emitted whitespace and their hashes; the source whitespace check explicitly excludes only `conductor/toolchain-qualification-20261023/*.log`. The first all-file whitespace check reported those raw-output lines; the logs were preserved rather than rewritten.

## Local runtime binding and focused testing

Python workflow jobs verify repository-local `.venv` version/prefix after checkout, then export its path for later commands and subprocesses. The trusted archive preflight creates an isolated verified local interpreter before validating pins, preserves validation before producer API access, and recreates the environment after checkout cleanup. Guard tests execute that preflight without source/API access. Existing environments are never replaced. Shell-only binding aggregation no longer installs an unused Python. Eight runtime cases plus 27 archive/mainline/security workflow cases and actionlint pass; see [local receipt](local-runtime-evidence/receipt.json). Latest hash-locked bootstrap dependencies install on explicit local 3.15.1.

AGENTS/workflow preserve focused native selectors, per-worktree pytest cache semantics and full milestone gates. pytest-difftest 0.3.0 remains blocked by the reported PyO3 3.14 support ceiling; no repeated failed build, downgrade or bypass. Retries default to zero. Original native qualification is retained, while hosted/current-main/required-platform/handover proof remains open.

## Active default cleanup

Track 105 now reports In Progress consistently. Active website/template/release checklist claims require Python 3.15.1; dated receipt evidence stays intact. The unused pre-tomllib developer parser fallback is removed, and the actual packaging helper resolves all 27 real workspace manifests on 3.15.1. The unrelated existing archive manifest tests (2) also pass; they are not claimed as coverage of this helper. POSIX venv creation uses supported symlinks, preserving the canonical executable provenance expected by the existing Syft validator; Windows retains copied launchers. Eight runtime cases and phase/DAG checks pass. See [active-default evidence](active-default-evidence/receipt.json).

## Container and platform wiring

Docker creates `/work/.venv`, verifies exact Python/prefix and selects it for the telemetry plugin. Devcontainer, devbox and bootstraps use the common version/prefix guard. Hosted session jobs run the focused refusal suite on Linux/macOS; Conductor jobs now probe the real environment on Windows/Linux; the existing Docker build lane additionally probes actual container version/prefix. No local container/Windows execution is claimed. Eight guard cases, actionlint and shell syntax pass. The cloud static validator exposes a stale bare-Alpine-image assertion; it must be updated/tested before PR. See [platform wiring receipt](platform-wiring-evidence/receipt.json).

## Container contract correction and active inventory

The static cloud validator now requires the exact digest-pinned Python 3.15.1 runtime image and local `/work/.venv` contract, instead of the obsolete bare-Alpine assertion. Negative fixtures reject an old image, missing venv creation and wrong runtime PATH. The actual cloud validator and nine focused runtime/container cases pass. Root `.venv` is ignored explicitly, and a fresh Git fixture proves guard creation leaves tracked source clean. See [cloud correction receipt](cloud-validator-evidence/receipt.json) and [active-use inventory](inventory.md). Hosted/main/platform/handover proof remains open.

## Prepublication source-digest review

The new-commit gitleaks scan identified one source fingerprint in the runtime receipt: the recorded SHA256 of the public secret-scan workflow. It was recomputed against its recorded Git commit and matches. Only the exact historical finding is recorded, with no rule/path-wide exception. The current receipt retains identical path/hash data as explicit records, so a future squash does not repeat the credential-like key layout. The rescanned new branch history passes; see [security receipt](prepublication-security/receipt.json). Hosted full security remains required.

## Actual main handover and remaining provider/CI work — 2026-10-24

PR #297 merged normally as `14c100168a87dcaa02d13ca2967af69a959a76cd` after every actual final-head check passed. One stalled R setup was canceled after its retained log showed slow Ubuntu mirror downloads; exactly that read-only setup job passed one bounded retry. The clean primary main was fast-forwarded with the identical reviewed tree, a fresh repo-local final3.15.1 environment was created without replacement, and9runtime/3policy/21binding cases plus metadata/devcontainer/clean-ledger checks passed. The first mistyped runtime selector matched zero tests and was explicitly credited as no proof before the exact files ran.

Main-only checks exposed the publishing docs evidence omission and12Scorecard dependency advisories. The candidate corrects the publishing workflow, adds non-publishing PR qualification, updates the active binding/template/docs-tool dependencies and retains strict gates. [Main repair evidence](main-ci-repair.json) records first findings; [renderer evidence](mermaid-runtime-evidence.json) proves that metadata overrides alone were insufficient, then qualifies the actual patched public-core runtime with4controls and all15tracked diagrams. The existing locked esbuild is reused; no new bundler dependency or third-party source fork. Retire the thin preparation after an upstream prebundle is actually qualified.

The hosted Dependabot graph updater explicitly rejects Python>=3.15.1,<3.16 (only3.10–3.14 supported). No downgrade or feature disabling is accepted. A separate [source-bound inventory caller](dependency-inventory-evidence.json) reuses the independently qualified shared adapter at MAIN`cb62ad8`, installs all4existing locks on3.15.1, preserves privacy, and isolates trusted-main submission from read-only verification. All local caller/provenance/negative controls pass. This is package inventory, not graph/extras evidence, and no API submission has been claimed. Accepted native responses, async graph package readback, final-head/main checks and remaining provider status still gate this Kairos track.

CareOps Sim independently accepted its own runtime at main`43ea690` through PRs178/179; its engine pin and domain acceptance remain separate. Polars/FMI/private source and environments remain preserved. This Kairos track remains In Progress; neither the goal nor full migration is marked complete while provider/main acceptance remains open.
