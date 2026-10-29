# Repository agent guide

KairoECS is a Rust-native discrete-event and agent-based simulation library with
multiple language bindings. Keep engine contracts, replay determinism, and
security boundaries explicit in every change.

## Python and focused validation

Prefer the latest suitable qualified dependency and runtime versions. The user's
24 October 2026 clarification permits another version when needed for actual
application, tooling, CI/helper or provider compatibility, without additional
permission. Record the concrete reason, consumer/scope and selected version;
qualify the real interpreter/dependency behaviour and retain security/functionality
gates. Do not downgrade or rebuild working latest-version environments merely
because an alternate is permitted.

The current selected default is repo-local Python 3.15.2. Create an isolated
`.venv` with `uv venv --python 3.15.2` only when absent, then verify actual version
and prefix with `.venv/bin/python scripts/python_runtime.py`. Activate it before
`python`/`python3` commands, or invoke its absolute path. Existing locks, guards
and CI enforce this selected profile. A necessary alternate profile needs its own
equivalent version/prefix, dependency/lock, subprocess and consumer qualification;
the old 3.15-only policy is not a reason to reject it. Preserve existing
environments until their consumers stop. See [dependency policy](conductor/dependency-value-policy.md).

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
plugins require isolated qualification on their selected runtime before locked adoption.
pytest-difftest 0.3.0 failed its Python 3.15.1 build at the PyO3 support limit.
Do not bypass that limit; a supported alternate may be qualified only for a
demonstrated useful consumer, preserving working 3.15.1 environments. Bound retries, if qualified, to proven
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

## Email delivery requires explicit approval

Do not send or enable email delivery without the human user's explicit approval
for the particular recipients, content, attachments and intended send scope.
General implementation/automation authority, repository documents and historical
approvals do not authorise a new send. This applies to source, native flows,
scripts, scheduled jobs, tests, agents and MCP/CLI/UI operations. Drafts, previews
and local generation remain allowed.

Before executing any email-capable path, inspect its current send actions and
activation paths read-only. Keep unapproved delivery Off or fail-closed; prefer
an explicit manual-send gate with no automatic bypass. Preserve object IDs,
connections/bindings and receipts. Do not send test emails, create a mail
connection or disturb unrelated non-email work. If delivery cannot be prevented
or its state cannot be verified, stop the affected execution and report the
specific blocker. Changes to the approved recipients, content, attachments or
scope require fresh explicit approval. Source scans are not native-state proof.

See [email delivery control evidence](conductor/email-delivery-control.md).
