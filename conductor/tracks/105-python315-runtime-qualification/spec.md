# Python 3.15 runtime migration and qualification

Maturity: candidate; main-branch migration and operational handover are unaccepted.

## Objective

Require every active repository Python use to execute Python 3.15 at the newest
qualified patch, currently exactly 3.15.1. Verify the actual explicitly selected
repo-local interpreter; a manifest, installed binary or successful unrelated
job is insufficient. Preserve simulation contracts and private work.

## Inputs

The direct user runtime mandate, current remote main, authoritative owner claims,
active process/environment inventory and the [retained qualification evidence](prior-qualification.md).
The active conversation goal covers this repository migration and its paired
repository; a track is not a substitute for that goal.

## Outputs and requirements

- Inventory development, build, tests, automation, CI matrices, containers,
  bootstrap, package/build backends, native binding tools and helper scripts.
  Classify historical evidence and synthetic version-negative fixtures separately.
- Use exact repo-local Python 3.15.1 for active execution, including subprocesses.
  Align requires-python constraints, lock resolver targets, dependency pins, CI,
  environment files, generated templates and current setup/run instructions.
  Remove older active default and compatibility runners. Bound development
  discovery to the authorized 3.15 line; reject 3.16 before environment creation.
- Qualify current supported dependency candidates in isolated environments;
  preserve trusted hashes/signature/identity checks and native binding contracts.
  Record genuine incompatibilities rather than silently retaining an unexplained
  old interpreter or claiming unexecuted targets passed.
- Reuse earlier qualification only after checking source/input/lock equivalence.
  Rerun affected tests after changes; do not repeat unchanged expensive suites
  simply because this track is new. Keep raw evidence and honest known failures.
- Integrate through reviewed PRs and all required checks. Verify post-merge source
  and required hosted outcomes at the actual main commit, including real runtime
  version output in active Python lanes. Commit identity alone is not execution.
- Hand over a verified repo-local environment only at a safe terminal boundary.
  Preserve in-flight environments and recovery paths; do not replace them beneath
  running operations. Record the consuming command/path and its actual version.

## Owned paths

Planning owns this track and coordinated Conductor registration only. Runtime
work needs fresh exact-path claims and current owner handoffs for manifests,
locks, workflows, helpers, bindings, containers and instructions. Existing
candidate work remains with its current owner; no blanket ownership transfer.

## Blocked paths

Unrelated Polars/FMI/GPU/MPI/runtime implementation and public schema/ABI changes,
private source history, active operations, other repositories or shared-service
workflows without their owner handoff. No email, native service change, release
publication or protected-branch setting change.

## Dependencies and sequencing

Runtime inventory and isolated qualification can proceed independently of the
Polars dependency-adoption sequence. Serialization is required at shared files
and active environments. The paired repository accepts its own Python execution
and parent helper paths independently; this does not authorize advancing its
Kairos runtime pin or bypassing the existing reconciliation/Polars receipt gates.
External reusable workflow or pinned-source consumers need explicit evidence or
remain unresolved; an upstream version string alone cannot satisfy this track.

## Parallel-safe work

Read-only audits and disjoint documentation; one writer per claimed file.
Environment replacement waits for terminal process evidence, never lease age.

## Acceptance criteria

The [test matrix](test-matrix.md) must prove a complete active-use inventory,
actual 3.15.1 execution, coherent constraints/locks, meaningful binding/package
and helper tests, supported CI/container evidence, no older active runners,
required PR checks, current-main readback and safe environment handover. Any
unavailable required target remains a blocker. Domain results and provenance
must be preserved. No completion from fixtures alone.

## Quality gates

Use existing Conductor validators and exact-command evidence; runtime and hosted
oracles below remain mandatory beyond static document checks.

## Release implications

Changing the supported Python line may break older consumers; document the
explicit user-selected compatibility change without maintaining an old runner.
No library release or native/clinical approval is implied.

Track 30 retains cross-language version-policy ownership; Tracks 06/13/15 and their current owners retain binding, CI and packaging contracts. Track 105 coordinates the complete Python migration beyond Track 30’s policy-only scope.
