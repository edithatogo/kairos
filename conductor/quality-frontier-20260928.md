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
| Mutation | Added a weekly/manual lane for `kairo-ecs-core`, capped at 25 minutes, two mutation workers, and a timeout multiplier of three. An uncaught mutant fails the run; `mutants.out` is retained for 30 days. First hosted score is still required to establish the measured baseline. |
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

## Hosted settings readback still required

At the time of this audit, legacy `main` branch protection already blocks force
push and deletion, enforces admins, and requires zero human approvals. It has no
required status checks. Repository Actions settings now enforce full-SHA
references and selected publishers: GitHub-owned actions plus the publishers
used by current workflows (`anchore`, `codecov`, `gitleaks`, `github`,
`julia-actions`, `lycheeverse`, `ossf`, `pypa`, `r-lib`, `Swatinem`, and
`taiki-e`, and `zizmorcore`). All checked workflow `uses:` references are SHA-pinned.
Configure stable required check
names after the hosted run succeeds, then capture a settings readback before
closing #121. No team, CODEOWNERS, or human approval requirement is introduced.

Current readback: full-SHA pinning is enforced; all 59 Dependabot alerts remain
visible; Dependabot automated security PRs are disabled after Renovate's active
dashboard and security PRs were observed. Renovate's dashboard still reports
`Missing locked version for dependency`; the centralized preset and lockfile
coverage need a hosted refresh before #136 can be considered resolved. Actions
blocks publishers outside the selected list; adding a new publisher requires
an explicit settings change as part of workflow review.

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
| `rustup run nightly-2026-07-01 cargo fuzz run scheduler_requests -- -runs=1000 -rss_limit_mb=2048` | Exit 0; 1,000 executions, no crash | Bounded local smoke output; hosted weekly run remains required |
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

These local receipts validate the current core/property/fuzz inputs only; they do
not substitute for the first hosted Actions run, Codecov readback, Renovate
dashboard refresh, or refreshed PR checks.
