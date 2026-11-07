# Optional game execution port: design constraints

Owner: Track 114 / K01.port with receiving T17.08 and UOGTO's executable
profile. This is a design input to the versioned adapter, not a released Rust
trait, ABI or accepted implementation. Exact symbols and supported operations
are settled in the bounded contract leaf before implementation. See the
[source/extraction map](game-extraction-map.md) and
[movement/resource contract](movement-resource-usage.md).

## Dependency and execution ownership

The optional game-owned adapter imports game core and Kairos public APIs. Pure
game core/solve import neither the adapter nor Kairos. The application composes
domain models, policies and valuation; Kairos owns one simulation clock, state,
event order, resource lifecycle and replay. Static solving needs no clock, ECS
world or simulation random stream.

Use a small execution port, not a universal model framework. A static game,
policy lookup or economic valuation must not be forced into ECS entities or
physical coordinates. Public game identity remains separate from a replaceable
runtime mapping to full generational entity/event handles.

## Records and lifecycle

| Boundary | Required semantics |
| --- | --- |
| Game specification / policy artifact | Executable-profile and schema version, immutable model/parameter identity, supported game class, player/action/information-set namespaces, solver algorithm/version and selected policy. Reject unknown versions and unsupported constructs. |
| Observation | Session, player/information-set identity, observation/state revision, permitted information and legal actions. A live policy sees only its allowed observation, even if an offline solver analyzed the complete game model. No accidental full-world view or opponent-private-state leak. |
| Proposed action | Policy identity, observation revision, action ID, intended effective simulation time and a unique command identity. Validate legality, authorization in the domain model, stale revisions and timing before mutation. |
| Committed transition | Native event identity/order, before/after revision, chosen action/chance outcome and committed effects. Rejected, duplicate, cancelled or stale commands cannot apply a resource effect twice. Resource admission failure is an explicit modeled outcome, not a partial successful action. |
| Outcome and usage | Realized domain observations plus neutral usage intervals/quantities. Keep expected solver payoffs, realized outcomes and economic valuation distinct; preserve strategy/scenario/draw/provenance through optional HEOR and Voiage exchange. |

Do not replay `Solution.trace` as events: the incubator's current trace includes
postorder visits to hypothetical alternatives. The adapter executes a selected
policy and declared chance process, generating a new causal execution trace.
Tie/equilibrium selection is part of the policy version. Replanning invalidates
stale decisions explicitly. Cancellation and delayed effects use Kairos's event
and resource contracts rather than a second queue in the game library.

The fixture must state same-tick action ordering and reject unsupported
simultaneous-action semantics. Do not quietly serialize a simultaneous game
into a sequential game. Bound zero-time policy/event loops and report an
explicit execution limit; solver iteration count and wall time are not virtual
simulation time. Continuous-game numerical time (`dt`) needs a declared unit,
tick conversion and discontinuity policy before it schedules native events.

## Randomness and uncertainty

Deterministic expectation-based solvers consume no random draws. A stochastic
standalone solver accepts an explicit RNG source/configuration and records its
algorithm, seed/stream policy and numerical version without depending on Kairos.

When executing inside Kairos, the adapter obtains named streams through the
native RNG contract. Separate solver/policy exploration from exogenous model
randomness by purpose and stable identity. Varying the solver iteration budget
must not advance unrelated patient-arrival, service or environmental streams.
Use stable mappings to `DeterministicStream::from_entity_purpose` only after
qualifying entity and purpose identity across construction order and branches;
calling that constructor alone does not prove common-random-number coupling.

The domain comparison declares which randomness is coupled across strategies
and which is intentionally independent. Preserve run, scenario, draw, entity,
purpose and stream-version provenance. Reordered entities, policy branches and
parallel workers must not silently change the declared coupling. A checkpoint
must include live policy state and its stochastic streams, or explicitly reject
resume for that unsupported capability. A frozen offline policy retains its
training provenance separately from simulation replay state.

## Minimum shared fixtures

The contract leaf fixes expected values and exact commands; the table defines
the distinguishing behaviours, not a completed test suite. Keep the positive
case tiny and public, then reuse it in the adapter and each actual consumer.

| Fixture | Distinguishing result | Owning implementation lane |
| --- | --- | --- |
| Two-action static game | Correct analytic decision/residual without Kairos in resolved normal/build closure | T17.01–T17.03 and relevant solver leaf |
| Hidden-information decision | Policy observation cannot reveal forbidden state; legal-action set is consistent within the declared information set | Game core/profile and T17.08 family |
| Policy plus one chance transition | Exactly the realized causal branch executes; diagnostic alternatives never mutate the world | T17.08 family |
| Delayed action, stale revision, cancellation and duplicate command | One scheduled effect at the declared time; rejected/repeated inputs leave state/resources unchanged | Adapter plus existing native event/resource APIs |
| Changed solver budget and reordered construction | Exogenous streams/coupled draws remain as declared; intentional independent streams remain distinct | Adapter RNG lane and consumer comparison fixture |
| Checkpoint/resume and branch | Supported live policy/native RNG/state resume reproduces the causal trace; sibling branches remain isolated | Adapter replay lane |
| Wrong profile/unit/perspective or unsupported simultaneous game | Explicit error before mutation or valuation; no hidden conversion/serialization | Profile and domain adapter |

Do not make every fixture a prerequisite for a static-only package. A dynamic
release claims only the supported lifecycle/features it actually exercises.
Any omitted operation must fail explicitly and remain on its owning completion
track; documentation cannot turn a failed runtime check into support.

## Performance and maintenance

Measure solver time, observation construction, marshalling, scheduling and
execution separately, then end to end on the same model. Keep repeated solver
work out of the per-event path when a qualified immutable policy can be reused.
Cache by the relevant specification, parameters, information/state revision and
solver policy; invalidation is a contract, not permission to use stale decisions.
Batch only where ordering and observation semantics are preserved. Arrow/Polars
is an optional large-result interchange path, not required for a two-action
decision. Benchmark the adapter cost before adding parallel or asynchronous
machinery.

Reuse existing graph, optimization and RNG building blocks after qualification.
Own one tiny compatibility fixture per interface and run the affected feature
closure on provider changes. Actual consumer pin adoption remains a
consumer-owned change. Kairos's CI must not require a reimbursement, Innovate or
CareOps checkout to prove its public native port.
