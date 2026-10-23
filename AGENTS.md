# Repository agent guide

KairoECS is a Rust-native discrete-event and agent-based simulation library with
multiple language bindings. Keep engine contracts, replay determinism, and
security boundaries explicit in every change.

## Python and focused validation

Use an explicit repository-local Python 3.15.1 interpreter. Create an isolated
`.venv` with `uv venv --python 3.15.1`, then verify actual version and prefix with
`.venv/bin/python scripts/python_runtime.py`. Activate it before documented
`python`/`python3` commands, or invoke its absolute path. CI verifies the local
environment after checkout and exports its bin directory for all later helpers.
Preserve existing environments until their consumers stop.

During edits, select the affected unittest file/class/method, pytest node/file
or `-k`, Cargo package/filter, or PowerShell validator. Keep pytest cacheprovider
per worktree/environment. `--lf --last-failed-no-failures=none` may execute zero
tests, which verifies nothing; `--ff` still runs the selected suite; `--sw` is a
local debugging loop. Never add these flags globally to addopts. Reuse evidence
only when source, dependencies, config, schema and fixtures are unchanged;
unknown impact requires broader checks. Keep full integration, clean-package,
coverage, security and release gates at coherent milestones.

Default to zero retries. Conditional skips need a real missing capability/reason;
xfail needs a known issue/reason and strict=True. Neither is for speed. Optional
plugins require isolated Python 3.15.1 qualification before locked adoption.
pytest-difftest 0.3.0 is currently blocked by its PyO3 Python support limit; do
not bypass the limit or downgrade Python. Bound retries, if qualified, to proven
transient fictional/read-only cases and preserve first failure; never retry
native mutations or email sends through a runner.

## Start here

- Read `README.md` and `CONTRIBUTING.md` for project workflow and contribution rules.
- Read `SECURITY.md` before changing CI, dependencies, FFI, publishing, or artifact handling.
- Use `conductor/workflow.md`, `conductor/track-map.md`, and `conductor/tracks.yaml` to find the active owner and acceptance gates.
- Check `conductor/subagents.md` and the selected track's `spec.md`, `plan.md`, and `test-matrix.md` before changing owned paths.

## Change and validation rules

- Keep patches within the selected track's ownership contract; hand off before crossing owners.
- Preserve deterministic scheduling and shared conformance contracts. Public API, ABI, schema, or fixture changes require the review steps in `CONTRIBUTING.md`.
- Run `just ci` as the one-command Rust validation lane. It uses one coverage-instrumented workspace test pass, checks the core coverage floor, and runs formatting, lint, docs, and dependency policy checks.
- Run `just quality-drift` for a fresh read-only GitHub settings/source readback. It writes `artifacts/quality-frontier-drift.json`; `pass`, `drift`, `pending`, and `unavailable` remain distinct, and non-pass results exit nonzero.
- For a narrower change, run the smallest relevant validator and report skipped checks. A local pass is not evidence of a hosted Actions run or release acceptance.
- Record command, working directory, commit, toolchain, deterministic seed/input hash where applicable, exit status, and artifact path for validation claims.
- Keep secrets and private vulnerability details out of source and public CI artifacts. Do not publish packages, create releases, or change protected branch settings as an implicit consequence of a passing test.

## Agent session coordination

For authorized writer work, use the [single-maintainer harness](conductor/harness/single-maintainer.md): clean isolated worktree, repository-wide advisory path claim, bounded hashed context, and precommit scope check. Reconcile already-active sessions before adopting the lease store. Lease expiry alone does not permit takeover; context or a claim does not authorize a task or certify completion.
