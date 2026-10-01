# Open Game Theory Ontology and Multi-Game Runtime Wave

Status: active implementation charter
Owner: ontology-agent + game-theory-agent + wave-manager-agent
Track range: 56-61

## Purpose

This wave turns the earlier monolithic game-theory concept into an auditable implementation sequence:

- a standalone open ontology repository surface,
- deterministic Turtle and JSON-LD ingestion,
- deterministic Rust component generation,
- feature-gated graph-relational ECS support,
- normal-form game execution over flat ECS arrays,
- extensive-form game execution over Entity ID graph relations.

The wave may not claim production game-theory runtime parity until Tracks 56-61 all record local validation, phase review, pushed commit SHAs, GitHub Actions review, and evidence manifests.

## Parity Targets

The minimum parity target is a practical subset used by established game theory and simulation tooling:

- normal-form games: players, strategies, payoff matrices, utilities, best responses, pure Nash equilibria, dominated strategy elimination;
- extensive-form games: decision nodes, chance nodes, terminal nodes, information sets, action edges, transition edges, terminal utilities, cycle rejection, backward induction over finite trees;
- ontology interoperability: stable semantic class identifiers, Turtle fixtures, JSON-LD fixtures, provenance records, deterministic internal IR;
- ECS integration: all graph links are represented as Entity IDs in components, never by self-references, raw pointers, or topology-owned Box allocations;
- code generation: generated Rust is deterministic, formatted, reviewed, and compiled in the workspace before any API claim.

## Evidence Policy

Every task-level commit must leave its owned gate set passing. Failing tests may be observed during TDD, but the failing state must not be committed.

Each phase closeout must record:

- task commit SHAs for every completed task in the phase;
- review command, review result, and accepted fixes;
- validation commands and results;
- pushed ref;
- GitHub Actions command and result;
- evidence manifest path;
- waivers, if any, with owner and expiry.

Evidence that is only documentation, scaffolding, or placeholder code cannot satisfy production runtime claims. A track can remain useful while still being blocked from `Done` if the runtime or CI evidence is absent.

## Release Wording Rules

Allowed before Track 61 closeout:

- planned,
- scaffolded,
- prototype,
- feature-gated,
- validated fixture,
- local deterministic parser/codegen/runtime slice.

Disallowed before Track 61 closeout:

- production-ready game theory runtime,
- complete open game theory ontology,
- best-in-class multi-game solver,
- fully certified extensive-form solver,
- ontology parity achieved.

## Dependency Order

1. Track 56 defines the evidence, claim, and review gates.
2. Track 57 creates ontology fixtures and parser IR.
3. Track 58 generates and compiles Rust components from the IR.
4. Track 59 adds graph relations behind the `graph-relations` feature.
5. Track 60 implements normal-form runtime components and solvers.
6. Track 61 implements extensive-form traversal, solvers, and certification.

Downstream tracks must not bypass their declared dependencies. If a dependency is incomplete, the dependent track may prepare tests and documentation but must record the unresolved dependency in its handoff.
