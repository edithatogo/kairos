# Minimal Rust consumer contract

Track 114 uses the four native crates below for Paignia's bounded game-path
adapter. All four resolve to Kairos
[`235e6a3`](https://github.com/edithatogo/kairos/commit/235e6a333ccc6e690f5b41f2c5eaa9a6784fe147)
with their default feature selections. Paignia owns the game graph, selected
policy, chance distribution and observation handling. Kairos owns the
scheduler, entity lifetime and random stream primitives.
[Track 114 plan](../tracks/114-dependency-and-consumer-contracts/plan.md)

| Direct crate | Public API used by the consumer |
| --- | --- |
| `kairo-ecs-core` | `Scheduler` schedules and dispatches a selected transition. `Scheduler::checkpoint` captures a `Versioned<SchedulerCheckpoint>`. `Scheduler::from_checkpoint` restores it. `SchedulerStats` reports event counters and current virtual time. |
| `kairo-ecs-state` | `World::is_alive` checks the bound generational entity. The application provides the world that owns it. |
| `kairo-ecs-rng` | `DeterministicStream`, `derive_stream_id` and `RngCheckpoint` identify and resume named streams. The adapter uses `rand_distr::weighted::WeightedIndex` for chance draws. `RngError` and `rand_distr::weighted::Error` surface stream and weight failures. |
| `kairo-ecs-types` | `EntityId`, `EventId`, `EventKind`, `ScheduleRequest`, `DispatchedEvent`, `StepOutcome`, `SimTime`, `SimDuration` and `Versioned` carry native identities, time, dispatch results and checkpoint versions. `TypesError` carries version and time errors. |

The union of the four provider normal dependency trees contains 21 packages on
the tested host, including the four Kairos crates. Its dependencies cover
randomness, serialization, numeric utilities, error types and their supporting
crates. It excludes game and ontology crates, Arrow and Polars, GPU and MPI,
and host-language bindings. DES and ABM remain separate choices for
consumers that need resource or agent-behaviour APIs. The DES crate's normal dependency
tree contains 22 packages. The ABM tree contains 50 and includes spatial and
graph dependencies such as `geo`, `rstar` and `petgraph`.

## Verification

Cargo 1.99.1 and rustc 1.99.1 (`58cf8f9c3`) on macOS arm64 returned exit 0
for the locked trees. The DES and ABM commands ran in the Kairos checkout
with the Cargo manifests and lock at `235e6a3`. The four-root tree and adapter
tests ran in a fresh archive of Paignia
[`d4047c0`](https://github.com/edithatogo/paignia/tree/d4047c0648a89f2aee4461ff70bbc9641e5320cd/crates/paignia-kairos).

```bash
set -euo pipefail
export RUSTC="$(rustup which --toolchain 1.99.1 rustc)"
export RUSTDOC="$(rustup which --toolchain 1.99.1 rustdoc)"
cargo_199="$(rustup which --toolchain 1.99.1 cargo)"
evidence_dir="$PWD/target/k01-evidence"

for crate_name in kairo-ecs-des kairo-ecs-abm; do
  "$cargo_199" tree --locked -p "$crate_name" --edges normal --no-dedupe \
    --prefix none --format '{p}' > "$evidence_dir/$crate_name.rust199.raw.txt"
  LC_ALL=C sort -u "$evidence_dir/$crate_name.rust199.raw.txt" | wc -l
done

cd "$evidence_dir/paignia-source"
: > "$evidence_dir/paignia-provider.rust199.raw.txt"
for crate_name in kairo-ecs-core kairo-ecs-state kairo-ecs-rng kairo-ecs-types; do
  "$cargo_199" tree --locked --manifest-path crates/paignia-kairos/Cargo.toml \
    -p "$crate_name" --edges normal --no-dedupe --prefix none --format '{p}' \
    >> "$evidence_dir/paignia-provider.rust199.raw.txt"
done
LC_ALL=C sort -u "$evidence_dir/paignia-provider.rust199.raw.txt" | wc -l
"$cargo_199" test --locked --manifest-path crates/paignia-kairos/Cargo.toml \
  --all-targets --jobs 4
```

The test command compiled the immutable Kairos dependency and passed all 12
tests across the unit, native dispatch, RNG isolation and policy resume
targets. The local receipts in `target/k01-evidence` retain the
`*.rust199.normal.txt` trees, `paignia-native.rust199.log`, and the actual
compiler record in `paignia-rust199-target/.rustc_info.json`.

K01.fixture owns the standalone installed event, state and resource run.
K05.package owns the clean beta package install and upgrade job design.
K01.port owns adapter operations and errors, identity, lawful observation,
separate RNG purposes, checkpoint semantics, and positive and negative fixtures.
[Bounded task definitions](../execution/maturity-tasks.json)

## Standalone Rust fixture — 25 November 2026

The independent [Cargo fixture](../../tests/fixtures/minimal-consumer/Cargo.toml)
resolves `core`, `des`, `rng`, `state` and `types` at the immutable
`235e6a333ccc6e690f5b41f2c5eaa9a6784fe147` revision. Its build and normal graph
has 24 distinct package names including the fixture on macOS ARM64.
With cached dependencies, Cargo 1.99.1 built the fixture in 7.17 seconds
with debug information disabled. Execution dispatched the service
completion, released capacity and admitted the waiting job. The
[installed SDK lanes](https://github.com/edithatogo/kairos/actions/runs/45051171243)
separately exercise the release-candidate artifact on Linux x86-64 and macOS ARM64.

```sh
export PATH="$(dirname "$(rustup which --toolchain 1.99.1 cargo)"):$PATH"
cargo --version
.venv/bin/python scripts/check_minimal_features.py
.venv/bin/python -m unittest discover -s tests -p test_minimal_features.py -v
cargo run --locked --manifest-path tests/fixtures/minimal-consumer/Cargo.toml
```

The guard reads Cargo's resolved normal and build edges. Its negative case
locks a direct CareOps dependency and a build helper that pulls in Polars and
bindgen. The check rejects all three. The independently tested
[optional game adapter](game-port-acceptance.md) uses its own lock.
