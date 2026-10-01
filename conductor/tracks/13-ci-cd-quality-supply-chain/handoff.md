# Handoff — 13 CI/CD, Code Quality & Supply Chain

## Summary

CI and supply-chain gates now cover core Rust quality, binding smoke workflows, conformance fixture validation, benchmark smoke checks, dependency policy, and workflow security checks. This pass also wires the Track 07-13 hardening validator into the conformance workflow and keeps release/registry actions out of local validation.

The latest pass tightens Track 13 metadata validation so every checked-in `.github/workflows/*.yml` file must have an explicit workflow name, trigger block, and top-level permissions block. It also checks that both `ci-policy.yml` and `workflow-security.yml` inventory every workflow, closing the gap where newer workflows could exist without being named in the policy gates.

A dedicated offline supply-chain gate now lives at `scripts/validate_track13_supply_chain.ps1`. It runs the existing Track 13 metadata validator, verifies `cargo metadata --no-deps --format-version 1`, and executes cargo-deny advisory/source checks plus `cargo audit` only when those tools are installed locally. Missing advisory tools are reported as skipped instead of failing the local gate.

## Files changed

Current R2 slice:

`.github/workflows/conformance.yml`
`.github/workflows/benchmark-smoke.yml`
`renovate.json`
`.github/workflows/actions-security.yml`
`.github/workflows/codeql.yml`
`.github/workflows/ci-policy.yml`
`.github/workflows/ci-skip-guard.yml`
`.github/workflows/workflow-security.yml`
`deny.toml`
`rust-toolchain.toml`
`scripts/validation/validate-track13-metadata.mjs`
`scripts/validate_track13_supply_chain.ps1`
`conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md`
`conductor/tracks/13-ci-cd-quality-supply-chain/handoff.md`

Earlier Track 13 pass:

`.github/workflows/ci-core.yml`
`.github/workflows/docs.yml`
`.github/workflows/ci-bindings.yml`
`.github/workflows/package-dry-run.yml`
`.github/workflows/release.yml`
`.github/workflows/release-attestations.yml`
`.github/workflows/benchmarks.yml`
`.github/workflows/fuzzing.yml`
`.github/workflows/docs-quality.yml`
`.github/workflows/nightly.yml`
`scripts/validate_conductor_setup.ps1`
`conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md`

## Contracts consumed

`conductor/workflow.md`
`conductor/contracts/conformance-contract.md`
`conductor/contracts/package-release-contract.md`
`conductor/contracts/supply-chain-contract.md`

## Contracts changed

None.

## Tests added

Workflow existence checks, Rust metadata checks, cargo-deny/audit gates, Dependabot coverage, CI skip guard checks, offline conformance checks, Track 07-13 hardening, benchmark smoke checks, explicit workflow permissions checks, and dynamic workflow inventory checks are documented in `test-matrix.md`.

The new offline supply-chain gate adds a local PowerShell entrypoint for the Track 13 metadata validator plus `cargo metadata`, with advisory scanners skipped cleanly when unavailable.

## Validation

