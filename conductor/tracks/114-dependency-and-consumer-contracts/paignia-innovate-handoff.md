# Paignia and Innovate provider supplement

Date: 2026-11-08. Status: planned, no new native/scientific acceptance. Owner: Track 114, coordinated with Track 108/issue #336 and the existing scientific-completion programme. Read the track spec, plan, test matrix and current bounded implementation workpack first. This supplement resolves the game-provider identity and adds consumer-specific checks; it does not duplicate the workpack or change statuses.

## Boundary

**[Paignia](https://github.com/edithatogo/paignia)** is the maintainer's chosen
name and actual standalone repository for the reusable Rust game library.
Transfer the existing independent seed to `paignia-core` and `paignia-solve`
there; the former incubator's PR 101 becomes the immutable dependency switch
plus a bounded reimbursement application call. Provider T17 leaves retain their
IDs in Paignia; application .09/.11 and the second consumer .10 keep their local
owners. Registry publication and reusable-release acceptance remain separate.
Other `paignia-*` adapters remain proposed.

Kairos retains exactly one execution clock, state/resource authority, simulation RNG and replay implementation. Paignia owns generic game representation and qualified algorithms; Innovate owns innovation dynamics and game/payoff construction; UOGTO owns semantic contracts; Voiage owns decision uncertainty/VOI. None becomes a mandatory dependency of the minimal Kairos engine.

The optional Paignia execution adapter depends on Kairos plus the game core.
Static solving must not require Kairos. S01-S04 correction/withdrawal obligations
remain open per method, with source/API disposition in Track 110/K01.games and
receiving implementation in T17.04-07. A bounded static release needs only the
methods it actually claims; it does not wait for general CFR or dynamic execution.
Generic execution traces are not UOGTO games merely because an adapter exists.

## Acceptance refinements mapped into existing leaves

KP labels are cross-references, not extra tasks or a completion ledger. Keep
status only in the owning track. T17 has 17 leaves; the first static path is
T17.01-03 -> two actual application cases T17.09/.10 -> scoped eligibility T17.12.
Reimbursement and Innovate can supply those cases. CareOps is an optional
additional consumer, not an obligatory prerequisite.

| Alias | Authoritative existing work |
| --- | --- |
| KP01 | K01.games / 114.1.5; [source extraction map](../../contracts/game-extraction-map.md); T17.01 |
| KP02 | K01.features / 114.2.3; T17.02 dependency isolation |
| KP03 | K01.port / 114.1.10 for optional dynamic games; 114.1.9 and Innovate I01-I09 for runtime dispatch |
| KP04 | Track 110 S01-S04 / receiving T17.04-07; T17.17 only for information-set scope |
| KP05 | K01.fixture / 114.1.2 and 114.3.1; T17.03/.09/.10/.12; optional dynamic receipts separately |

All target tests below are future unless already found in the current workpack. Reuse existing guards first. Each leaf resolves exact source/test paths, changes one behaviour and touches at most five files. A missing test or zero-test filtered Cargo run is not a pass.

### KP01 — inventory game ownership without changing behaviour

Complete the [existing source map](../../contracts/game-extraction-map.md),
including exact caller/API dispositions and feature-aware baseline execution.
Retain per-artifact source/licence/commit records; canonical UOGTO and legacy
Kairos ontology namespaces are not interchangeable. The current inventory is
source evidence and does not close numerical or package work.

### KP02 — reject reverse core dependencies

In the current dependency-boundary guard, add a seeded direct and transitive dependency from minimal Kairos to Paignia/Innovate/Voiage and require rejection. The optional external adapter is permitted; the minimal engine graph is not. Check the resolved Cargo graph, including features/root patches, rather than grepping one manifest. Preserve the latest qualified provider pins and deployment constraints; no blind dependency upgrade.

Run minimal/default/each advertised optional profile in standalone consumer workspaces, including normal/build dependencies and feature-unified transitive closure. A provider workspace all-features build cannot prove minimal isolation. Test static Paignia and deterministic Innovate without Kairos separately from actual native-dynamic consumers. Qualification-candidate locks and reproducible release locks remain distinct; record exact closures rather than imposing an untested common version.

### KP03 — native game/innovation execution fixture

Through the already accepted public event/resource/RNG interfaces, execute a bounded consumer-owned game/adoption case. Test deterministic tie ordering, single clock/state, resource conservation, repeatable seeded traces and explicit rejection of unsupported operations. Include a negative test where an adapter secretly advances a second clock or launches a Python process. Record actual runtime-dispatch evidence, provider/consumer SHA, feature set, artifact hashes and the real Cargo command/nonzero test count.

The same scenario must not be reimplemented inside Kairos. Innovation/clinical/payoff assumptions stay with consumers. A Paignia static fixture must separately pass with Kairos absent.

Keep Innovate's I07 optional-game result separate from I10 runtime qualification,
and CareOps R76.5 separate from R76.6/R77. Innovate G04 static research cases use
the qualified static provider; only dynamic cases add G03. UOGTO's SGC.08 static
semantic acceptance does not acquire SGC.06's execution prerequisite.

Extend the existing observation-port test with two world states differing only in opponent-private information. If the player has the same lawful observation and legal actions, its serialised policy input and action under identical policy randomness must agree. Seed a leak via a private debug field or action mask and observe failure before fixing projection. Do not give live policies world-state references or future event/RNG access. Offline solution algorithms can inspect the game without granting that access to players. Reuse Paignia T17.14/.17 and the existing Kairos public port; unsupported information structures fail before state mutation.

Extend the existing random-stream test with actual solver/policy exploration and exogenous draws. Increasing solver iterations must leave paired exogenous draws unchanged under the declared coupling. Reordered construction, rejected actions and a branch with no extra draw must follow the documented purpose/identity policy. Equal seeds alone are not the oracle. T17.15 owns the adapter-side guarantee; Kairos owns stream semantics. Static solver randomness is explicit and does not acquire a mandatory simulator dependency.

### KP04 — preserve inherited scientific obligations and compatibility

Attach receiver acceptance to S01 RK4 replicator order/control, S02 the named mixed-Nash algorithm and dominant-action counterexample, S03 actual correlated incentive constraints/objective and S04 supported general information-set CFR/CFR+. Retain unsupported states where an algorithm is unfinished. Pin the independent oracle and tolerances before inspecting final results. If accepted output semantics change, version them and preserve a compatibility/rollback receipt rather than silently replacing a public method.

Paignia result guarantees remain visible through any execution wrapper: game class, information structure, solution concept, algorithm/version, tolerance, residuals, selection and completeness. One returned solution is not all equilibria; a learned policy is not automatically a Nash solution. Preserve nonconvergence/unsupported status rather than converting it to economic zero or a successful replay. Kairos does not implement the replacement numerical solver.

### KP05 — consumer release gate

Require clean installed-consumer receipts from qualified Paignia/Innovate artifacts, not source-tree shims or a README import. Verify Arrow/Polars schema semantics, nulls, units, time bases and ownership lifetimes where exchanged. Qualification at the shared data boundary does not force dataframes into the minimal scheduler. Preserve native-only execution and all existing HPC/hardware limits; a CPU fixture does not certify distributed/GPU deployment.

Preserve parameter/model/equilibrium/measurement/simulation-replicate identities in consumer outputs where supplied. Simulation replicates are not posterior parameter draws. Voiage owns conditional-expectation/VOI arithmetic and the finite no-information-noise test; Kairos supplies explicit execution/replicate/coupling evidence, not a hidden valuation step. An analyst's draw is not implicitly observed by a simulated actor. Runtime events carry effective time; source knowledge dates remain a consumer observation contract rather than a second execution clock.

Keep native-core, data/binding and optional-integration acceptance separate. Every advertised capability retains its full tests, but unclaimed strategic/VOI research does not block Kairos core, a static Paignia slice, Innovate's deterministic release or the CareOps MVP. No pending scientific obligation is deleted. Each implementing leaf records a concrete input, expected result, prohibited shortcut and stop condition; unresolved scientific assumptions stop that leaf.

## Coordination

Paignia provider extraction: [Paignia Conductor](https://github.com/edithatogo/paignia/tree/main/conductor), preserving T17 IDs. Reimbursement consumer handoff: issue #100 and PR #101. Innovation programme: Innovate PR #570. Decision outcomes/VOP promotion: Voiage issue #1286 and PR #1287. Central identity and policy: repository-standards PR #120, `docs/paignia-library-boundary.md` and `docs/scientific-result-contracts.md` (proposed until merged). Paignia PR #1's Conductor refinement maps these requirements onto T17; strategic-information handoffs remain coordinated with Kairos PR #383 and Voiage PR #1288 rather than a duplicate programme.

CareOps's existing shared healthcare package and concrete generic mechanism
handoffs are recorded in the [composition review](../../design/careops-composition-plan-20261020.md#paignia-and-careops-boundary-review).
Keep that work in existing A0-A7/R75-R77 and K-C owner lanes. Source helper cleanup,
RNG contract review and experiment lifecycle design need not wait for optional
games or for a separate healthcare repository.

No Kairos algorithm migration, public package, consumer pin advance or
completed scientific claim is delivered by this document. Record leaf evidence
once and reuse it across owning tracks; avoid repeated human approval gates for
routine agent implementation.
