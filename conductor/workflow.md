# KairoECS Conductor Workflow

## Principle

KairoECS uses a **contract-first, core-first, bindings-second** workflow.

```text
1. Define contracts.
2. Let subagents work in parallel within owned paths.
3. Integrate through conformance fixtures, not subjective API preference.
4. Promote to release only after docs, CI, packages, and compatibility gates pass.
```

## Contract-first execution

The project must establish these contract artifacts before broad implementation begins:

```text
conductor/contracts/core-contract.md
conductor/contracts/ffi-contract.md
conductor/contracts/arrow-schema-contract.md
conductor/contracts/conformance-contract.md
conductor/contracts/versioning-compatibility.md
```

These are the handoff points for subagents. A subagent may implement behind a mock, stub, or feature flag as long as it preserves the contract.

## Path ownership rule

Each subagent owns a narrow set of paths and should not modify another agent's path without a handoff note.

Track `owner` fields name the responsible agent role; they do not create a
separate human-owner review requirement in this single-maintainer repository.
Use a reviewer agent/subagent with a distinct review lens and record its exact
commit, role, findings and disposition. The human maintainer sets scope and
accepts the integrated result. Preserve explicit human approvals only where the
decision itself requires external, legal, security, release, or domain authority.

| Subagent | Primary paths |
|---|---|
| foundation-agent | root metadata, license, governance files |
| contracts-agent | `conductor/contracts/`, `crates/kairo-ecs-types/` |
| core-scheduler-agent | `crates/kairo-ecs-core/` |
| ecs-agent | `crates/kairo-ecs-state/` |
| ffi-agent | `crates/kairo-ecs-ffi/`, `include/` |
| uniffi-agent | `crates/kairo-ecs-uniffi/` |
| diplomat-agent | `crates/kairo-ecs-diplomat/` |
| des-api-agent | `crates/kairo-ecs-des/` |
| abm-api-agent | `crates/kairo-ecs-abm/` |
| arrow-agent | `crates/kairo-ecs-arrow/`, `schemas/arrow/` |
| viz-agent | `crates/kairo-ecs-viz/`, `examples/viz/` |
| python-agent | `bindings/python/` |
| r-agent | `bindings/r/` |
| julia-agent | `bindings/julia/` |
| typescript-agent | `bindings/typescript/`, `crates/kairo-ecs-wasm/` |
| csharp-agent | `bindings/csharp/` |
| go-agent | `bindings/go/` |
| conformance-agent | `conformance/`, `tests/conformance/` |
| performance-agent | `benches/`, `crates/kairo-ecs-bench/` |
| ci-agent | `.github/`, `deny.toml`, dependency automation |
| docs-agent | `docs/`, `website/` |
| release-agent | `packaging/`, `CHANGELOG.md`, release checklists |
| security-agent | `SECURITY.md`, advisories, supply-chain policy |

## Required artifact shape per track

Every track should contain:

```text
spec.md
plan.md
agent-contract.md
risk-register.md
test-matrix.md
handoff.md
```

At minimum, every `spec.md` must define:

```text
Inputs
Outputs
Owned paths
Blocked paths
Dependencies
Parallel-safe tracks
Acceptance criteria
Quality gates
Release implications
```

## Delivery order

Use the live dependency graph, not the obsolete calendar waves. Strengthen the
small Rust kernel and consumer contract first; complete supported scientific
methods and installed adapters; qualify beta/stable packages; then evaluate
advanced backends on measured workloads. Track 114 coordinates ecosystem
interfaces and the [bounded workpack](execution/README.md) provides leaf scopes.

## Merge protocol

1. Subagent opens a PR against its track branch.
2. CI must pass for the owned surface.
3. Conformance fixtures must pass when relevant.
4. Public contract changes require an ADR and versioning note.
5. Integration happens through a merge queue or integration branch.
6. Release branch only accepts PRs with docs and tests.

## Definition of done

A track is not done when the code compiles. A track is done when:

```text
spec accepted
plan executed
unit tests pass
integration/conformance tests pass where applicable
benchmarks recorded if performance-sensitive
docs updated
release notes updated
quality gates pass
handoff file written
```

