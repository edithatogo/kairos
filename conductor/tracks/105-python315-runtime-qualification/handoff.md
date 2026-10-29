# Handoff

## Qualified runtime pin repair — 29 October 2026

Integration chat `01a18344-cfb8-7711-8c38-fef1052b9fcf` owns this bounded
repair in `/Users/doughnut/.codex/worktrees/kairos-track107-reconcile/kairos`,
branch `codex/pr332-runtime-pin-repair-20261029`, based on PR #332 head
`51e4b1601baf728ef7ba5ca7c90ded891d666323`. Scope is the Python inputs in
`.github/workflows/` and this note; no native objects, authentication, other
environments or runtime promotion are involved. Earlier assignments remain
history. Next operation is focused runtime/workflow validation and an ordinary
source commit to the existing PR branch.

Renovate's grouped action update also changed Python inputs to 3.15.2 without
updating the repository's qualified 3.15.1 bootstrap, locks and consumers.
Hosted archive inventory, Windows Conductor and macOS session-guard jobs failed
at the explicit bootstrap version check. These inputs now retain the existing
qualified 3.15.1 profile; action updates remain. Promoting the newer 3.15.2 patch
requires isolated qualification and coordinated alignment of the active guards,
locks and consumers. No environment was replaced and no gate was relaxed.

The same packet aligns the two existing exact action-pin assertions in
`scripts/validation/validate-track13-metadata.mjs` and
`scripts/validation/validate-publication-readiness.mjs`. GitHub readback confirms
taiki-e/install-action v2.88.0 resolves to `86f7a0ffb25f40114606501a311ebcfaf882ea32`,
rust-cache v2.9.3 to `d1527c8f67ef9af357290475d4af4b0fab8f12e0`, and the PyPI
publisher update `7245906fb61fa33a13554f9d5fb856fe61758086` is the upstream
certifi/sigstore dependency update. Exact pins, the main-only publication job,
protected publication environment and all other policy checks remain enforced.

Focused local validation passed with repo-local Python 3.15.1: nine local runtime
tests and three runtime-policy tests. Actionlint, Track 13 metadata validation,
publication-readiness validation and Conductor phase gates passed; whitespace
checks passed. Hosted checks must verify the corrected inputs before merge.

Hosted follow-through also found an outdated offline cloud CLI assertion and a
missing changelog entry for the release-workflow updates. The cloud validator
now recognizes the existing typed `Commands::Run`, `Checkpoint` and `Resume`
dispatch instead of the removed string-match implementation. The CLI and
provider behavior are unchanged. The required changelog records the CI/release
maintenance; its policy remains enforced.

Local follow-through passed `cloud/validate_cloud_hpc.py` on repo-local Python
3.15.1 and all five `kairo-ecs-cli` process tests on Rust 1.99.1, including real
run/checkpoint/resume JSON results with quoted paths. Cached build output is in
`/Volumes/PortableSSD/codex-builds/kairos-arrow-publication-20261029`.

## Current policy and successor evidence — 24 October 2026

The human's revised dependency policy prefers latest suitable versions and permits
necessary alternates with concrete reason/scope and actual qualification, without
further permission. It supersedes earlier no-alternate instructions in the dated
entries below; those entries remain historical evidence. The accepted default
3.15.1 guards, locks, containers and environments remain intact. No alternate
profile is adopted merely to apply this policy.

PRs #297/#301/#302 integrated the accepted runtime and tooling repairs. Native
inventory accepted/read back all 51 expected packages; this proves package presence,
not graph edges/extras. Both certifi alerts #98/#99 are now natively fixed at
2026-10-23T18:20:14Z (24 October in Brisbane), without dismissal or rerun.
Authenticated Codecov configuration now identifies project coverage, flags and
components as "Available with Pro Plan" despite GitHub PUBLIC visibility. The
processed report and rust-core flag exist, and the effective YAML requests project
status. See [native readback](codecov-entitlement-readback.json). This is a provider
entitlement/reporting restriction, not evidence for changing coverage thresholds
or repeating uploads. No purchase, permission change or external support message
is authorised by this receipt; genuine project status remains an open gate.

Provider quality follow-through and this policy update remain separately scoped;
no Polars/FMI source, engine pin or other-owner environment is transferred.

## Documentation dependency gate repair — 24 October 2026

PR #303 exposed new upstream advisories at the hosted audit gate. Zod
GHSA-mrhx-c7xc-m3w5 affects both documentation locks; the website audit also
reports Undici GHSA-4qv8-2p9j-2685, GHSA-f72v-6w45-q472 and
GHSA-xjj6-f33q-929c. The candidate updates Zod to 4.6.7 and Undici to 8.12.3.
Mermaid CLI remains 11.17.0; its supported Puppeteer 25.16.0 peer chain supplies
Chromium BiDi with Zod 4 support, avoiding a forced major-version override on the
old Zod 3 consumer. Other dependency pins remain unchanged except that supported
browser chain. Both candidate npm audits report zero vulnerabilities; five
pin/integrity regression tests pass. Installed renderer/build qualification now passes: five Mermaid/runtime tests,
old Zod negative control, site build and quality checks. See
[local receipt](docs-advisory-qualification.json). Hosted qualification and native
alert closure remain required before acceptance. The first fresh CI run caught
the exact website-lock binding and a separate bootstrap Undici copy; both are
now repaired with [targeted follow-through evidence](docs-advisory-ci-followthrough.json).
The real private npm tree passes its strict zero-vulnerability gate, with exact
source/CLI/Node hashes retained and no legacy exception expansion. The dependency
identity control also rebinds to the exact new lock hash; the private HCS archive,
payload hashes, identity match and two-advisory correction remain unchanged. All
five identity tests pass, including wrong package/path/version/severity/advisory,
source drift and later-page high findings. No generic advisory exemption is added. The LLMS renderer
lock/graph oracle is also requalified: an independent before/after comparison
proves only Zod/Undici version, registry URL and integrity records changed, and
reproduces the old normalized graph hash before updating its expected hashes.
All four renderer contract tests pass; forbidden-package and renderer semantics
checks remain intact. This repair does not alter
Polars, engine pins or simulation behaviour.

## Historical execution entries

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

## Current handoff structure — 29 October 2026

## Summary

Python runtime qualification and CI guard alignment.

## Files changed

Runtime guards, CI workflows and packaging helpers; detailed historical packets follow.

## Contracts consumed

Selected Python profile, hash-locked dependencies and repository-local prefix contract.

## Contracts changed

The current frontier selects Python 3.15.2; prior receipts retain their original versions.

## Tests added

Runtime/prefix negative controls, packaging and workflow contracts.

## Known risks

Hosted/platform acceptance remains distinct from local qualification.

