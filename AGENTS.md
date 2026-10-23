# Repository agent guide

KairoECS is a Rust-native discrete-event and agent-based simulation library with
multiple language bindings. Keep engine contracts, replay determinism, and
security boundaries explicit in every change.

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
