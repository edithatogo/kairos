# ADR 0011: Simulation mechanisms, analytical methods and domain models

Status: accepted direction, 7 November 2026. Implementation and extraction tasks
remain pending in Track 114. This records the maintainer's requested scope;
it does not publish a new API, advance a dependency pin or qualify a method.

## Decision

Kairos owns the Rust simulation mechanisms: virtual time, events, deterministic
ordering and cancellation, entity/component state, identity, random streams,
resource lifecycle, checkpoints, replay and optional execution/telemetry adapters.
Generic spatial topology and route primitives may be optional Kairos modules.
No entity is required to have a physical location.

UOGTO owns game terminology, identifiers, ontology constraints and mappings to a
versioned executable subset. A separate Rust game library owns representation,
solution concepts, algorithms and solver diagnostics. Domain applications own
the particular games, empirical assumptions, payoffs and interpretation.

Reuse `uogto-game-core` and `uogto-game-solve`, already incubated in
`edithatogo/new-drug-reimbursement-game`. Pure game representation and static
solvers do **not** require Kairos. An optional game execution adapter depends on
both the game library and Kairos for dynamic execution. Kairos never imports the
game solver or a domain application into its minimal kernel. UOGTO validation
does not establish that a game has an equilibrium or that a solver is correct.

Economic evaluation is another explicit domain layer. Kairos records neutral
resource allocation and usage; a HEOR model supplies costs, health outcomes,
utilities, perspective, horizon and valuation assumptions. Voiage owns VOI
methods on appropriately paired outcomes/net benefits. Neither price nor QALY is
a primitive ECS component required by Kairos. Operations research also allocates
resources; allocation itself is not renamed HEOR.

## Package and repository boundaries

Establish independent Cargo packages and public interfaces before creating more
repositories. The game incubator can become an independently versioned library
after real consumers exercise its package boundary. A separate repository is
useful when release cadence or maintenance ownership warrants it, not a
prerequisite for sound architecture. Use two concrete consumers to test reusable
design, while allowing the first useful adapter to ship earlier.

The same rule applies to HEOR. Start with an explicit Rust valuation module and
one worked model. Extract a reusable package when a second model demonstrates
the shared semantics. Reuse existing cost data providers through adapters;
operational costing data does not by itself specify economic opportunity cost.

## Existing code and scientific obligations

The experimental `kairo-ecs-game-theory` and `kairo-ecs-game-ontology` crates
remain accurately labelled until a consumer-tested Rust replacement is ready.
Move or retire APIs through one compatibility decision and fixtures; do not
maintain two permanent solver implementations. This Rust-to-Rust extraction
does not relax ADR 0010: non-Rust Kairos cores remain unsupported immediately.

Scientific obligations S01–S04 travel with the game methods. A move, renamed
crate, ontology mapping or old completed track is not correctness evidence.
Track 110 retains those obligations until the receiving library has a named
owner, implementation, independent numerical oracle and supported result
contract, or the old API is explicitly withdrawn. Calibration and sensitivity
methods likewise remain optional analytical modules, outside the minimal event
kernel, and must finish their existing obligations before support is claimed.

## Consequences

- CareOps owns patient/staff/transport policies, physical movement models and
  healthcare model composition. It consumes Kairos as an ordinary dependency.
- Innovate owns diffusion, adoption, incentives and forecasting; only models
  needing simulation consume the Kairos adapter.
- Reimbursement applications own payer/manufacturer assumptions and economics;
  their reusable game machinery stays free of that vocabulary.
- Providers expose compact versioned contracts. Consumers own adapters and pin
  changes. Avoid cross-repository workspace paths, copied engine code, universal
  model interfaces and dependency cycles.
- Routine work uses one bounded task and focused checks. Numerical, public
  interface and release changes receive proportionate review at a coherent
  milestone; ordinary authorized work does not require new human approval.

See [ecosystem guide](../ecosystem.md), [consumer inventory](../../conductor/ecosystem-consumers.md),
[movement and resource contract](../../conductor/contracts/movement-resource-usage.md),
and [bounded workpack](../../conductor/execution/README.md).
