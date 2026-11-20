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
| `kairo-ecs-core` | `Scheduler` schedules and dispatches a selected transition. Its checkpoint and `from_checkpoint` restore the scheduling state. |
| `kairo-ecs-state` | `World::is_alive` checks the bound generational entity. The application provides the world that owns it. |
| `kairo-ecs-rng` | `DeterministicStream`, `derive_stream_id` and `RngCheckpoint` identify and resume named streams. The adapter uses the crate's re-exported `rand_distr::weighted::WeightedIndex` for chance draws. |
| `kairo-ecs-types` | `EntityId`, `EventId`, `EventKind`, `ScheduleRequest`, `DispatchedEvent`, `StepOutcome`, `SimTime`, `SimDuration` and `Versioned` carry native identities, time, dispatch results and checkpoint versions. |

The union of the four provider normal dependency trees contains 21 packages on
the tested host, including the four Kairos crates. Its dependencies cover
randomness, serialization, numeric utilities, error types and their supporting
crates. It excludes game and ontology crates, Arrow and Polars, GPU and MPI,
and host-language bindings. DES and ABM remain separate choices for
consumers that need resource or agent-behaviour APIs. The DES crate's normal dependency
tree contains 22 packages. The ABM tree contains 50 and includes spatial and
graph dependencies such as `geo`, `rstar` and `petgraph`.

## Verification

Cargo 1.99.1 on macOS arm64 returned exit 0 for the locked normal dependency
trees at the selected Kairos revision.

```sh
rustup run 1.99.1 cargo tree --locked -p kairo-ecs-des --edges normal --no-dedupe
rustup run 1.99.1 cargo tree --locked -p kairo-ecs-abm --edges normal --no-dedupe
```

In a fresh archive of Paignia
[`d4047c0`](https://github.com/edithatogo/paignia/tree/d4047c0648a89f2aee4461ff70bbc9641e5320cd/crates/paignia-kairos),
Cargo 1.99.1 compiled the four crates from the immutable Git dependency.
The following command returned exit 0 with 12 passing tests across the unit,
native dispatch, RNG isolation and policy resume targets.

```sh
rustup run 1.99.1 cargo test --locked \
  --manifest-path crates/paignia-kairos/Cargo.toml --all-targets
```

The local verification outputs are under `target/k01-evidence`, including
`kairo-ecs-des.normal.txt`, `kairo-ecs-abm.normal.txt`,
`paignia-provider.normal.txt` and `paignia-native.log`. The installed consumer
fixture and package qualification remain with K01.fixture. K01.port retains
the full operations, errors and fixture acceptance for the adapter.
[Bounded task definitions](../execution/maturity-tasks.json)
