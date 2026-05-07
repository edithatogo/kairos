# Track 31 Plan: Performance Regression Guard

## Phase 0 - Scope lock

### Task 0.1 - Inventory existing benchmarks
- List every benchmark in `benches/` and `crates/kairo-ecs-bench/`.
- Categorize by area: scheduler throughput, entity insertion/deletion, component iteration, event dispatch, serialization, Arrow conversion.
- Record current baseline values (if available) from Track 12 and Track 18 outputs.

### Task 0.2 - Lock the owned surface
- Keep new work to `conductor/tracks/31-performance-regression-guard/`.
- Write threshold definitions into `conductor/performance-thresholds.md`.
- Write regression comparison scripts into `benches/regression/`.
- Write `bench-regression.yml` into `.github/workflows/`.
- Do not modify benchmark harness code, comparative benchmark scenarios, or CI infrastructure.

## Phase 1 - Define performance thresholds

### Task 1.1 - Set baseline values
- For each benchmark, record the current mean and standard deviation from 5+ runs on a controlled runner.
- Document the runner hardware, OS, and Rust version used for baseline measurement.

### Task 1.2 - Define acceptable regression
- Set a per-benchmark regression threshold:
  - Scheduler throughput: 5% regression allowed.
  - Entity insertion/deletion: 3% regression allowed.
  - Component iteration: 5% regression allowed.
  - Event dispatch: 5% regression allowed.
  - Serialization/Arrow: 10% regression allowed.
- Measure using mean-based comparison against the percentage-of-baseline thresholds
  used by `benches/regression/compare.py`.

### Task 1.3 - Publish thresholds
- Write `conductor/performance-thresholds.md` as a markdown table.
- Include runner environment metadata.
- Mark which thresholds are blocking (PR gate) versus advisory (release note only).

## Phase 2 - Build regression detection workflow

### Task 2.1 - Write comparison scripts
- Script in `benches/regression/compare.py` that takes two benchmark JSON outputs (base and PR) and produces a comparison report.
- Report: benchmark name, base mean, PR mean, percentage change, threshold, pass/fail.

### Task 2.2 - Write CI workflow
- `.github/workflows/bench-regression.yml`:
  - Triggered on PRs that touch `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, `benches/`, `crates/kairo-ecs-bench/`.
  - Runs benchmarks on the PR branch.
  - Checks out the base branch and runs benchmarks.
  - Runs `compare.py` on both outputs.
  - Fails if any benchmark exceeds its threshold.

### Task 2.3 - Wire the gates
- Add `benchmark-regression-check` and `threshold-definition-exists` to `conductor/quality-gates.md`.
- Ensure `threshold-definition-exists` fails if a benchmark lacks a threshold entry.

## Phase 3 - Handoff and closeout

### Task 3.1 - Prepare maintainer notes
- Document how to update baseline values when intentional performance changes are made.
- Document how to adjust thresholds for specific benchmark categories.
- List the controlled runner hardware requirements.

### Task 3.2 - Cross-track communication
- Hand off to Track 12 (Conformance/Benchmarks) for baseline cooperation.
- Hand off to Track 13 (CI/CD) for workflow review and runner provisioning.
- Hand off to Track 18 (Comparative Benchmarks) for alignment on methodology.
- Notify Track 15 (Packaging) that this track is non-critical for release.

### Task 3.3 - Update the risk register
- Mark resolved risks as mitigated.
- Escalate any benchmark that cannot produce a stable baseline.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
5. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
6. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.