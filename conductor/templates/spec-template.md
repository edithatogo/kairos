# <!-- __SPEC_ID__: TRACK <NN>: <TRACK_NAME> -->

## Mission

<!-- REQUIRED per track. 1-3 sentences describing what this track delivers and why. -->

## Primary subagent

<!-- REQUIRED per track. List the owning subagent(s) from conductor/subagents.yaml. Use `+` to join multiple agents sharing ownership of the same track. -->

```text
<!-- e.g. core-scheduler-agent + ecs-agent + contracts-agent -->
```

## Dependencies

<!-- REQUIRED per track. List track IDs (e.g. "00", "01") and any non-track prerequisites. Link to conductor/tracks.yaml for the canonical graph. -->

```text
<!-- e.g. Track 00 skeleton. Contract phase can begin immediately after repo setup. -->
```

## Owned paths

<!-- REQUIRED per track. File-system paths this track exclusively owns. Must match conductor/tracks.yaml. -->

```text
<!-- e.g. crates/kairo-ecs-core, crates/kairo-ecs-state -->
```

## Blocked paths

<!-- MANDATORY per track. Paths this track must NOT modify. List at minimum: .github/ (owned by Track 13), bindings/ (owned by Tracks 06-11), and any paths owned by other tracks that are adjacent to this track's scope. -->

```text
<!-- e.g.
.github/ — owned by Track 13 (CI/CD)
bindings/ — owned by Tracks 06-11
crates/kairo-ecs-ffi/ — owned by Track 02 (FFI bridge)
-->
```

## Parallel-safe with

<!-- REQUIRED per track. List tracks that can execute concurrently after contract inputs are accepted. Reference conductor/parallel-execution.md for wave model details. -->

```text
<!-- e.g. Most tracks are parallel-safe after their contract inputs are accepted. See conductor/parallel-execution.md. -->
```

## Inputs

<!-- REQUIRED per track. Concrete list of documents, contracts, or artifacts this track consumes. -->

<!-- e.g.
- Accepted project identity and naming status.
- Relevant files under conductor/contracts/.
- Prior track handoff notes.
-->

## Outputs

<!-- REQUIRED per track. Concrete list of deliverables produced by this track. -->

<!-- e.g.
- Implementation in owned paths exists and is wired to the workspace.
- Tests or test-plan.
- Docs updates.
- Release notes or compatibility notes when public surfaces change.
-->

## Acceptance criteria

<!-- REQUIRED per track. Verifiable conditions that mark this track complete. -->

<!-- e.g.
- Owned paths are created and documented.
- Track lanes are updated when ownership or scope changes.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- handoff.md is completed before merge.
-->

## Release implications

<!-- MANDATORY per track. Describe what changes in this track mean for the release process: breaking vs additive, ADR requirements, regression gates, and which release stages (alpha/beta/RC/1.0) are impacted. -->

<!-- e.g.
- Any change to event ordering semantics or SimTime representation requires an ADR and is breaking.
- Adding new public API surfaces is additive and safe within the same major version.
- Performance regression in benchmark scenarios blocks release.
-->

## Non-goals

<!-- REQUIRED per track. Explicitly state what this track will NOT deliver, to prevent scope creep. -->

<!-- e.g.
- This track does NOT implement FFI bindings.
- This track does NOT provide visualization.
- This track does NOT ship package registries.
-->

## Quality gates

<!-- REQUIRED per track. List track-specific CI gates. Shared gates from conductor/quality-gates.md apply automatically. Track-specific gates must also appear in test-matrix.md. -->

<!-- e.g.
Use the gates in conductor/quality-gates.md. Track-specific gates must be listed in test-matrix.md.
- cargo fmt --all --check
- cargo clippy -p <crate> --all-targets -- -D warnings
- cargo test -p <crate>
-->
