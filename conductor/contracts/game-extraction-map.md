# Game-library extraction map

Direction refined 8 November 2026 (Australia/Brisbane). Owner: Track 114,
`K01.games`; receiving provider: [Paignia](https://github.com/edithatogo/paignia),
with the existing game incubator's
[issue 100](https://github.com/edithatogo/new-drug-reimbursement-game/issues/100)
and [T17](https://github.com/edithatogo/new-drug-reimbursement-game/tree/main/conductor/tracks/t17_game_runtime_ecosystem_20261107).
This source inventory refines [ADR 0011](../../docs/adr/0011-ecosystem-boundaries.md).
**Paignia is the chosen library name and actual standalone repository.** This
inventory is not completed package acceptance. Its initial provider change owns
the `paignia-core`/`paignia-solve` seed transfer. Existing incubator PR 101 owns
the immutable dependency switch and real static reimbursement fixture.

## Source disposition on 21 November 2026

The receiving static and optional runtime packages are maintained in Paignia.
This inventory binds their bounded APIs at
[`d4047c0`](https://github.com/edithatogo/paignia/commit/d4047c0648a89f2aee4461ff70bbc9641e5320cd)
to the experimental Kairos sources at
[`235e6a3`](https://github.com/edithatogo/kairos/commit/235e6a333ccc6e690f5b41f2c5eaa9a6784fe147).
The legacy game crate sources and Cargo inputs match Kairos `40ba664`.
The following sections retain the original inventory and transfer history.

| Legacy API family | Source disposition and receiving contract |
| --- | --- |
| `StrategySpace` and `PayoffMatrix` constructors and accessors, `Utility`, `NormalFormError` | Replace for maintained finite games with Paignia `NormalFormGame`, scoped `ActionId` and `PlayerId`, and the selected method's payoff inputs and typed errors. Record the strategy order and each payoff owner explicitly during migration. |
| `BestResponseSolver::best_responses` and `BestResponse`, `PureNashSolver::equilibria` and `PureNashEquilibrium`, `StrictDominanceSolver::strictly_dominated_strategies` and `DominatedStrategy` | Retain the standalone response and dominance calls as experimental references, along with their result types. Migrate qualified pure Nash calls to Paignia's `pure_nash_equilibria` with an explicitly chosen `SolverConfig` tolerance. It returns strategy profiles. Reconstruct a required payoff vector from the caller's validated table using the same player order, and verify any intended exact ties with tolerance zero. |
| `ReplicatorDynamics::step_discrete`, `step_continuous_rk4` and `run_trajectory` | S01 replaces the continuous model step with Paignia's versioned checked Euler or four-stage RK4. Withdraw the legacy RK4 claim. Preserve the clipped, renormalized Euler computation as a historical baseline, including the trajectory helper that calls it with canned fitness. Retain `step_discrete` and `run_trajectory` as experimental references without a production replacement in this slice. The source suite does not exercise either legacy replicator step. [T17.04 numerical evidence](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/validation/replicator-rk4.md#execution-evidence) |
| `MixedNashSolver::solve_2x2_and_support_enumeration`, `LemkeHowsonSolver::solve`, `MixedStrategyProfile`, `nash_conv` and `exploitability` | S02 uses `solve_mixed_nash` and `evaluate_mixed_nash_candidate` for bounded support witnesses with typed status and scope. Paignia reports Lemke–Howson as unsupported. Withdraw the old equal-mixture and complementary-pivot claims. The legacy product CE shortcut calls the Lemke–Howson wrapper, which calls the 2×2 helper. Dispose of that caller before retiring the shared path. On an admitted candidate, the old Nash-convergence convention maps to `sum_gain` and average exploitability to half that sum. Invalid inputs require a typed rejection, since legacy `nash_conv` can return zero on undersized inputs. The old matching-pennies check leaves the dominant-action case uncovered. [Dominant-action oracle](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/validation/mixed-nash.md#independent-controls-and-provenance) |
| `CorrelatedEquilibriumSolver::verify_and_solve`, `ClarabelConvexSolver::solve_correlated_equilibrium_social_welfare`, its `CorrelatedEquilibriumResult` and `verify_correlated_equilibrium_incentives` | Withdraw the independent-product CE shortcut. For equivalent social welfare within one to eight actions per player, pass unmodified complete payoff matrices `A`, `B` and explicit coefficients `c = A + B` to Paignia `solve_correlated_equilibrium`. Read the receiving result and its typed status before using the joint weights. Map incentive-only verification to `evaluate_correlated_equilibrium_candidate` and its feasibility diagnostics. Use `evaluate_correlated_equilibrium_certificate` for primal–dual optimality evidence. Retain the traffic and Prisoner's Dilemma oracle cases for migration. [T17.06 contract](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/validation/correlated-equilibrium.md) |
| `solve_zero_sum_minimax`, `verify_minimax_residuals`, `ZeroSumMinimaxResult` and the re-exported backend `SolverStatus` | Retain the legacy minimax surface and independent tests as experimental. The accepted Paignia CE boundary selects a different objective. Each provider owns its result/status contract. |
| `ExtensiveNode`, `InformationSet`, `ActionEdge`, `ChanceOutcome`, `TerminalUtility`, `InformationSetFixture` and `information_set_fixtures` | Replace standalone game semantics with Paignia `GameSpecification`, native node and payoff types, and `InformationSetDeclaration` passed to `build_information_set_graph`. Bind the versioned source vocabulary at the UOGTO boundary. |
| `BackwardInductionSolver::solve`, `ChanceWeightedBackwardInductionSolver::solve` and `ExtensiveFormError` | Use Paignia `backward_induction_result` for declared static games. Preserve intentional sparse-zero payoffs, ordered actions, and the tolerance-aware tie policy and result version. |
| `CfrSolver::new`, `solve_kuhn_poker` and `CfrVariant` | S04 uses `train_counterfactual_regret` with a native specification, information-set declaration and a bounded positive iteration budget. Vanilla and CFR+ return the average policy and values and support receiving best-response and Nash-convergence diagnostics. Classify legacy `LinearCfr` and zero iterations as unsupported on migration. The legacy in-module test checks zero-iteration Vanilla and one positive-budget Kuhn CFR+ result. Preserve those historical controls. UOGTO records executable profile evidence in its T17.17 handoff. Track 110 owns source API migration and retirement for T17.07. [Kuhn and Leduc evidence](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/validation/cfr.md), [UOGTO profile](https://github.com/edithatogo/UOGTO/pull/139) |
| `ExtensiveFormTopology`, traversal stores and returned paths, `outgoing_action_edges`, `outgoing_chance_outcomes`, `extensive_form_paths` and `validate_extensive_form_topology` | Retain ECS-bound traversal as an experimental source reference. The optional `paignia-kairos` adapter projects the selected policy and chance path onto public scheduling, state and RNG interfaces. K01.port owns that runtime contract. |
| `ChildOf`, `TransitionTo`, `children_of`, `transition_target` and `depth_first_descendants` | Retain neutral state and relationship mechanics in Kairos. Game interpretation and payoff data remain with the adapter or application. |
| `OntologyClass`, `OntologyProperty`, `OntologyDocument`, `ParseError`, `parse_turtle`, `parse_jsonld`, `normalize_ontology`, `generate_rust_components`, `CodegenError` and the generated-component re-exports | Retain legacy tooling and its qualified source tests for reproduction. UOGTO owns the maintained executable profile and its fixtures. Withdraw the old scanners and generated-component layer from the receiving package boundary. |

### Compatibility and scope

Paignia's `GameSpecification.id` stores the full specification string.
Result records use `GameSpecificationId`. Player, action, node and set
identities are typed strings. Runtime entity generations stay in the
optional adapter. Preserve full IRIs and reject stale entity bindings.
Require an explicit choice of compatibility when migrating backward induction.
Kairos keeps `z` for equal-value actions ordered `[z, a]`. The receiving
input-order, pairwise-tolerance and lexicographic rule selects `a` in that
case. Kairos rejects zero-weight chance edges and can error on an omitted
decision payoff. Paignia's named profile admits zero-weight paths, checks
total mass within `1e-9` and interprets intentional sparse payoffs as zero.
Recall still checks a zero-weight path. The Paignia converter rejects these
legacy profile declarations. Adopt the receiving rules explicitly with
parity controls, or retain the legacy reference behavior where callers need
it. Any complete vector conversion retains each payoff owner and value.
Use the versioned result for provenance and keep the diagnostic trace with
the solver. A causal trace comes from dispatching the selected policy and
chance events.
[Receiving runtime contract](https://github.com/edithatogo/paignia/blob/d4047c0648a89f2aee4461ff70bbc9641e5320cd/docs/architecture/game-runtime-contract-v1.md)

### Verification and callers

Both locked source test commands passed with Cargo and rustc 1.99.1 on
macOS arm64. The game-theory command executes 45 tests and the
ontology command executes eight. The recorded logs are
`target/k01-evidence/game-theory-feature-baseline.log` and
`target/k01-evidence/game-ontology-feature-baseline.log`.

```bash
RUSTC="$(rustup which --toolchain 1.99.1 rustc)"
RUSTDOC="$(rustup which --toolchain 1.99.1 rustdoc)"
kairos_cargo="$(rustup which --toolchain 1.99.1 cargo)"
export RUSTC RUSTDOC
"$kairos_cargo" test --locked -p kairo-ecs-game-theory --features graph-relations,generated-components
"$kairos_cargo" test --locked -p kairo-ecs-game-ontology --features codegen
```

The locked package listings in
`target/k01-evidence/game-theory-package-files.txt` and
`target/k01-evidence/game-ontology-package-files.txt` contain 21 and nine
files respectively. Game theory's `generated-components` feature build
requires the external
`open-game-theory-ontology/fixtures/generated/rust/game_components.rs`.
Its re-exports include `Entity`, `PlayerId`, `StrategyId`, `ActionId`,
`InformationSetId`, `GameNodeId`, `Action`, `ChanceNode`, `DecisionNode`,
`Game`, `InformationSet`, `PayoffMatrix`, `PayoffOutcome`, `Player`,
`Strategy`, `StrategySpace`, `TerminalNode`, `Transition` and `Utility`.
Retain these as source-only experimental types. The ontology package omits
the inputs used by its packaged codegen tests. The receiving static provider
keeps both Kairos crates out of its dependency closure. [Paignia artifact verification](https://github.com/edithatogo/paignia/actions/runs/44758809445)

Kairos lists the two game crates as workspace members. Retain
`examples/normal_form_runtime.rs` and `benches/normal_form.rs` as experimental
comparisons because both call the standalone response and dominance APIs.
The normal-form and Clarabel oracle tests, the extensive-form and relation
tests, and the ontology ingestion, normalization and codegen tests retain
the legacy behavior in their respective suites. The bounded migrated pure
Nash calls use the explicit profile and payoff mapping above. A focused
manifest and Rust-source search found zero imports of the old crate names
in these inspected trees:

- Paignia `d4047c0` and reimbursement `b669bd0`.
- Innovate `fd7e259` and CareOps `23553b7`.
- Voiage `3fdac2f` and Dapane `8ef6c51`.

This inventory bounds the verified migration scope to those sources. The
reimbursement Chapter 8 receipt binds application `63a3801` to the distinct
provider artifacts at `bd880dd`. The feature tests preserve the legacy comparison baseline while Track 110 records
S01–S04 production API dispositions and source retirement. The native
execution-port acceptance, real consumer evidence and release qualification
have separate tasks.

## Historical source inventory

Kairos source: `018ef4975f7f146c9fa730381ac3c2bab8b15b60`. Receiving source:
`90602b93db7352bfb307bac7852db211cf94af1e` in
`edithatogo/new-drug-reimbursement-game`. The generic packages are already
`uogto-game-core` and `uogto-game-solve` at that historical snapshot. The Paignia
rename reuses those packages and tests; it does not create a parallel solver.

| Source and current API family | Destination and required action |
| --- | --- |
| `kairo-ecs-game-theory/src/normal_form.rs`: `StrategySpace`, `Utility`, `PayoffMatrix` and game representations | Reconcile with the receiving `Game`, `GameSpecification`, `NormalFormGame`, IDs and payoff representation. Keep representations in game core; representation changes require versioned conversion fixtures. |
| Normal-form best response, dominance, Nash, CE and replicator methods; extensive-form backward induction and CFR | One authoritative implementation in game solve. Retain analytical references, capability limits and S01–S04; Clarabel is an existing backend candidate, not a reason to write another optimizer. |
| `extensive_form.rs`: `InformationSet`, action/chance/terminal semantics | Represent independently of ECS in game core. The receiving core currently lacks the information-set representation needed for a general CFR claim. |
| `ExtensiveFormTopology`, traversal stores and paths exposing `ComponentStore` / `EntityId` | Optional game-owned Kairos adapter. Separate static topology/algorithms from runtime storage rather than moving the present crate wholesale. |
| `graph_relations.rs`: `ChildOf`, `TransitionTo`, generic traversal | Retain only demonstrated generic runtime utilities in Kairos, or replace adapter-local use through public state APIs. Game interpretation and game traversal policies belong outside the kernel. |
| `kairo-ecs-game-ontology`: `OntologyDocument`, `parse_turtle`, `parse_jsonld`, normalization and `generate_rust_components` | Executable-profile mapping/tooling coordinated with UOGTO. The specialized scanners are not a general RDF/JSON-LD implementation; qualify their declared subset or replace them with maintained tooling at build/input boundaries. |
| `SimTime`, `SimDuration`, generational IDs, scheduling/cancellation, state, RNG/checkpoints, resources and replay | Remain Kairos mechanisms. No solver, payoff valuation, ontology reasoner or domain model becomes a core dependency. |

The `graph-relations` feature currently imports `kairo-ecs-state` and
`kairo-ecs-types`; state imports core. Pure game packages must not inherit that
resolved dependency chain. No other production Kairos crate imports the two
game crates in the inspected tree; their local tests/examples/benchmarks still
need an explicit compatibility disposition. This observation does not prove
that external callers are absent.

The reimbursement application declares the generic crates as dependencies, but
its inspected Rust library and executable do not call them. That declaration
does not count as a real installed consumer. T17 must add an application case;
unit tests inside the provider cannot supply both consumer receipts.

## Immediate handoff and retained authority

The follow-up inspected Kairos `e76b97be2ea3712645933d903822ff6381c8883e`,
the incubator `f6c1c87b6b00875b33c1e19401c1de09481913ce`, and UOGTO
`a5f90557c40f054255a1d6c2541fd76f07aa5090`. The application statement above is a
dated baseline; PR 101 adds a source-level offer/acceptance consumer fixture.
That fixture alone does not close installed-package or two-consumer qualification.

| Move or retain now | Executable next step / receiving owner |
| --- | --- |
| Transfer the independent std-only core and core-dependent solve seed to Paignia, preserving tests and Apache-2.0 notices | T17.01–03 provider work moves to Paignia; incubator PR 101 replaces local implementations with an immutable dependency and actual reimbursement call. Record extraction provenance; package verification and publication remain separate. |
| Assign generic Kairos normal-form representation, best response, strict dominance and bounded solver work to Paignia | K01.games/T17 reconcile definitions and error contracts into one implementation. Do not copy a second pure-Nash implementation alongside the seed. |
| Select Kairos's existing Clarabel backend as a transfer candidate | Preserve its independent residual/incentive oracle tests and supported scope; establish an optional receiving backend boundary. Clarabel is currently unconditional in the Kairos game manifest. Qualify the receiving artifact before forwarding or retiring the old API. |
| Transfer S01–S04 correction responsibility and their source fixtures with the methods | T17.04–07 remain pending. Euler-under-RK4, limited mixed-under-Lemke, product-distribution CE and Kuhn-only CFR are not promoted by a move. Track 110 retains source characterization/withdrawal decisions. |
| Keep UOGTO ontology/IRIs, SHACL, competency queries and semantic examples in UOGTO | Consume a pinned executable profile through optional `paignia-uogto` mapping. Share licensed fixtures by provenance; semantic conformance is not numerical proof. |
| Keep UOGTO's Python inspector/reference playground out of production execution | Its RDF game inspector is not a solver; the current `LLMPlayer` chooses the first action. Retain accurate reference/demo labels or retire misleading claims. No host-language runtime fallback. |
| Split ECS topology/execution and restricted ontology input tooling before transfer | Optional `paignia-kairos` / `paignia-uogto` own those adapters. The core does not import Kairos, a reasoner or a second clock. |

Record source repository, immutable commit, path, original licence and
transformation for every copied artifact. The seed is Apache-2.0; Kairos source
is MIT OR Apache-2.0; UOGTO code is MIT while its ontology/documentation is
CC-BY-4.0. Retain per-artifact notices instead of applying a Rust package licence
to imported ontology assets. Referencing a pinned UOGTO profile is preferable
to copying its complete ontology into the core.

## Ontology identity and packaging defects to resolve

Kairos's bundled experimental fixtures use
`https://kairos.dev/ontology/game-theory#`; canonical UOGTO uses
`https://w3id.org/uogto/core#` and `https://w3id.org/uogto/extensions#`.
Matching local class names do not establish equivalence. The optional importer
needs an explicit versioned mapping with full-IRI collision/unknown-term negative
fixtures; branding must not rewrite ontology identifiers.

The bundled fixture manifests omit author/date/licence/review-commit fields
required by their own `provenance/POLICY.md`. Keep these artifacts labelled legacy
experimental until those fields are resolved. The generated Rust `include!`
reaches outside the crate root; independently unpacked package tests must prove
the replacement closure. Neither defect is repaired by moving the directory.

The repeated `graph_relations::children_of` whole-store scans are an extraction
performance candidate: construct a validated immutable adjacency snapshot once,
with explicit semantic/game/ECS identity mapping and topology invalidation.
Benchmark total snapshot-plus-solve cost and reuse before changing the reference
representation. See the [performance programme](../performance-programme.md).

## Resolve incompatible semantics before copying code

| Difference in inspected source | Required versioned decision / negative case |
| --- | --- |
| Receiving player/node/action IDs are strings; Kairos uses player positions, numeric information-set IDs and generational entity handles | Use an explicit namespace/version mapping. Reject duplicates, stale generations and unknown IDs; never truncate a runtime identity or assume array position is stable identity. |
| Receiving chance edges allow probability zero; Kairos chance construction rejects zero | Specify the executable profile's probability domain and tolerance. Test zero, negative, non-finite and non-normalized inputs; conversion must not silently change the model. |
| Receiving backward induction reads an absent player payoff as zero; Kairos errors | State whether complete payoff vectors are mandatory in the supported profile. Reject accidental omission rather than silently reinterpret it; intentional sparse-zero semantics need an explicit profile/version. |
| Receiving ties use tolerance and lexicographic action IDs; Kairos keeps the first maximal traversed action | Version numeric tolerance and tie/equilibrium-selection policy. Test reordered inputs and multiple equilibria; changed policy can change a consumer's decision. |
| Receiving `Solution.trace` records postorder analysis of alternatives | Preserve it as a solver diagnostic. Execute only a policy/chance projection through the [execution port](game-execution-port.md); an expected payoff is not a sampled realized outcome. |
| Kairos results expose backend status; the receiving result lacks a complete version/status/residual envelope | Define a stable library-owned result with algorithm/version, supported game class, status, tolerance, residual/exploitability and selection rule. Keep backend-specific diagnostics optional. Never turn nonconvergence or an unsupported class into an equilibrium claim. |

## Package and dependency work

T17.02–T17.03 own actual resolved normal/build dependency checks and standalone
artifact installation. Test the supported default, minimal and optional feature
profiles. Kairos's `generated-components` currently includes a Rust file outside
the crate root; ontology codegen tests also read repository-relative fixtures.
Unpacked package checks must expose those assumptions. Vendor or generate the
required supported artifact inside the owning package, or leave that feature
explicitly unsupported; source-checkout success is insufficient.

Keep Clarabel and other numerical backends in solver scope. Keep optional
serialization, Arrow/Polars, ontology tooling and Kairos out of the static
representation closure unless a supported feature expressly needs them.
Resolve the observed Rust-version declarations (Kairos 1.99; incubator 1.75)
through a supported build/feature matrix. These declarations are not proof that
every compiler/version combination works. No lock or provider pin changes in
this planning amendment.

## Delivery ownership and ordering

1. **T17.01 and K01.games:** reconcile the representation/result contract and
   exact callers. This map supplies the source inventory; complete feature-aware
   baseline checks and the API-by-API retain/replace/withdraw decisions before
   closing K01.games.
2. **T17.02–T17.03:** qualify the smallest useful static package and an unpacked
   standalone consumer. A bounded backward-induction capability need not wait
   for every future equilibrium method or a simulation adapter.
3. **T17.04–T17.07:** complete S01–S04 once in the receiving solver. Kairos
   Track 110 keeps source regressions and the transfer/retirement record. Do not
   implement the same corrected solver independently in both repositories.
4. **T17.08 family and K01.port:** bind the optional adapter to public
   Kairos APIs using the execution-port fixtures. Static package qualification
   does not depend on this dynamic lane; a dynamic claim does.
5. **T17 consumer leaves, CareOps and Innovate:** exercise real application
   models against exact package artifacts. Depend only on the methods/features
   each case uses. Record two distinct consumers before declaring reusable
   extraction qualified; repository creation and package publication remain
   separate operations.
6. **Compatibility leaf after receiving acceptance:** switch or withdraw the
   old experimental Rust APIs with named replacement/version notes and caller
   fixtures. Temporary forwarding wrappers may call the one accepted provider;
   they must not retain a second authoritative solver.

Each unresolved design produces one-behaviour, at-most-five-file implementation
leaves in the existing owning track before coding starts. Reuse the same tiny
fixtures and one review per coherent contract/numerical milestone. There is no
new cross-repository approval committee or status ledger.

Paignia now owns provider T17.01–.08 and .12–.17. Preserve those stable IDs during
the handoff; the former incubator retains historical receipts and application
.09/.11, while .10 executes in the chosen second application. One canonical plan
owns each method. The new repository's existence does not satisfy T17.12's two
installed-consumer acceptance, and no generic source fork should remain in the
reimbursement workspace after its dependency switch.

The [release gates](../release-gates.md) keep Kairos core, static games and
dynamic integration independently scoped. Moving or excluding a method does
not discharge its scientific obligation or permit a false support claim.

## Source and coordination references

- [Kairos game manifest and source](https://github.com/edithatogo/kairos/tree/018ef4975f7f146c9fa730381ac3c2bab8b15b60/crates/kairo-ecs-game-theory)
- [Kairos ontology tooling](https://github.com/edithatogo/kairos/tree/018ef4975f7f146c9fa730381ac3c2bab8b15b60/crates/kairo-ecs-game-ontology)
- [Receiving game core](https://github.com/edithatogo/new-drug-reimbursement-game/blob/90602b93db7352bfb307bac7852db211cf94af1e/crates/uogto-game-core/src/lib.rs)
- [Receiving solver and diagnostic trace](https://github.com/edithatogo/new-drug-reimbursement-game/blob/90602b93db7352bfb307bac7852db211cf94af1e/crates/uogto-game-solve/src/lib.rs)
- [Receiving application](https://github.com/edithatogo/new-drug-reimbursement-game/tree/90602b93db7352bfb307bac7852db211cf94af1e/crates/new-drug-reimbursement-game)
- [Innovate PR 570](https://github.com/edithatogo/innovate/pull/570), G01–G04:
  coordinated consumer programme, still a proposal at this inspection.
- [Repository standards PR 120](https://github.com/edithatogo/repository-standards/pull/120):
  central ownership policy proposal; policy checks do not accept consumer runtime.
