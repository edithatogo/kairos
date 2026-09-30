# 41 Documentation Platform, Quality Gates & Learning Coverage -- test-matrix.md

## Baseline validation

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`

## Docs and site validation

- `npm --prefix website run check:links`
- `npm --prefix website run check:quality`
- `npm --prefix website run check:all`
- `npm --prefix website run build`
- `node scripts/validation/validate-learning-coverage.mjs`
- `python notebooks/validate_notebooks.py`

## CI strictness validation

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `go vet ./...`
- `gofmt -w -l .`
- `python -m pytest -q`
- `python -m ruff check kairo_ecs tests`
- `python -m compileall kairo_ecs tests`

## Learning coverage validation

- `rg -n "tutorial|example|notebook" docs tutorials examples notebooks`
- `node docs/assets/validate-playground-figures.mjs`

## Closeout reminders

- `$conductor-review` must run before any phase/status advance.
- Any deliberate exception must be recorded in `handoff.md`.
- The final commit must leave the worktree clean and the registry surfaces in
  lockstep.
