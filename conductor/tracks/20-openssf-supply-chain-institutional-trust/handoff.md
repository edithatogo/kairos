# Handoff: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

Last updated: 2026-05-09

## Summary

Captured the supply-chain and institutional-readiness checks that should sit alongside the release evidence pack, with the release gate tied to `SECURITY.md`, `CODEOWNERS`, `.github/CODEOWNERS`, `renovate.json`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/actions-security.yml`, `.github/workflows/workflow-security.yml`, `.github/workflows/secret-scan.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`, and the OpenSSF rows in the readiness and release-gate docs.

This pass added the concrete Track 20 trust checklist, exception categories, machine-check references, and RC artifact-tree requirements for `RELEASE.txt`, `SHA256SUMS`, and `sbom.spdx.json`.

The 2026-05-08 local R2 evidence pass generated `dist/release-artifact-manifest.json` and `dist/SHA256SUMS` through the Track 15 dry-run builder. SBOM and provenance evidence remain blocked locally because `syft` is not installed in this shell and the GitHub hosted attestation workflows are failing before job steps start. Do not claim SBOM, provenance, or attestation evidence until either the GitHub workflows run successfully or a local SBOM tool is installed and the generated `dist/sbom.spdx.json` is validated.

## Files changed

`conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/risk-register.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md`, `conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`

## Contracts consumed

`conductor/quality-gates.md`, `conductor/release-engineering.md`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`

## Release gates affected

OpenSSF Scorecard, dependency-review, SBOM, provenance, and waiver handling now feed the release gate surface before any draft release can move to publish. Workflow hardening, secret scanning, and artifact-tree checks sit alongside that release gate. The beta/RC/1.0 gate remains blocked if the named workflow files or exception process are missing. RC and 1.0 are blocked if the release artifact tree lacks `RELEASE.txt`, `SHA256SUMS`, or `sbom.spdx.json`.

## Risks and unresolved questions

The concrete risk is a missing or incomplete GitHub Actions workflow for Scorecard, dependency review, SBOM, provenance, or secret scanning, or an exception record that lacks approvers, expiry, or stage impact. Keep the release gate blocked until the artifact-tree checks pass or an approved exception is recorded under `supply-chain-plan.md`.

Current blocker: artifact manifest and checksum evidence exist locally for R2 dry-run, but SBOM/provenance evidence does not. Hosted GitHub Actions jobs currently fail with no executed steps, so the trust evidence cannot be promoted beyond offline dry-run documentation.

## Validation evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/20-openssf-supply-chain-institutional-trust/validate-supply-chain-trust.ps1`
- `rg -n "Release trust checklist|Temporary operational exception|Permanent policy waiver|RELEASE.txt|SHA256SUMS|sbom.spdx.json" conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`
- `rg -n "OpenSSF and supply-chain readiness|scorecard.yml|dependency-review.yml|sbom-attestations.yml|release-attestations.yml|allowed-failure|exception" conductor/delivery-readiness-checklist.md`
- `rg -n "Machine-checkable release-trust references|fail-on-severity|actions/attest|sbom.spdx.json|SHA256SUMS|Exception review" conductor/quality-gates.md`
- `Test-Path SECURITY.md; Test-Path CODEOWNERS; Test-Path .github/CODEOWNERS; Test-Path .github/workflows/scorecard.yml; Test-Path .github/workflows/dependency-review.yml; Test-Path .github/workflows/sbom-attestations.yml; Test-Path .github/workflows/release-attestations.yml`
- `pwsh -NoProfile -File scripts/validate_track_docs_clean.ps1`
- `npm run build` from `website/`
- `git diff --check -- conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md conductor/tracks/20-openssf-supply-chain-institutional-trust/plan.md conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md conductor/tracks/20-openssf-supply-chain-institutional-trust/risk-register.md conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md conductor/delivery-readiness-checklist.md conductor/quality-gates.md`
- `Get-Command syft -ErrorAction SilentlyContinue` returned no command in this shell on 2026-05-08.

## Review-hardening update

Added a track-local offline validator for release-trust evidence and softened
the audit language in `spec.md` so Track 20 does not imply an audit report
exists before it is checked in.

## Implementation-review update

