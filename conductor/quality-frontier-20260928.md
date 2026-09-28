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
  test matrix), lets npm pack inspect the single build produced by `npm ci`, and
  packs NuGet without rerunning its net10 test project. Julia and Go dry-run
  jobs were removed because they repeated binding tests and had no distinct
  package artifact check. Hosted Actions on the updated PR head remains pending.
- CI installs the pinned cargo tools from their release binaries through a
  SHA-pinned installer action with checksum verification and source-build
  fallback disabled, avoiding long cold `cargo install` builds.
- Pull-request runs cancel stale runs for the same ref; push and scheduled work
  remain non-cancellable. The daily heavy Miri/benchmark lane is filtered away
  from the additional weekly mutation schedule, avoiding a duplicate Sunday run.
- Python CI installs its pinned Ruff version from the package's test extra.
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
and linear history, and requires zero human approvals. Required checks are
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

Current readback: full-SHA pinning and selected publisher policy are enforced;
43 Dependabot alerts remain open (2 critical, 19 high, 18 moderate, 4 low).
Dependabot-created update PRs from September 3–18 remain open alongside active
Renovate PRs and its Dependency Dashboard. No Dependabot configuration file is
present in `.github` on `main`; the remaining Dependabot PRs are backlog to
triage, not evidence that another configuration file should be added. The
hosted Renovate app has produced PRs and its dashboard, but the shared preset in
this branch has not yet been loaded by hosted Renovate. After this PR merges,
refresh the dashboard and confirm lock coverage before closing #136. Actions
blocks publishers outside the selected list; adding a publisher requires an
explicit settings change as part of workflow review.

## Local validation receipt

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
