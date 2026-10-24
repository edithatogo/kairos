# Python 3.15 runtime migration and qualification

Maturity: Python 3.15.1 baseline integrated and handed over through PRs #297/#301/#302; provider quality follow-through remains open. This policy amendment does not advance the phase-closeout ledger.

## Objective

Prefer the newest suitable qualified dependencies and runtimes. Python 3.15.1
remains the accepted default; the human clarification of 24 October 2026 permits
necessary alternate versions for actual consumer/provider compatibility without
additional permission. Record the concrete reason and scope, qualify the actual
repo-local interpreter/dependencies, and preserve simulation contracts, working
latest-version environments and private work. A declaration alone is not proof.

## Inputs

The direct user dependency-policy clarification (24 October 2026), accepted
3.15.1 baseline, current remote main, authoritative owner claims,
active process/environment inventory and the [retained qualification evidence](prior-qualification.md).
The active conversation goal covers this repository migration and its paired
repository; a track is not a substitute for that goal.

## Outputs and requirements

- Inventory development, build, tests, automation, CI matrices, containers,
  bootstrap, package/build backends, native binding tools and helper scripts.
  Classify historical evidence and synthetic version-negative fixtures separately.
- Use the explicitly selected repo-local profile, including subprocesses. The
  current profile is 3.15.1 and its guards/locks/CI stay intact. A necessary
  alternate needs a concrete consumer, reason, newest suitable target and
  equivalent runtime/prefix, constraint/lock, package/native and hosted evidence.
  Remove unexplained stale defaults; do not remove a justified qualified
  compatibility lane simply because it differs from 3.15.1. Preview and
  out-of-profile candidates still require deliberate qualification.
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
actual selected-profile execution (default 3.15.1), coherent constraints/locks,
meaningful binding/package/helper tests, supported CI/container evidence and
no unexplained stale runners,
required PR checks, current-main readback and safe environment handover. Any
unavailable required target remains a blocker. Domain results and provenance
must be preserved. No completion from fixtures alone.

## Quality gates

Use existing Conductor validators and exact-command evidence; runtime and hosted
oracles below remain mandatory beyond static document checks.

## Release implications

Changing the supported Python line may break older consumers; document the
qualified compatibility change and any necessary retained compatibility lane.
No library release or native/clinical approval is implied.

Track 30 retains cross-language version-policy ownership; Tracks 06/13/15 and their current owners retain binding, CI and packaging contracts. Track 105 coordinates the complete Python migration beyond Track 30’s policy-only scope.