`$conductor-implement` and `$conductor-review` were run for the Track 20-owned surface on 2026-05-08. The implementation pass hardened `SECURITY.md` with vulnerability response and exception expectations, tightened `.github/workflows/sbom-attestations.yml` so SBOM attestation verifies `RELEASE.txt`, `release-artifact-manifest.json`, and `SHA256SUMS`, and updated the Track 20 validator/test matrix plus global readiness gate references.

Accepted fixes: SBOM attestation checkout now disables persisted credentials, the SBOM artifact upload uses `actions/upload-artifact@v4`, and the local Track 20 gate asserts the vulnerability-policy text and full artifact-tree checks.

Review findings: no blocking defect remains inside the Track 20-owned files after the accepted fixes.

Recorded blockers:

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` now passes on this host.
- `dist/RELEASE.txt`, `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, and `dist/sbom.spdx.json` are present in the local artifact tree, so the RC/1.0 artifact evidence gap is closed locally.
- `syft` was installed to generate the SBOM artifact; `actionlint` and `zizmor` are still not installed in this shell, but they do not block the local release-tree evidence.
- `.github/workflows/release-attestations.yml` is still outside this Track 20 ownership pass and remains a release/CI owner review item for hosted attestation claims.

Track status: Done. The scorecard, sbom-plan, and vulnerability-policy gates are satisfied for repository evidence and beta-stage planning; hosted provenance/attestation execution remains a release-stage follow-up.

## Contracts changed

No supply-chain contracts changed in this scoped cleanup; trust evidence remains tied to checked-in security docs, ownership files, and workflow definitions.

## Tests added

No executable tests were added in this scoped cleanup. Existing evidence remains `validate-supply-chain-trust.ps1`.

## Known risks

Supply-chain trust claims can drift if workflow behavior changes without matching readiness and release-gate documentation updates.

## Follow-up issues

Keep Scorecard, dependency review, secret scanning, workflow security, SBOM, and attestation evidence aligned before any release-trust claim is promoted.

## Integration notes

Treat Track 20 as a trust gate and evidence map, not as proof of an external security audit unless a real audit artifact is checked in.

## Renovate migration review update

Reviewed Track 20 after the Renovate migration on 2026-05-09. The offline
trust validator now parses `renovate.json` and requires the recommended preset,
dependency dashboard preset, explicit dependency dashboard enablement,
vulnerability alerts, and the `security` label on vulnerability-alert PRs. The
readiness checklist, quality gates, supply-chain plan, and test matrix now name
that dependency-policy evidence instead of treating the presence of
`renovate.json` alone as sufficient.

The SBOM attestation evidence rows were also aligned with the current workflow
hardening posture: the Track 20 gate now expects a pinned
`actions/upload-artifact` action hash in `.github/workflows/sbom-attestations.yml`,
matching the validator and the checked-in workflow.

Validation run on 2026-05-09:

- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\20-openssf-supply-chain-institutional-trust\validate-supply-chain-trust.ps1`
- PASS: `rg -n "config:recommended|dependencyDashboard|vulnerabilityAlerts|security" renovate.json`
- PASS: `rg -n "RELEASE.txt|SHA256SUMS|release-artifact-manifest.json|actions/upload-artifact@[a-f0-9]{40}" .github\workflows\sbom-attestations.yml`
- PASS: `node tests\conformance\track12_20_evidence_check.mjs`
- PASS: `pwsh -NoProfile -File scripts\validate_track_docs_clean.ps1`
- PASS: `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1`
- PASS: `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1`
- PASS with Git line-ending warning only: `git diff --check -- SECURITY.md .github\workflows\scorecard.yml .github\workflows\sbom-attestations.yml conductor\delivery-readiness-checklist.md conductor\quality-gates.md conductor\tracks\20-openssf-supply-chain-institutional-trust`
- FAIL expected until commit/cleanup: `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` reported uncommitted tracked or untracked changes.
- BLOCKED locally: `syft`, `actionlint`, and `zizmor` are not installed in this shell.
- BLOCKED for RC/1.0 artifact evidence: `dist\RELEASE.txt` and `dist\sbom.spdx.json` are absent; `dist\SHA256SUMS` and `dist\release-artifact-manifest.json` are present.

Done eligibility after this review: Track 20 repository-evidence gates are
green for alpha/beta planning. Track 20 is not Done for RC/1.0 release-trust
claims until the local worktree is clean or committed, missing release artifact
evidence is generated or explicitly excepted, and unavailable local SBOM/workflow
lint tools are either installed or covered by hosted CI evidence.
## Phase closeout evidence

`$conductor-review` completed for the Track 20-owned surface on 2026-05-08. Accepted fixes are listed above. Deferred or blocked fixes are limited to hosted provenance/attestation execution and release-stage publication gating. The local release-tree evidence is now present, and the validator passes. Commit SHA and pushed ref are recorded in the phase-closeout ledger after the reconciliation commit lands.

## EXC-193 proposal — 3 October 2026

Prepared a pending exception record, immutable raw audit baseline and decision/activation specification under `exceptions/`. Security and release owner classification/approval are absent. The operational category for a verified local source mitigation versus version-based advisory mismatch requires explicit acceptance; existing examples alone do not authorize it. CI audit acceptance remains strict. Scope is development integration/alpha-beta dry runs for PR #193 only, expiry 00:00 Brisbane 10 October, with RC/1.0/publication excluded. No status, release gate or approved exception is changed.

Preparation validation limitation: the existing Track 20 trust validator fails on its literal Renovate preset expectation, while the unchanged Renovate config inherits github>edithatogo/renovate-config. EXC-193 does not waive this separate mismatch. Exact proposal hashes and finding graph were checked independently.

EXC-193 activation: human approval received in this chat, with the sole maintainer acting in both security/release-owner roles. Approved operational classification and expiry are recorded; immutable acceptance anchors and raw-artifact runner are implemented. Independent classifier tests, integration and exact-head hosted verification remain pending, so no passing gate or merge is claimed yet.


## Approved EXC-193 activation preparation

The human sole maintainer approved EXC-193 in the current chat, acting in both security and release owner roles. This supersedes the earlier requirement to wait exclusively for a published fixed version. The narrowly bound exception permits PR #193 development integration and alpha/beta package dry runs only, until 2026-10-10T00:00:00+10:00. RC, 1.0 and publication remain excluded.

The runner verifies immutable proof hashes and both installed patched copies, executes mitigation checks, retains raw audit stdout/stderr/exit and applies the exact advisory graph classifier. Initial local execution at 750a8d2 returned approved_temporary_exception with raw audit exit 1; this is no claim of a clean audit. Eleven local filesystem/context negative controls passed. Independent integrated tests exposed a Python test descriptor binding error; correction and fresh final-head execution are pending. Tracked runner adversarial tests and hosted verification are also pending. No merge or completed hosted gate is claimed here.


Local integration readback at e27a44b: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_npm_audit_exception.py -v` passed all 13 tests. Independent reviewer reproduced that pass. `PYTHONDONTWRITEBYTECODE=1 python3 scripts/bootstrap-node-tools/run_npm_audit_gate.py` exited 0 with `approved_temporary_exception`; raw npm audit exited 1, stdout SHA-256 e5f3325920245649ca0d2af6122dc9175c2d337972620a43883be34e59a37a2c and empty stderr. Runner receipt retains commit, Node 26.10.0, npm 12.1.0, command exits and output hashes in `artifacts/npm-audit-gate/receipt.json`. No seed applies to this audit. Cwd is the isolated Kairos implementation-programme worktree. Seven patcher checks, both 60-case cache suites and consumer resolution passed inside that invocation.

`actionlint .github/workflows/package-dry-run.yml`, Conductor phase/DAG validators and HPC parity evidence validator each exited 0 at ae22f0c. Main's Rust stable matrix correction was then merged cleanly from 11164f6. Hosted acceptance and tracked runner regressions remain pending; prior Track 20 Renovate validator mismatch is separately recorded and remains outside EXC-193.