- `node scripts/validation/validate-track13-metadata.mjs` passed on 2026-05-08.
- `node tests/conformance/track07_13_hardening_check.mjs` passed on 2026-05-08.
- `node tests/conformance/track12_20_evidence_check.mjs` passed on 2026-05-08.
- `node scripts/validation/validate-track13-metadata.mjs` passed on 2026-05-07.
- `pwsh -NoProfile -File scripts/validate_track13_supply_chain.ps1` passed on 2026-05-08.
- `node tests/conformance/track07_13_hardening_check.mjs` passed on 2026-05-07.
- `node tests/conformance/track12_20_evidence_check.mjs` passed on 2026-05-07.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` passed on 2026-05-07.
- `git diff --check -- .github/workflows/ci-policy.yml .github/workflows/workflow-security.yml .github/workflows/codeql.yml scripts/validation/validate-track13-metadata.mjs conductor/tracks/13-ci-cd-quality-supply-chain` passed on 2026-05-07 with only line-ending normalization warnings.

## Known risks

Binding, package, benchmark, and fuzz jobs require checked-in manifests or harness directories before their workflow jobs can run successfully. Workflow inventory drift is now a validator failure rather than a manual review-only risk.

## Integration notes

Tracks 07-13, 14, 15, 20, 25, and 28 consume these gates directly. Keep future workflow changes aligned with `conductor/quality-gates.md` and `conductor/delivery-readiness-checklist.md`.

No release, registry, or remote publication side effects were performed.

## 2026-09-29 post-integration CI follow-up

Base: `dc8ba8f5f68168456f1e8710b62b5e59060eb8f3` (main after PR #154).
Working tree: `/Users/doughnut/Documents/careops-sim/.worktrees/kairos-docs-ci-dedup-154`.
The required Rust core context now aggregates stable verification, a Rust 1.76
locked library/binary compile, and a Rust 1.77 locked wasm-export compile. The
first hosted 1.76 attempt used stable because the repository toolchain file
overrides `rustup default`; an explicit `cargo +1.76.0` invocation then exposed
that current `wasm-bindgen` requires Rust 1.77. The Wasm crate now declares that
floor and has its own wasm-target lane. Stable verification runs doctests
separately from the single coverage-instrumented nextest suite. The clap
dependency range and lock were constrained to releases compatible with the
declared core MSRV.

The quality drift receipt now keeps the PR-only skip guard in the five-context
ruleset contract, verifies that its workflow source is present on main, and
expects only four push-capable contexts on an exact main SHA. Its Codecov
configuration requests a `rust-core` project status. Exact-SHA hosted acceptance
is still pending: the trusted-main OIDC upload succeeded on `dc8ba8f`, but GitHub
returned no Codecov project status for that SHA.

Validation on this working tree:

- `actionlint .github/workflows/*.yml` — exit 0.
- `node tests/conformance/quality-frontier-drift-check.mjs` — exit 0; pass,
  pending, drift, unavailable, PR-only guard, and push-check cases covered.
- `node scripts/validation/validate-track13-metadata.mjs` — exit 0; 46 tracks.
- `cargo test --doc --workspace --all-features` — exit 0 on stable; 29.72 s;
  doc-test harnesses compiled and completed across the workspace.
- `rustup run 1.76.0 cargo check --workspace --exclude kairo-ecs-wasm --lib --bins --all-features --locked` — exit 0; no tests were repeated in the core MSRV lane.
- The Rust 1.77 Wasm MSRV lane still requires a fresh hosted result on the updated PR head.
- `just quality-drift` — exit 1 with receipt
  `artifacts/quality-frontier-drift.json`; only the uncommitted source state and
  missing exact-SHA Codecov project status were non-pass. The PR-only skip
  workflow matched main and all four push-required contexts were present.

The manual `Fuzzing Smoke` dispatch on main is run `36556097004`; it completed
successfully on `dc8ba8f5f68168456f1e8710b62b5e59060eb8f3` in 2 minutes. The
scheduler fuzz step and job passed; no crash corpus artifact existed to upload.
Automated review of PR #161 found that the repository's `rust-toolchain.toml`
overrides `rustup default`, so the hosted MSRV step is being changed to invoke
`cargo +1.76.0` explicitly before merge. The initial PR run is not treated as
MSRV evidence; the corrected hosted check must pass. No Track status or phase
closeout was advanced.

## 2026-09-28 quality frontier cross-track handoff

User-authorized issues #120, #121, #122, and #136 required bounded scheduler
quality coverage, one-pass CI, dependency automation, and hosted security gates.
PR #154 integrates the Track 13-owned workflow, Renovate, validation, and test
matrix changes. No public schema or API contract changed.

- Track 01 owns the scheduler source and property tests touched by this work.
  Its handoff records the release-only pending-event accounting correction and
  the deterministic property suite. Track 01 remains under review; this note
  does not advance its status or waive its owner lane.
- Track 06 owns the Python package metadata and binding files touched by this
  work. Its handoff records the declared Ruff test extra and lint corrections.
  Track 06 remains under review; this note does not advance its status or waive
  its owner lane.
- Track 30 owns the stable-toolchain expectation corrected after hosted CI
  installed Rust 1.98 while the matrix still expected 1.95. Its matrix handoff
  and validation record that refresh.

PR #154 hosted Actions at commit `a97d54063c3080e401fc19a17fb8bb1c018b70c8`
passed 61 checks, skipped two conditional jobs, and had no pending or failing
checks. The active `main quality and security gates` ruleset is recorded in
`conductor/quality-frontier-20260928.md`. The first trusted main-push Codecov
upload and hosted Renovate refresh remain post-merge evidence gates; weekly
hosted fuzz and mutation runs also remain due.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.

## 2026-09-29 package-check deduplication

The package dry-run workflow now checks packaging outputs without rerunning
language tests already covered by `ci-bindings` and workspace compilation
already covered by `ci-core`. The Python package and metadata check runs once on
Python 3.14; the binding test matrix still covers Python 3.10–3.14. Julia and
Go package jobs were removed because they only repeated their binding tests and
did not create package artifacts. npm builds once during `npm ci` preparation,
then inspects the package with lifecycle scripts disabled; NuGet restores and
packs the library without rerunning the net10 test project. The PR path filter
includes the workflow file itself. Local `actionlint` and `git diff --check`
passed; hosted Actions must pass on the refreshed PR head before integration.

The changelog, this handoff, and the Track 13 test matrix were updated with the
workflow change. No release or package publication was performed.

## 2026-09-29 quality-frontier drift receipt

Added `just quality-drift`, a read-only GitHub settings readback that records
source context, the active main ruleset, legacy branch protection, Actions
publisher and token settings, private vulnerability reporting, the default-
branch Renovate preset, required check runs bound to the resolved default-branch
SHA, trusted-main Codecov upload/status, and Renovate refresh evidence. The JSON
receipt is written to the ignored `artifacts/quality-frontier-drift.json` path.
It distinguishes `pass`, `drift`, `pending`, and `unavailable`; pending provider
evidence cannot be treated as a pass. The conformance workflow exercises these
states with offline fixtures. Renovate refresh evidence is measured strictly
after the latest merged pull request's GitHub `merged_at`, and duplicate
Codecov upload/status records are reduced to the newest run/status before they
can satisfy the receipt.

Before PR #154 integration, Renovate preset refresh and Codecov upload/status
remain pending. Rerun `just quality-drift` after integration and retain that
fresh receipt with the issue closeout evidence.

## 2026-09-29 Codecov configuration discovery

After trusted-main upload on `8fd4ab83daacfe0494fe16ecde2d320ffa3faef0`, the
CodeCov CLI log said it could not find a config file. The upload job downloaded
`lcov.info` but did not check out repository sources, so the root `codecov.yml`
was absent from its workspace. The job now checks out the exact triggering
commit with credentials disabled before downloading the artifact. The Track 13
metadata validator checks that order. A fresh trusted-main upload and exact-SHA
`codecov/project` status are still required to confirm hosted provider behavior.

## 2026-09-30 selective binding CI follow-up

Pull requests now run a small path-classifier job and only execute binding
checks for changed language surfaces. Shared compiler, crate, FFI, workflow,
conformance, and build inputs fan out to all binding lanes. Unknown paths,
missing diff inputs, and classifier errors run every lane or fail the stable
`Binding CI` aggregate. Main pushes and manual dispatch retain the complete
matrix. Documentation-only changes avoid binding runtimes.

The path trigger covers every pull request so an unlisted/root path cannot be
silently omitted from classification. The `python/kairo_gym` contract suite now
runs at its declared Python 3.9 floor and current Python 3.14; the latter
installs the optional Gymnasium extra and runs Gymnasium's environment checker.
Unit coverage includes C#-only updates, each language map, Gym, shared inputs,
docs-only paths, unknown/root paths, rename/delete pairs, and empty/malformed
input fallback.

Validation is local only at this handoff: `node
tests/conformance/ci-binding-change-classifier-check.mjs`,
`node scripts/validation/validate-track13-metadata.mjs`, `actionlint
.github/workflows/ci-bindings.yml`, and `git diff --check` pass. Hosted Actions
must pass on the exact PR head before integration; no merge was performed.
## Phase closeout evidence

`$conductor-review` completed on 2026-05-08 with no blocking Track 13 findings. Accepted fixes: none required in the workflow surface during this closeout pass. Validation commands passed: `node scripts/validation/validate-track13-metadata.mjs`, `node tests/conformance/track07_13_hardening_check.mjs`, `node tests/conformance/track12_20_evidence_check.mjs`, and `pwsh -NoProfile -File scripts\validate_track13_supply_chain.ps1`. `cargo-deny` and `cargo-audit` were unavailable locally and reported as skipped by the Track 13 supply-chain gate. Git cleanup state: dirty because local Conductor closeout/status edits are pending commit. Commit SHA: `5dd1937566898b2e028ac61dab1e9dd173e6d919`; pushed ref: `origin/main`. Strict cleanup gate `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` remains pending until these local closeout edits are committed. Next-phase decision: Track 13 is Done for the current CI/CD and supply-chain scaffold; future mandatory advisory scanner installation or release hardening belongs in Track 20 or a scoped follow-up.

## Follow-up evidence — issue #122 npm bundle remediation candidate (2026-09-30)

The package dry-run now prepares the locally repacked, registry-integrity-pinned npm 12.1.0 CLI before `npm ci`, validates the resolved top-level patched dependencies, then runs npm version/help, audit, and audit-signature checks on the minimum Node 22 runtime supported by npm 12. Bootstrap no longer suppresses generator or locked-install failures. Local validation in the isolated Kairos candidate passed: generator `--check` twice with identical artifact SRI; clean bootstrap-tools `npm ci`; npm version/help; runtime resolution validator; `npm audit` (zero vulnerabilities); `npm audit signatures` (196 registry package signatures and 91 attestations); and `git diff --check`. Hosted Actions execution and Dependabot/Security readback after integration are still pending; do not mark alerts #64–66 or issue #122 closed from local evidence.