## Proportionate task execution

For routine authorized work, use one bounded task: one observable behaviour,
up to five source/test files, explicit context and at most three focused check
commands. Target 30–90 minutes for a settled local-model task. Return the diff,
actual check results and any unresolved issue in the PR/commit. The existing
path claim prevents concurrent writers; it is not a second approval process.
Do not create a new ADR, risk register, review panel or evidence dossier per leaf.

Keep status in the existing track plan/registry; the workpack describes scope
and dependencies without a competing status ledger. One review may cover a
coherent batch of routine leaves. A reviewer can be an agent; no additional
human approval is required for reversible work already authorized by the user.
Use specialist review for a numerical algorithm, public API/ABI/schema, replay,
security or release decision. Missing architecture is a small design task before
implementation, not an invitation for a basic model to guess.

Existing per-phase review checkboxes refer to coherent milestones, not every
leaf or red/green test iteration. Reuse unchanged evidence by source/config/input
identity. Full native, consumer and release checks remain at the milestones they
actually qualify. An unavailable tool is reported; it must not become a fake pass.

## Automatic phase closeout gate

At a coherent phase/status milestone, run `$conductor-review` (the Conductor
review operation, not a requirement for a separately installed command), inspect
the diff and auto-apply accepted review fixes. Record unresolved findings once
in the track handoff. Synchronize `conductor/tracks.yaml`, `conductor/tracks.md`,
`conductor/phase-closeout.yaml` and `conductor/status.md` only when their state
changes; do not edit every ledger for every leaf. Update track-map/readiness only
when ownership/dependencies/readiness change.

Run the affected test matrix and
`pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`,
`pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` at that milestone.
Commit and push the cleaned slice, then use
`pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
for publication/cleanup checks. Record a real blocker when a required gate
cannot run. These are integration checks, not new user permission gates.

## Handoff timing rule

Handoff.md must follow this lifecycle:

1. **Phase 0-3**: handoff.md is a draft. It may say "No code files were changed in this handoff pass" or similar placeholder text.
2. **Phase 4 (Cross-track integration)**: handoff.md is updated with actual changed files, contracts consumed, and tests added.
3. **Phase 5 (Closeout)**: handoff.md is finalised with resolved risks, follow-up issues, and integration notes.

CI lint rule: If a track is `In Progress` or higher and its handoff.md contains the phrase "No code files were changed", CI emits a warning — the handoff may be stale.

## Focused validation and local Python

Start with an affected file/node or native selector, for example `.venv/bin/python -m unittest discover -s tests -p test_local_python_runtime.py -v`, `.venv/bin/python -m pytest bindings/python/tests/test_import.py -q`, `cargo test --locked -p PACKAGE FILTER`, or the exact PowerShell validator. Use existing real nodes/files, not a guessed selection; verify tests executed. Python commands must use the verified local selected profile described in AGENTS.md; 3.15.2 remains the current qualified default. Necessary alternate versions are permitted under the dependency policy, with explicit consumer/scope, reason and actual qualification.

Keep pytest cacheprovider enabled per worktree/environment. Failure-only mode is `--lf --last-failed-no-failures=none` (zero selected tests proves nothing). `--ff` still runs the whole selected suite; `--sw` is a local debug loop. Do not put selection flags in global addopts. Dependency/configuration/schema/fixture changes or unknown impact invalidate narrow selection and cached evidence. Full clean integration, package, coverage, security and release gates remain required at coherent milestones.

Optional plugins require isolated selected-runtime qualification before locked adoption. The reported pytest-difftest 0.3.0 source build fails on 3.15.1 because PyO3 supports only through 3.14; do not bypass that check. A useful consumer may qualify a supported alternate profile without further permission, preserving the working 3.15.1 environment. Default retries to zero. Only demonstrated transient fictional/read-only cases may use bounded opt-in retries with the first failure retained. Native mutations/email are never runner retry targets. skip/skipif requires an actual capability condition/reason; xfail requires a known issue/reason and strict=True. Preserve meaningful unittest/Cargo/PowerShell selectors instead of converting suites for plugins.
