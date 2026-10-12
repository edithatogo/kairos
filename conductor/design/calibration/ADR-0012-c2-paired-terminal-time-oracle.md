# ADR-0012: C20 paired-flow terminal-time oracle

**Status:** versioned fixture amendment; C2 phase and acceptance gates remain open.

## Context

The historical v1 paired-flow fixture asserts that completed `WorkProgress` keeps
`completion_at = Some(tick 30)`. The Q3 runtime uses that field as the active
segment due time and clears it when `complete_timed` checkpoints terminal
progress. The terminal state instead retains completed state, full useful time,
zero remaining duration, and the completion lifecycle record with its causal
tick. The v1 assertion therefore reads an active scheduling field as completion
history.

## Decision

Keep `conformance/c20/paired_flow_c20.rs` byte-for-byte unchanged as historical
v1. Add `paired_flow_c20_v2.rs` with the same paired Macro/explicit-zero-Micro
scenario and existing parity, provider, service-stream, request association,
resource, context, and no-transit-event oracles. V2 asserts terminal
`completion_at == None`, `useful_elapsed == 30`, `remaining == 0`, and exactly
one `Completed` lifecycle record for each actual work/request at tick 30; that
record's progress snapshot must also show completed state and terminal elapsed
and remaining values. Full dispatch and final work/request parity between the
two scenarios remains asserted.

The paired runner selects a hardcoded fixture version and output location.
Version 1 remains the default historical fixture with its original bytes. Its
legacy `--expect red` path only recognizes missing production module files; it
does not classify the earlier missing-accessor compile failure. With production
modules available, v1 executes and fails its invalid terminal-time assertion,
which remains recorded as a failed semantic oracle. Version 2 is selected
explicitly with `--fixture-version 2 --expect green` and writes only to
`.artifacts/mvp/C2.0.red-tests.paired-flow-v2`. The two versions have separate
result directories and fixture hashes. Green requires the exact named test to
be the sole selected result: one passed, zero failed, zero ignored, zero
measured, with other tests counted as filtered.

## Boundaries

- V2 corrects the terminal-time observation while preserving the frozen v1
  fixture and all of its other semantic oracles.
- This runner proves one local deterministic paired scenario only. It does not
  close C2, hosted, release, or external acceptance gates.
- No FlowRuntime or WorkProgress scheduling semantics are changed.

## Verification

A v2 run records its selected fixture version, fixture hash, committed source
SHA, toolchain, Cargo command, exit status, and output hash under the
version-specific artifact directory. The retained v1 result remains a failed
historical oracle and is not rewritten as a pass.
