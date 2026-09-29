# Quality frontier findings and implementation

Scope: Kairos issues [#120](https://github.com/edithatogo/kairos/issues/120),
[#121](https://github.com/edithatogo/kairos/issues/121),
[#122](https://github.com/edithatogo/kairos/issues/122), and
[#136](https://github.com/edithatogo/kairos/issues/136). This records local
changes on `codex/quality-frontier-122`; hosted checks and settings must be
read back after integration before the issues can be closed.

## Evidence-based test scope

| Technique | Decision and evidence |
| --- | --- |
| Property-based | Added 256-case scheduler ordering and cancellation/accounting properties with a fixed seed. Shrinking remains available for deterministic reproduction. |
| Mutation | Added a weekly/manual lane for `kairo-ecs-core`, capped at 25 minutes, two mutation workers, and a timeout multiplier of three. An uncaught mutant fails the run; `mutants.out` is retained for 30 days. Hosted manual run `36456693140` passed on foundation commit `c54a949` with 73 mutants: 52 caught, 21 unviable, and 0 missed. The first scheduled weekly run on `main` remains to be verified. |
| Fuzzing | Added a scheduler request harness with fixed-size, bounded inputs, accounting/order invariants, a 60-second fuzz budget, a 2 GiB RSS limit, and retained crash artifacts. |
| DST | Excluded from engine tests. `SimTime` is an integer logical tick; the engine has no civil calendar, timezone, or daylight-saving conversion. A future adapter that maps civil time to ticks must own and test that conversion. |
| Multithreading | Excluded from scheduler/transport stress tests until a concurrent runtime is implemented. The core contract explicitly says single-threaded; `ThreadChannelTransport` is currently an in-memory `BTreeMap`/`VecDeque` and mutating operations require `&mut self`. These types do not expose a concurrent transport contract. |
| Contract | Already present in the conformance fixtures, core/FFI/Arrow contracts, and their schema/replay validators; no new duplicate contract framework is warranted. |

## CI and dependency automation

- `just ci` is the local entry point for the Rust CI lane. `just test` runs the
  full workspace once with coverage instrumentation and writes `lcov.info`;
  the checker filters that report to `kairo-ecs-core` production sources and
  enforces 90%. The latest local run passed 258/258 tests and measured core
  coverage at 94.38% (437/463 lines). The exploratory workspace aggregate was
  79.98%, so this does not introduce an arbitrary 80% workspace gate or a
  second test run.
- Core CI remains the sole cargo audit/advisory scan. Policy CI still checks
  workflow/dependency policy metadata but no longer installs and repeats both
  scanners.
- Package dry-run CI now checks package artifacts without duplicating the
  language tests in binding CI or workspace test compilation in core CI. It
  builds Python artifacts once on Python 3.14 (binding CI retains the 3.10–3.14
  test matrix), installs the locked npm package, runs one explicit build before
  pack, and
  packs NuGet without rerunning its net10 test project. Julia and Go dry-run
  jobs were removed because they repeated binding tests and had no distinct
  package artifact check. Hosted Actions on the updated PR head remains pending.
- CI installs the pinned cargo tools from their release binaries through a
  SHA-pinned installer action with checksum verification and source-build
  fallback disabled, avoiding long cold `cargo install` builds.
- Pull-request runs cancel stale runs for the same ref; push and scheduled work
  remain non-cancellable. The daily heavy Miri/benchmark lane is filtered away
  from the additional weekly mutation schedule, avoiding a duplicate Sunday run.
- Python CI installs its pinned Ruff version and remaining tools from the
  hash-locked tools file, then installs the editable binding with dependency
  resolution and build isolation disabled.
- OpenSSF Scorecard PinnedDependencies findings #455, #454, #453, #404, #403, #386, and #377 now have local source fixes: NuGet package dry-run restore is in locked mode against the committed `net10.0` lock file; Python bootstrap, binding CI, and package dry-run dependencies install from SHA-256 hash-verified locks; bootstrap npm and docs Mermaid CLI install via committed `npm ci` package locks with integrity hashes. The Python lock inputs retain exact direct versions, and generated locks capture transitive versions and available wheel/sdist hashes. The Mermaid CLI is `11.17.0`: its `12.0.0` dependency tree produced six high-severity audit findings, while the selected lock audits cleanly. Hosted Scorecard alert refresh remains pending integration and a new default-branch scan.
- Renovate now extends `github>edithatogo/renovate-config`, retains repo-specific
  grouping, runs on Brisbane time, and is limited to one PR per hour/two open
  PRs. Inherited low-risk automerge is disabled until stable required checks
  exist. GitHub Dependabot alerts remain enabled for visibility.
- Rust coverage uploads to Codecov only from trusted `main` pushes, in a
  separate minimal-permission OIDC job. PR workflow definitions are editable by
  the PR, so they use the Rust core check as the merge gate and receive no OIDC
  permission. The first hosted upload must verify the Codecov project, report,
  and main commit status. A PR-specific Codecov status remains deferred until it
  can be provided by a workflow whose privileged definition is not PR-controlled.

## Hosted settings readback

Legacy `main` branch protection blocks force-push and deletion, enforces admins
and linear history, and requires zero human approvals. This is deliberate for
the solo-maintainer model in issue #121; Scorecard's CodeReview alert #117 is
therefore an evidence-backed exclusion until a qualified second maintainer is
available, rather than a reason to impose an unstaffed approval gate. Required checks are
configured in the separate active ruleset. Repository Actions settings enforce full-SHA
references and selected publishers: GitHub-owned actions plus the publishers
used by current workflows (`anchore`, `codecov`, `gitleaks`, `github`,
`julia-actions`, `lycheeverse`, `ossf`, `pypa`, `r-lib`, `Swatinem`, and
`taiki-e`, `zizmorcore`). All checked workflow `uses:` references are SHA-pinned.
The active ruleset `main quality and security gates` (ID `24119475`) targets
`main`, requires strict up-to-date results for `Rust core quality`,
`CodeQL (javascript)`, `gitleaks`, `Reject CI skip directives`, and
`code and repository health`, and blocks deletion and non-fast-forward updates.
The repository owner has PR-only bypass for recovery. Legacy protection also
enforces admins and linear history, blocks force-push/deletion, and requires
zero approvals. Root and `.github/CODEOWNERS` contain duplicate maintainer
templates, but required code-owner reviews are disabled; they impose no approval
or team gate on this solo-maintained repository.

Hosted PR #154 readback at head `c54a949709873dcf1ce515c71276a6e3e526a0fc`:
59 checks passed, the Codecov OIDC upload was skipped as intended on a PR, and
none failed or remained pending. Rust core run `36456678729` passed formatting,
Clippy, one workspace test-and-coverage pass, docs, and dependency policy. The
first trusted main push must still verify the Codecov repository, uploaded
report, and main commit status.

Current default-branch security snapshot (2026-09-29, `fae901558f07b7b717a676adbafbe2cdc78dea1c`): 43 Dependabot alerts (2 critical,
19 high, 18 moderate, 4 low), 13 code-scanning alerts, and 0 secret-scanning
alerts. This snapshot predates integration of this branch; its alert counts do
not represent the state after these changes.
Dependabot-created update PRs from September 3–18 remain open alongside active
Renovate PRs and its Dependency Dashboard. No Dependabot configuration file is
present in `.github` on `main`; the remaining Dependabot PRs are backlog to
triage, not evidence that another configuration file should be added. The
hosted Renovate app has produced PRs and its dashboard, but the shared preset in
this branch has not yet been loaded by hosted Renovate. After this PR merges,
refresh the dashboard and confirm lock coverage before closing #136. Actions
blocks publishers outside the selected list; adding a publisher requires an
explicit settings change as part of workflow review.

Scorecard SecurityPolicy alert #118 is addressed in the branch policy text:
`SECURITY.md` links directly to GitHub's private vulnerability reporting action,
and a live repository API readback confirmed private vulnerability reporting is
enabled (`enabled: true`). The policy names GitHub Security Advisory publication
as the coordinated public disclosure route and does not invent an email contact.
Refresh Scorecard's hosted finding after this branch is integrated.

### Scorecard alert dispositions

These are dispositions against the default-branch Scorecard snapshot at
`fae9015` (2026-09-29), not claims that hosted alerts are cleared. A fresh
default-branch scan is required after integration.

| Alert | Disposition and remaining evidence gate |
| --- | --- |
| #117 CodeReview | Intentional solo-maintainer exclusion: required approvals remain zero under issue #121. Reconsider when a qualified second maintainer is available; do not weaken this staffing constraint to improve the score. |
| #119 CII Best Practices | No CII badge or certification is currently claimed. This is an external, voluntary program; do not mark satisfied absent an application and independently verified award. |
| #118 SecurityPolicy | Branch policy text and enabled private vulnerability reporting address the finding locally. The fix is unmerged; hosted alert closure remains pending a post-integration scan. |
| #120 SASTID, #121 FuzzingID | Latest default-branch scan reports 21/22 commits scanned for these code-scanning checks. Branch workflows add CodeQL and bounded fuzzing, but these findings remain open until a fresh default-branch scan confirms coverage. |
| #482 VulnerabilitiesID | Default-branch snapshot reports 31 advisories. No local evidence here establishes that this alert is cleared; refresh dependency scanning on the integrated default branch and triage any remaining advisories. |
| #455, #454, #453, #404, #403, #386, #377 PinnedDependencies | Local source fixes and validation are recorded below. Hosted findings remain pending integration and a new Scorecard scan. |

The active ruleset currently requires five stable checks: `Rust core quality`,
`CodeQL (javascript)`, `gitleaks`, `Reject CI skip directives`, and `code and
repository health`. Branch protection enforces admins and linear history with
zero required human approvals, consistent with issue #121. These controls are
current settings readback; they do not clear the default-branch CodeReview
finding or the other Scorecard alerts by themselves.

## Local validation receipt

### Scorecard pinned dependency remediation

The seven live alerts from the default-branch Scorecard snapshot pointed to
package installs in package dry-run, bootstrap, binding CI, and docs CI. The
updated commands use `pip install --require-hashes`, `npm ci` with committed
package locks, or `dotnet restore -p:RestoreLockedMode=true`; the NuGet lock
contains the current `net10.0` graph (no third-party packages are currently
referenced by the package project). These local checks validate the lock
formats and commands but do not update GitHub's hosted alert snapshot.

| Alert | Source location on `main` | Local closure evidence |
| --- | --- | --- |
| #455 | `.github/workflows/package-dry-run.yml` NuGet restore | Uses committed `bindings/csharp/src/Kairo.ECS/packages.lock.json` in locked restore and pack mode; local restore and pack succeeded. |
| #454 | `.github/workflows/package-dry-run.yml` Python build/twine install | Uses `scripts/package-python-tools.lock` with exact inputs and SHA-256 hashes for the complete transitive graph; hash-mode dry-run, build, and twine check succeeded. |
| #453, #403 | `scripts/bootstrap.sh` Python tool installs | Both mutable pip commands now share `scripts/bootstrap-python-tools.lock`; full hash-verified installs succeeded under Python 3.10, 3.11, and 3.14. |
| #404 | `scripts/bootstrap.sh` global npm install | Replaced with `npm ci` from `scripts/bootstrap-node-tools/package-lock.json` (npm 11.20.0 plus resolved integrity hashes); CLI execution and audit succeeded. |
| #386 | `.github/workflows/ci-bindings.yml` Python installs | Uses the bootstrap lock across Python 3.10–3.14; full hash-verified installs and package-lock validation succeeded locally under Python 3.10, 3.11, and 3.14. The editable install disables dependency/build-isolation resolution after those tools are installed. |
| #377 | `.github/workflows/docs-quality.yml` global Mermaid CLI install | Replaced with `npm ci` from `tools/docs-quality-node-tools/package-lock.json` (Mermaid CLI 11.17.0); install, npm audit, CLI version, and a real diagram render succeeded. |

| Check | Result | Evidence and limit |
| --- | --- | --- |
| Python requirement locks and package | Pass | Created clean temporary venvs using Python 3.10.20, 3.11.6, and 3.14.7. In each, `python -m pip install --require-hashes -r scripts/bootstrap-python-tools.lock` and the same install for `scripts/package-python-tools.lock` exited 0. The package lock was also installed before `python -m build --no-isolation` and `python -m twine check dist/*`, both successful. An initial host-wide dry-run was blocked by PEP 668; venv validation succeeded. |
| npm CLI lock | Pass | `npm ci --ignore-scripts --prefix scripts/bootstrap-node-tools`; locked CLI reported `11.20.0`; `npm audit --prefix scripts/bootstrap-node-tools` reported zero vulnerabilities. |
| Mermaid lock and render | Pass | `npm ci --prefix tools/docs-quality-node-tools`; `mmdc --version` reported `11.17.0`; npm audit reported zero vulnerabilities. The workflow explicitly installs Puppeteer's browser after npm ci; with the pinned Chrome headless shell `154.0.8037.57` in `/tmp/kairos-puppeteer-cache`, Mermaid rendered `planning/diagrams/api-review-gate.mmd` successfully. Mermaid CLI `12.0.0` was rejected after its transitive tree reported six high findings. |
| NuGet lock | Pass | `dotnet restore` with `RestoreLockedMode=true`, followed by `dotnet pack --no-restore` with locked mode, succeeded for `net10.0`; generated package was `Kairo.ECS.0.1.0-preview.1.nupkg`. |
| Workflow and shell syntax | Pass | `actionlint` on all three changed workflows, `shellcheck scripts/bootstrap.sh`, `node scripts/validation/validate-track13-metadata.mjs` (46 tracks checked), and `git diff --check` all exited 0. |

Hosted CI initially exposed platform-specific dependencies omitted from the
Python 3.14-generated universal locks: `backports.tarfile` through
`jaraco-context` on Python 3.10, and `importlib-metadata` through `keyring` on
Python 3.11; local Python 3.10 also exposed conditional `tomli`. Both lock
inputs now pin `backports.tarfile==1.2.0`, and both locks are regenerated with
`uv pip compile --python-version 3.10 --generate-hashes --universal`, so the
oldest supported interpreter determines the complete marker-aware graph. The
locks now include the required hashes and Python markers for
`backports-tarfile`, `importlib-metadata`, `tomli`, and `exceptiongroup`.

The hosted Track 13 metadata validator also expected the previous literal
editable-install command. It now checks for both the hash-locked tool install
and the actual `--no-deps --no-build-isolation` editable install; the local
validator passes with 46 tracks checked.

Hosted Scorecard must be refreshed after integration; this receipt does not
claim that the live alerts are already closed.

Environment: `/Users/doughnut/Documents/careops-sim/.worktrees/kairos-quality-frontier-122`,
baseline commit `fae901558f07b7b717a676adbafbe2cdc78dea1c`, stable Rust `1.96.0`
(`ac68faa20`, Cargo `1.96.0`, `30a34c682`). Composite SHA-256 over the NUL-delimited
relative path and file contents of `Cargo.toml`,
`Cargo.lock`, the core manifest/source/property tests, and fuzz manifest/target:
`c091a243aeb7e23d86c095f6a48adb14f404b6e36165bfd25e01cfb7cce08588`.

| Command | Result | Artifact/evidence |
| --- | --- | --- |
| `rustup run stable cargo llvm-cov nextest --workspace --all-features --lcov --output-path lcov.info` followed by `node scripts/validation/check-core-coverage.mjs lcov.info` | Exit 0; 258/258 tests passed; core 437/463 lines (94.38%) | Local `lcov.info`, SHA-256 `ac8268dc4218ad1e63f8b5731b7e29b0c183513c11d969115c419e2704f7dfcc` |
| `cargo mutants --package kairo-ecs-core --jobs 2 --timeout-multiplier 3 --output mutants.out.20260928` | Exit 0; 73 mutants, 52 caught, 21 unviable, 0 missed | Local `mutants.out.20260928/mutants.out/outcomes.json`, SHA-256 `d31661f976539f13425fe1dbf133de9ffa3cab8c0f5575f9ef2be55b26fd6622` |
| `rustup run nightly-2026-07-01 cargo fuzz run scheduler_requests -- -runs=1000 -rss_limit_mb=2048` | Exit 0; 1,000 executions, no crash | Bounded local smoke output; hosted manual run `36456693099` also passed on foundation commit `c54a949` (2,419,484 executions in 61 seconds, no crash); first scheduled weekly run on `main` remains to be verified |
| `rustup run stable cargo test --release -p kairo-ecs-core` | Exit 0; 32 tests passed (17 unit, 4 conformance, 8 integration, 3 property) | Run after correcting release-only pending-event accounting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Exit 0 | Post-fix workspace lint |
| `actionlint .github/workflows/*.yml`, metadata validator, `git diff --check` | Exit 0 | Post-fix workflow and patch checks |

The release suite was preceded by a targeted release regression that failed:
dispatch removed IDs only inside `debug_assert!`, so release builds left events
pending and allowed cancelling an already-dispatched ID. Removal now runs in
all profiles, with the assertion checking the saved result. The full release
suite passes with the fix.

The Codex Security diff scan completed for an earlier snapshot of this branch
(scan `c42effdb-9289-424c-9e10-17cc82aa9233`, digest
`codex-security-snapshot/v1:sha256:dfc0f4c021b7defb8afff2127cbf3a0682ea3b4f0bd79b52ce50ebb16e5fc2b4`).
It identified the PR OIDC trust boundary, which was subsequently tightened to
trusted `main` pushes only. The scan report therefore does not certify the
final working tree; the hosted checks and final review remain outstanding.

The manual hosted fuzz and mutation runs above validate the bounded workflows on
foundation commit `c54a949`; the current PR checks validate the updated branch.
The first scheduled weekly run on `main`, trusted-main Codecov upload/readback,
Renovate shared-preset refresh, and final PR checks after integration remain
acceptance gates.

## 2026-09-29 receipt and PR refresh

The live `just quality-drift` readback was run from PR #154's local checkout at
base snapshot `fae901558f07b7b717a676adbafbe2cdc78dea1c`. The receipt returned
`pending` (exit 2), as expected while #154 remains open: its source checkout is
dirty, the main branch does not yet contain the candidate context and Renovate
changes, current main lacks the new skip-guard run, and trusted-main Codecov
upload/status have not run on the updated default branch. Ruleset, legacy
protection, Actions publisher/token settings, and private vulnerability
reporting passed their live readbacks. The generated receipt is intentionally
ignored at `artifacts/quality-frontier-drift.json`; rerun the command after
integration to create closeout evidence.

The receipt now compares Renovate bot PR/comment activity to the latest merged
PR's GitHub `merged_at`, not a commit timestamp, and evaluates only the newest
exact-name Codecov OIDC upload and `codecov/project` status on the default
branch SHA. Offline conformance cases cover pending, stale, failed, and
unavailable evidence. `node tests/conformance/quality-frontier-drift-check.mjs`,
`node tests/conformance/conformance-check.mjs`, JavaScript syntax checks, and
`git diff --check` passed after these changes; hosted checks for this new patch
have not run yet.

As of this live readback, PR #154 is open on head
`b64105836e75f3d9ce8cf60ea8fe2673cc13746d`, with a clean merge state and 53
successful checks plus the expected PR-only Codecov OIDC skip. The locally
updated receipt patch is not part of that head yet. Current open security
counts from GitHub are 43 Dependabot alerts (2 critical, 19 high, 18 medium,
4 low), 13 code-scanning alerts, and 0 secret-scanning alerts.
