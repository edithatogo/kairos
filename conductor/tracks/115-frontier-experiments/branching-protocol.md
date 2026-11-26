# Checkpoint Branching Held-Out Protocol

This is the first CPU-only comparison for the
[policy-branching question](../../frontier-programme-20261107.md). It tests
semantics before any runtime, memory, variance-reduction or novelty claim.
The [Rust fixture and oracle](../../../crates/kairo-ecs-bench/tests/held_out_checkpoint_branching.rs)
and this protocol are fixed together in the run's source commit. The
[handoff](handoff.md) records review and execution.

## Fixture

Start with two synthetic entities A and B. At tick 2, dispatch their services
in order A then B and consume each entity's first service-purpose draw.
Leave A and B's next services pending at tick 5 with equal priority, A
inserted first. Save the scheduler, entity/model state and addressed stream
cursors after both tick-2 dispatches.
Use root seed 78121 and replicates 9100 through 9111 inclusive. The first
`next_u64()` from `DeterministicStream::from_seed_u64(78121, replicate)`
supplies the run seed. Address each ChaCha8 stream by the full entity handle
and the literal purpose `service` or `routing`. Positions count 32-bit
words, with two words per recorded draw. A, B and the successor have
index/generation pairs (0, 0), (1, 0) and (1, 1) respectively. All
events have priority zero.

Capture the scheduler and world using their v1 JSON formats and each active
or previously used stream as an RNG checkpoint v1. The fixture keeps the
service counts per live entity and the ordered draw records in its own
in-memory checkpoint.

- **Control:** leave both pending services unchanged.
- **Intervention:** cancel B's pending service, retire B and allocate a new
  generation in its slot. Insert a routing event for B's successor at tick 4,
  consuming one routing-purpose draw, then its service at tick 5 after A.
  Each branch resolves its entity and event handles against its own state.
- **Restore:** in each arm checkpoint after consuming all events due through
  tick 4, before the tick-5 dispatches. Preserve the actual last dispatch
  clock in the control arm, which has no event at tick 4. Restore into fresh
  in-process objects. This is not a process-crash or distributed recovery test.

## Comparators and acceptance rule

1. Independently enumerate the expected live entities and dispatch order from
   the tape: A then B at tick 5 in control; routing at tick 4 and A then the
   successor at tick 5 in intervention. A cancelled or stale-generation event
   is a failure, not a tolerated difference. The successor must start from
   its new generation's service cursor zero despite B's advanced cursor.
2. Run each arm from tick 0 with conventional entity/purpose CRN, without
   checkpoint reuse. Require exact agreement with each checkpointed arm in
   ordered dispatches, final model and entity state, and raw integer draws.
   A's second service draw must also agree across arms. Probe that changes in
   one fork leave the other fork unchanged.
3. Set only A's service cursor to word 0 at the control's second cut. Its
   tick-5 draw must repeat its tick-2 draw, with both of B's draws intact.
   In a separate full replay, give both arms the same initial global stream.
   Require the same tick-2 prefix. The inserted routing draw must consume
   the control arm's word-4 draw and shift A's tick-5 draw to word 6. Retain
   the first failed assertion for every failing replicate.

Require zero failures on the frozen event, identity and replay comparisons
and reproduce the result independently. Only after semantic acceptance may
setup, fork, replay cost and retained/peak memory be compared using a separate
predeclared matched timing protocol. The fixture cannot establish
general model validity, empirical treatment effects, scientific novelty or
performance superiority. Automatic executor selection, full rollback and
hybrid evaluation remain separate gates.

## Run

```bash
cargo test --locked -p kairo-ecs-bench --features engine-checkpoint \
  --test held_out_checkpoint_branching -- \
  --exact checkpoint_branches_match_full_replay_with_addressed_streams --nocapture
```
