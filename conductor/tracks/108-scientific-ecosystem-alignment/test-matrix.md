# Test Matrix

Qualify minimal and optional feature closures, deterministic replay, numerical and schema contracts, and installed CareOps/Innovate consumers; record exact sources, resource receipts, agent review and required CI.

Before each phase closes run `scripts/validate_conductor_phase_gates.ps1` and `scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`; these planning checks do not substitute for native tests.

## Provider checks

Run the closure check from the repository root. It queries Cargo's locked
normal-dependency graph for host core, types and Arrow, plus WASM on its own
target. It checks both the default and no-default profiles and exercises the
exclusion predicate with actual optional-feature graphs.

| Contract | Focused Cargo selector |
|---|---|
| Minimal replay and ordering | `-p kairo-ecs-core --no-default-features --test conformance_fixtures` |
| Stable Arrow binding schema | `-p kairo-ecs-arrow --features arrow-native --test schema_compatibility` |
| Official Arrow type and nullability agreement | `-p kairo-ecs-arrow --features arrow-native --lib native_schema::tests` |
| Full-width time, reconstruction, IPC and Parquet | `-p kairo-ecs-arrow --features polars-native --test track70_phase1_native_tables --test schema_v2_contract` |
| Export lifetime across producer drop, streamed batches and argument rejection | `-p kairo-ecs-arrow --features polars-native --lib arrow_c_data::tests` |

Use `cargo test --locked` with the selectors above. For downstream acceptance,
record the installed artifact, provider revision, seed and numerical expectations,
backend actually dispatched, schema fingerprint, buffer lifetime and release
behavior for each consumer. See [consumer evidence and ownership](handoff.md#provider-qualification-and-consumer-readback--1-november-2026).

The [provider receipt](provider-receipt.json) records 30 passed Rust tests, exact source, input and output hashes, commands and measured resources. All six invocations exited zero.
