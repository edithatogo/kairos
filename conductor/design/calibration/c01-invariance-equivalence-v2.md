# C-01 exact execution equivalence addendum v2

Reviewed coordinator dispatch refinement, 5 October 2026. This supplements the
immutable v1 contract; all equality, negative, spill and physical coverage
oracles remain required. It changes only redundant actual pipeline execution.

Every one of the 216 reader-derived bundles is independently decoded and its
ordered raw array bytes and SHA-256 verified. All 3,888 configuration points
receive receipts. An in-process execution may serve another point only when
its exact ordered source NDJSON bytes (not merely hashes), profile, template,
validation policy, all normalization/sort limits, source code and toolchain
are identical. Compare bytes before reuse. Physical format/layout identity
alone neither proves nor prevents execution equivalence.

Receipts explicitly distinguish actual execution from equivalence alias, link
each alias to its actual execution and source fingerprint, and verify its
NDJSON digest equals that actual manifest input digest. Each actual execution
still compares all four normalized populations against the profile baseline
by exact bytes, ordered parsed records, counts, byte lengths and SHA-256, and
checks semantic accounting and observed spill counters. Each alias retains
its own physical bundle identity and matrix configuration. Missing or duplicate
points and references to missing executions fail closed.

No mutable cross-run cache, skipped-only pass or alias counted as an actual
run is permitted. Report measured actual executions and aliases separately;
the expected 108 actual executions is a prediction, not an acceptance count.
All 648 physical output checks, 24 adapter invocations, 216 source bundles,
independent C0 checks and both negative oracles remain mandatory. The local
Rust self-contained 108-run matrix is unchanged. Qualification dispatch must
bind both this addendum and v1 before executing the actual transport matrix.
