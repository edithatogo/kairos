# Checkpoint Branching: Proposed Held-Out Protocol

**State: Draft, not executed.** This is the first CPU-only comparison for the
[policy-branching question](../../frontier-programme-20261107.md). It tests
semantics before any runtime, memory, variance-reduction or novelty claim.
Independently review and freeze the final input tape, runner source, supported
checkpoint formats and oracle before opening the held-out run. Existing
[paired-draw evidence](../../execution/scientific/s06-completion.md) is a
previous, narrower result, not this experiment.

## Fixture

Start with two synthetic entities A and B. At tick 2, dispatch their services
in order A then B and consume each entity's first service-purpose draw.
Leave A and B's next services pending at tick 5 with equal priority, A
inserted first. Save the scheduler, entity/model state and addressed stream
cursors after both tick-2 dispatches.
Use root seed 78121 and replicates `9100..9112`; no optimisation data are
selected from these runs.

- **Control:** leave both pending services unchanged.
- **Intervention:** cancel B's pending service, retire B and allocate a new
  generation in its slot. Insert a routing event for B's successor at tick 4,
  consuming one routing-purpose draw, then its service at tick 5 after A.
  Branches use separate state and handles.
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
3. For the missing-cursor control, assert that A resumes at the wrong draw
   index specifically after restore. In the global-stream control, use the
   same stream in both arms; the inserted routing draw at tick 4 must shift
   A's second service draw across those arms. Ordinary differences between
   unrelated stream seeds do not pass either negative control. Retain all
   mismatches.

Require zero failures on the frozen event, identity and replay comparisons
and reproduce the result independently. Only after semantic acceptance may
setup, fork, replay cost and retained/peak memory be compared using a separate
predeclared matched timing protocol. The fixture cannot establish
general model validity, empirical treatment effects, scientific novelty or
performance superiority. Automatic executor selection, full rollback and
hybrid evaluation remain separate gates.
