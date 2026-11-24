# Bounded native game port

K01.port binds the optional `paignia-kairos` adapter to Kairos’s public scheduler,
world and random-stream APIs. The [port design](game-execution-port.md) and
[source disposition](game-extraction-map.md) retain the wider lifecycle and
scientific work. The verified receiver is Paignia
[`d4047c0648a89f2aee4461ff70bbc9641e5320cd`](https://github.com/edithatogo/paignia/tree/d4047c0648a89f2aee4461ff70bbc9641e5320cd).
Its separate workspace lock selects Kairos
[`235e6a333ccc6e690f5b41f2c5eaa9a6784fe147`](https://github.com/edithatogo/kairos/commit/235e6a333ccc6e690f5b41f2c5eaa9a6784fe147)
for `kairo-ecs-core`, `kairo-ecs-types`, `kairo-ecs-state` and `kairo-ecs-rng`.
The core and solver use the [separate static dependency contract](minimal-consumer.md).

## Operations and errors

| Adapter operation | Native API or admission rule | Supported behavior and rejection |
| --- | --- | --- |
| `NativeGame::start` and `start_comparison` | `kairo_ecs_state::World::is_alive`, `kairo_ecs_types::EntityId`, `kairo_ecs_core::Scheduler::pending_events` | Admit the finite perfect-information profile, execution schema `1.0.0`, a live entity generation and an empty live event queue. Bad schema or graph/profile, blank identity, inactive generation, busy scheduler and unrepresentable chance weights reject before scheduling. |
| `observe` | `paignia_core::build_policy_observation` | A decision yields a turn with the owned schema, profile, typed binding and ordered legal actions. Other phases give `NotAtDecision`. The host retains builder errors and supplies the successful projection to its policy. |
| `next_transition_time`, `submit`, `realize_chance` and `terminal_payoffs` | `kairo_ecs_types::{SimTime, SimDuration, ScheduleRequest, StepOutcome, DispatchedEvent}` and `Scheduler::{now, schedule, step, stats}` | Use priority zero at exactly `now + 1` tick. Verify the native ID, time, entity and event kind before advancing the game. The reached terminal supplies realized payoffs. Foreign, duplicate and stale turns give `StaleDecision`. Illegal action, cancellation, wrong time and exhausted bounds reject in preflight. `UnexpectedDispatch` interrupts graph advancement after scheduling and stepping have been attempted. |
| `exploration_stream` and chance sampling | `kairo_ecs_rng::{derive_stream_id, DeterministicStream, rand_distr::weighted::WeightedIndex}` | Sample admitted `u128` weights only after chance preflight, including at a chance root. The transition carries its `chance_stream_id`. Policy exploration has its own stream. A stale turn or absent comparison binding rejects a request for an exploration stream. |
| `checkpoint` and `ReplayCheckpoint::resume` | `Scheduler::{checkpoint, from_checkpoint}`, `kairo_ecs_types::Versioned`, `DeterministicStream::{checkpoint, restore}` | Capture at a committed chance boundary. Restore to a fresh native scheduler with a matching versioned policy format and artifact. Reject an invalid boundary, version, policy, decoder or nonfresh target. Validation and decoding precede scheduler installation. |

The immutable specification, instance, session and policy artifact have
separate typed Paignia identities. Nodes and action labels stay in their
original Paignia namespaces. A `DecisionTurn` binds the in-process live trace
and observation revision. The host supplies the generational `EntityId` in
the borrowed world, which holds that lifetime for the run. The native event
supplies its own `EventId`, sequence and time. A `CausalTransition` records the
actual dispatch, with the decision or ordered chance-edge index and before
and after revisions. Solvers’ hypothetical branches stay in solver results.
The trusted host supplies lawful public labels, authorizes the actual policy
artifact and passes the successful owned observation and policy state to it.
Information-set and simultaneous execution are outside the admitted profile.

Comparison mode keys native streams by seed, specification, logical instance,
scenario and paired draw. The full logical node and versioned purpose separate
chance and exploration. Independent-policy chance adds the policy artifact,
as does exploration. Shared chance omits it. The baseline entity mode uses
`DeterministicStream::from_entity_purpose` on the full generation. The
[stream contract](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/validation/game-rng-isolation.md)
specifies the addresses and distribution versions.

In-process restore binds the original immutable world, specification and entity.
It restores the opaque native scheduler snapshot, including the counters and
history needed for future dispatch. It restores the entity stream’s saved
position or the comparison stream addresses and seed. Each restore decodes
branch-owned state through the host’s versioned codec, which owns any retained
policy RNG state. Each branch gets a fresh trace identity. The frozen solver
artifact retains its own training provenance. Mixed native queues, resource
mutations, simultaneous and information-set execution, live-decision capture
and durable or cross-version checkpoint interchange are outside this profile.
[Native scheduler snapshot contract](../../docs/adr/adr-k01-scheduler-checkpoint.md)

## Native fixtures

Paignia owns the receiving game fixtures. From its repository root, select
both Cargo and the compiler explicitly. Use the checked-in locks and default
features. The test receipts verify Cargo’s actual compiler version.

```sh
run_locked_tests() {
    version=$1
    shift
    RUSTC="$(rustup which --toolchain "$version" rustc)" \
    RUSTDOC="$(rustup which --toolchain "$version" rustdoc)" \
        "$(rustup which --toolchain "$version" cargo)" test --locked --offline "$@"
}
run_locked_tests 1.99.1 --manifest-path crates/paignia-kairos/Cargo.toml --all-targets
run_locked_tests 1.75.0 -p paignia-core --test policy_observation
```

- T17.08 uses six `native_replay` cases and the exact weight-grid unit control.
  The host selects `play` after a prior scheduler event at tick 20. The action
  is dispatched as `PGAC` at 21 and one sampled edge as `PGCH` at 22.
  Only a realized payoff of 2 or 6 is possible in this fixture. The zero-weight
  edge and alternate action never dispatch. Sixty-four seeds exercise both
  supported outcomes. Typed error controls preserve the relevant scheduler
  state, and separate runs with identical labels retain distinct trace identity.
- T17.15 uses two `rng_isolation` cases. The same frozen game supplies the
  shared-node weights. The test varies exploration from one to nine draws, policy
  identity, entity construction order and realized detour paths across 64
  seeds. It checks identical shared chance outcomes at the same logical node,
  plus separate stream identities and differing outcomes in independent mode.
- T17.16 uses three `policy_resume` cases. The counter policy codec exercises
  entity-bound and shared comparison streams, prior native dispatch and
  cancellation history, exact resumed events and a sibling with independently
  decoded state and a different payoff. The comparison checks a future
  exploration draw. Invalid policy bytes, binding, version and nonfresh
  scheduler controls check that the supplied scheduler stays intact.
- `policy_observation` exhaustively destructures the owned public record and
  compares it after changing unobserved chance, opponent decisions and terminal
  payoffs. It rejects unsupported profile and cross-binding requests without
  introducing an information-set or hidden-state runtime claim.

Local results on 21 November 2026 on macOS ARM64 were 12 adapter tests
passed on Rust and Cargo 1.99.1 and 32 static observation tests passed on
Rust and Cargo 1.75.0. The reruns used the documentation-only Paignia head
`df21731dca53da2e12bc7b3a053af5bbd5e6633c` with the same Cargo manifests,
locks, source and test inputs as the receiving revision above.

| Paignia workspace | Cargo.lock SHA-256 |
| --- | --- |
| Optional adapter | `f24c795046db5d403e393275f48d4d683623c252f77cf8de8baa67f6af9cac2c` |
| Static workspace | `29132d6218500e8523f6e2a83b63cd5229d68a8fffd0a531eea78a939d41c196` |

The receiving provider also passed the Linux adapter jobs on Rust 1.99.0
and stable. [Paignia provider CI](https://github.com/edithatogo/paignia/actions/runs/44758809445)

At Kairos `0772016e7e42ebe79532e96a9529b6aaa8619981`, the same explicit
Rust and Cargo 1.99.1 selection passed 130 native tests with
`run_locked_tests 1.99.1 -p kairo-ecs-core -p kairo-ecs-types -p kairo-ecs-rng -p kairo-ecs-des`
from the Kairos root. Its lock SHA-256 is
`6f0517d02ffb3555b410ff2ae97892b8c639341785c65b5ae3864dbb6374bde7`.
The Kairos package run verifies the APIs. The Paignia run exercises their
composition and qualifies the bounded T17.08, T17.15 and T17.16 port.

Track 114.1.10 retains the wider semantic and paired-outcome handoffs. Static
game and runtime-only consumers retain their separate dependency contracts.
Application adoption, pin promotion, resource composition and release
qualification require their own receipts.

## Receiving handoff — 25 November 2026

On Paignia `e675da1a3a8bf9fbf88c43d25171643965aa05e7`, a Git-free source
archive passed 12 locked adapter cases on macOS ARM64 with Rust and Cargo
1.99.1. The adapter lock remains `f24c795046db5d403e393275f48d4d683623c252f77cf8de8baa67f6af9cac2c`
and retains Kairos `235e6a333ccc6e690f5b41f2c5eaa9a6784fe147`.
Hosted Linux stable and Rust 1.99.0 passed on that same Paignia revision.
The resolved normal and build closure of `paignia-core` and
`paignia-solve` has eight package names and excludes Kairos.
[Native CI](https://github.com/edithatogo/paignia/actions/runs/45441909884) ·
[Paignia adapter and lock](https://github.com/edithatogo/paignia/tree/e675da1a3a8bf9fbf88c43d25171643965aa05e7/crates/paignia-kairos)

Use UOGTO's versioned semantic profile for identity and policy compatibility.
The adapter accepts its bounded finite perfect-information execution profile.
For a paired outcome, retain the game specification, scenario, paired draw,
policy artifact, native stream ID and declared coupling from each arm.
Voiage owns the receiving strategy and valuation contracts. Policies receive
the owned observation, and their exploration leaves paired chance unchanged.
The receiving fixtures cover stale turns, entity lifetime and exact
in-process chance-boundary resume. Mixed workloads, resource effects,
simultaneous moves, information-set dispatch and durable restore require
their own qualification.
[UOGTO executable profile](https://github.com/edithatogo/UOGTO/blob/3000c231b2c102e18bf7c3294bec498f00f1976c/docs/contracts/executable-information-set-profile-v1.md) ·
[Voiage outcome handoff](https://github.com/edithatogo/voiage/pull/1287) ·
[Paignia policy replay](https://github.com/edithatogo/paignia/blob/e675da1a3a8bf9fbf88c43d25171643965aa05e7/docs/validation/game-policy-resume.md)