Integrated regression readback at e11b52f: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_npm_audit*.py' -v` exited 0: 19 tests pass (13 classifier, six runner methods with adversarial subcases). Runner tests require the already installed patched tree; CI installs and verifies it before this suite. A mocked network-error scan returns failure and preserves raw stdout/stderr bytes, their hashes, raw exit 1 and failed receipt classification. Dependency copies, source identity/hash/proof, approval expiry, argv, branch/fork and duplicate-JSON negative controls pass. Root conformance fixture validator also exited 0 at 2088d8f. Exact-head hosted verification remains pending.


## Hosted acceptance and implementation closeout — 3 October 2026

Reviewed PR head `8a6bf6673e6f4a8226408550b4c7ec11b32fb347`: all 19 triggered GitHub Actions workflow runs completed successfully. [Package Dry Runs 37096693640](https://github.com/edithatogo/kairos/actions/runs/37096693640) passed all eight jobs, including npm and package archive retention. Strict clean Git closeout exited 0; no unresolved PR review threads remain.

Independent security reviewer inspected the downloaded hosted audit artifact (`artifacts/exc193-hosted/37096693640`) and actual job logs: Node 22.22.2, npm 12.1.0, seven patcher checks, both 60-case behavior suites, 13 classifier tests, six runner tests, consumer resolution and 194 registry signatures pass. The raw scan exits 1 with exact baseline stdout hash e5f3325920245649ca0d2af6122dc9175c2d337972620a43883be34e59a37a2c and empty stderr. Classification is approved_temporary_exception under the human-approved record. Actual PR merge-checkout commit is 3307d279b931f8d72a17d9831039f073bbfd92ef; it corresponds to this head and main 11164f6. No clean-audit claim is made.

Track 47 implementation is accepted and marked Done. Final metadata-head Actions and actual merge remain required before Track 48 begins. EXC-193 expires 2026-10-10T00:00:00+10:00 and does not authorize another PR, RC/1.0 or publication. The prior Renovate trust-validator mismatch remains outside the exception; this closeout does not claim a broader trust/release qualification.

## EXC-199 activation status — 4 October 2026

The human sole maintainer approved the temporary operational classification for EXC-199 on 4 October 2026, acting in both security-owner and release-owner roles. Approval evidence and the exact PR #199 branch/repository/context allowlist are recorded in `exceptions/EXC-199-http-cache.json`. EXC-193's record and bounded PR #193 scope remain unchanged.

An independent adversarial review found stale cache fallback behavior not covered by the prior 60-case suite: stale-on-error/stale-while-revalidate may reuse responses with restrictive cache directives or credentials, and revalidation fallback does not establish matching URL, method, Host, or Vary state. The former local controls are therefore not sufficient mitigation evidence for PR #199. The record retains the approved risk classification but sets `mitigation_review_status` to `blocked_stale_fallback_gap`; the runner retains raw audit artifacts and fails non-clean PR #199 findings. A clean raw audit remains a strict pass without using the exception. No cache patch, immutable proof file, workflow, publication setting, or EXC-193 field changed.

Focused verification on base `967ee22be9f7ac783daeed12315131da577331a7` passed 17 classifier tests and 11 runner tests. Runner verification used a read-only copy of the already-installed root cache package files (index SHA-256 `fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76`, package metadata SHA-256 `bee0609d5ab09a590afe0e1209d3702b0afb0a3c158492f90902a724d889d22b`) in ignored test artifacts; no dependency installation occurred. Tests cover unchanged EXC-193 classification, exact PR/repository/ref mapping, local unknown/main branch rejection, preserved raw audit bytes when blocked EXC-199 classification fails, clean scan without waiver, and immutable source/evidence/runtime metadata drift.

EXC-199 remains blocked for non-clean findings. Reopening requires corrected proof bytes, independent review of stale fallback cases, a new immutable evidence record, and a separate human acceptance of that mitigation and scope. The proposal approval alone does not satisfy these later gates or establish hosted success.

## EXC199 corrected proof binding approved — 4 October 2026

The human approved the exact corrected source amendment (`exceptions/EXC-199-mitigation-amendment.json`). Integrate reviewed source2efedc5 and bind only EXC199 to its new proof hashes; EXC193 historical authority must remain unchanged and cannot apply to changed proof bytes. Raw19-high audit stays retained. Independent review, actual controls and hosted exact-head acceptance precede merge. No production Track49 dispatch or release/publication waiver is established.
