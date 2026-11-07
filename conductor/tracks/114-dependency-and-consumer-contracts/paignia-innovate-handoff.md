# Paignia and Innovate provider supplement

Date: 2026-11-08. Status: planned, no new native/scientific acceptance. Owner: Track 114, coordinated with Track 108/issue #336 and the existing scientific-completion programme. Read the track spec, plan, test matrix and current bounded implementation workpack first. This supplement resolves the game-provider identity and adds consumer-specific checks; it does not duplicate the workpack or change statuses.

## Boundary

**Paignia** is the working name for the reusable Rust game capability incubated in `edithatogo/new-drug-reimbursement-game` Track T17. Its current crate IDs remain `uogto-game-core` and `uogto-game-solve`; proposed `paignia-*` packages and a standalone repository remain uncreated/unpublished until evidence permits them.

Kairos retains exactly one execution clock, state/resource authority, simulation RNG and replay implementation. Paignia owns generic game representation and qualified algorithms; Innovate owns innovation dynamics and game/payoff construction; UOGTO owns semantic contracts; Voiage owns decision uncertainty/VOI. None becomes a mandatory dependency of the minimal Kairos engine.

The optional Paignia execution adapter depends on Kairos plus the game core. Static solving must not require Kairos. Do not move scientific obligations out of view when moving solver code: Kairos S01-S04 claims and caller compatibility remain open until the receiving provider and actual consumers pass. Generic execution traces are not UOGTO games merely because an adapter exists.

## Bounded leaves, mapped into Track 114

All target tests below are future unless already found in the current workpack. Reuse existing guards first. Each leaf resolves exact source/test paths, changes one behaviour and touches at most five files. A missing test or zero-test filtered Cargo run is not a pass.

### [ ] KP01 — inventory game ownership without changing behaviour

Inspect `kairo-ecs-game-theory`, `kairo-ecs-game-ontology`, their public callers and current S01-S04 rows. Extend the existing consumer inventory with an algorithm/source/licence/caller/receiver table and a state for every promise: retained-execution, migrate-to-Paignia, compatibility-adapter or explicitly unsupported. Write a guard failure for a missing caller/disposition before accepting the table. Do not mark numerical work complete from a documentation inventory.

### [ ] KP02 — reject reverse core dependencies

In the current dependency-boundary guard, add a seeded direct and transitive dependency from minimal Kairos to Paignia/Innovate/Voiage and require rejection. The optional external adapter is permitted; the minimal engine graph is not. Check the resolved Cargo graph, including features/root patches, rather than grepping one manifest. Preserve the latest qualified provider pins and deployment constraints; no blind dependency upgrade.

### [ ] KP03 — native game/innovation execution fixture

Through the already accepted public event/resource/RNG interfaces, execute a bounded consumer-owned game/adoption case. Test deterministic tie ordering, single clock/state, resource conservation, repeatable seeded traces and explicit rejection of unsupported operations. Include a negative test where an adapter secretly advances a second clock or launches a Python process. Record actual runtime-dispatch evidence, provider/consumer SHA, feature set, artifact hashes and the real Cargo command/nonzero test count.

The same scenario must not be reimplemented inside Kairos. Innovation/clinical/payoff assumptions stay with consumers. A Paignia static fixture must separately pass with Kairos absent.

### [ ] KP04 — preserve inherited scientific obligations and compatibility

Attach receiver acceptance to S01 RK4 replicator order/control, S02 the named mixed-Nash algorithm and dominant-action counterexample, S03 actual correlated incentive constraints/objective and S04 supported general information-set CFR/CFR+. Retain unsupported states where an algorithm is unfinished. Pin the independent oracle and tolerances before inspecting final results. If accepted output semantics change, version them and preserve a compatibility/rollback receipt rather than silently replacing a public method.

### [ ] KP05 — consumer release gate

Require clean installed-consumer receipts from qualified Paignia/Innovate artifacts, not source-tree shims or a README import. Verify Arrow/Polars schema semantics, nulls, units, time bases and ownership lifetimes where exchanged. Qualification at the shared data boundary does not force dataframes into the minimal scheduler. Preserve native-only execution and all existing HPC/hardware limits; a CPU fixture does not certify distributed/GPU deployment.

## Coordination

Paignia extraction: reimbursement-game Track T17, issue #100 and PR #101. Innovation programme: Innovate PR #570. Decision outcomes/VOP promotion: Voiage issue #1286 and PR #1287. Central identity and policy: repository-standards PR #120, `docs/paignia-library-boundary.md` (proposed until merged).

No algorithm migration, public package, new repository, pin advance or completed scientific claim is delivered by this document. Record leaf evidence once and reuse it across owning tracks; avoid repeated human approval gates for routine agent implementation.
