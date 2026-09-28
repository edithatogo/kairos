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
- For a narrower change, run the smallest relevant validator and report skipped checks. A local pass is not evidence of a hosted Actions run or release acceptance.
- Record command, working directory, commit, toolchain, deterministic seed/input hash where applicable, exit status, and artifact path for validation claims.
- Keep secrets and private vulnerability details out of source and public CI artifacts. Do not publish packages, create releases, or change protected branch settings as an implicit consequence of a passing test.
