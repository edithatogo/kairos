# Devil's Advocate Review

## Position 1: "Do not build KairoECS. Use Julia's existing ecosystem."

**Argument:** Julia already has strong scientific simulation packages, high performance, composability, and native data science workflows.

**Response:** This is a serious objection. KairoECS is justified only if the goal is a Rust-owned, safe, embeddable core with consistent semantics across Python, R, Julia, TypeScript, C#, and Go. If cross-language distribution becomes secondary, the project should narrow or stop.

**Roadmap patch:** Track 18 must benchmark and compare honestly against Julia alternatives, and Track 26 must document migration/interoperability mappings.

## Position 2: "Six language bindings are a trap."

**Argument:** Every binding adds packaging, support, documentation, and lifecycle burden. A weaker API in six languages is worse than an excellent Rust/Python product.

**Response:** Correct for release sequencing. Incorrect for architecture. Design the ABI/conformance layer for six languages, but publish stable packages gradually.

**Roadmap patch:** Track 15 now uses staged releases and maturity labels.

## Position 3: "DES and ABM equal priority will create a mushy API."

**Argument:** DES users want processes/resources; ABM users want agent behavior and spatial state. One API can fail both.

**Response:** KairoECS should have one core substrate but two ergonomic surfaces: DES trajectory/resource API and ABM behavior API. Equal priority applies to engine semantics, not a single blended user API.

**Roadmap patch:** Track 03 separates DES and ABM subagents and validates with Track 23 examples.

## Position 4: "ECS is a contributor detail, not a user value."

**Argument:** Simulation users may not care about ECS and may find it alien.

**Response:** True. ECS should be visible to Rust contributors and advanced users, but hidden behind idiomatic modelling APIs for analysts.

**Roadmap patch:** Product guidelines now avoid selling ECS to non-technical users.

## Position 5: "Arrow does not guarantee zero-copy."

**Argument:** Arrow helps, but FFI lifetimes, IPC, chunking, host library behavior, and process boundaries may still copy.

**Response:** True. The roadmap now says Arrow-first and batch-oriented, not universal zero-copy.

**Roadmap patch:** Track 04 and docs must document exact copy semantics per language.

## Position 6: "Visualization will eat the project."

**Argument:** WGPU/Bevy tooling, UI design, and browser demos can distract from simulation correctness.

**Response:** Visualization is useful for adoption but must remain optional and downstream of snapshots/telemetry.

**Roadmap patch:** Track 05 and Track 24 are non-blocking for core releases.

## Position 7: "The first release should be boring."

**Argument:** The best first release is deterministic scheduler + entity storage + C ABI + Python + Arrow, not every SOTA idea.

**Response:** Agreed. The SOTA plan should guide the project; the first public release should prove the central thesis.

**Recommended v0.1 hero path:**

```text
kairo-ecs-types
kairo-ecs-core
kairo-ecs-state
kairo-ecs-ffi
kairo-ecs-arrow preview
Python 3.10-3.14 preview
one DES example
one ABM example
one hybrid example
one reproducible benchmark
one docs site
```

## Release challenge ledger

| Objection | Capability needed to defeat the objection | Current owner | Release consequence |
|---|---|---|---|
| Use Julia instead | Honest comparative benchmark plus interoperability/migration map | performance-agent, interoperability-agent | Any Julia-comparison claim blocks until Track 18 evidence and Track 26 mappings are current |
| Six bindings are a trap | Staged package manifest, maturity labels, per-binding smoke/package checks | release-agent, binding owners | RC/1.0 must not describe all bindings as stable unless each published binding is green |
| DES plus ABM creates a weak API | Separate DES and ABM examples plus shared conformance semantics | des-api-agent, abm-api-agent, conformance-agent | Blocks broad "equal paradigms" claims until both example paths run |
| ECS is not user value | User docs lead with simulation workflows, not internal storage | docs-agent | Docs must avoid selling ECS as the main analyst-facing value |
| Arrow is not zero-copy | Exact copy/lifetime semantics by language and schema version | arrow-agent, binding owners | Any universal zero-copy claim is a blocker |
| Visualization will eat the project | Optional visualization and playground maturity labels | viz-agent, docs-agent | Visualization cannot block a core release or imply kernel readiness |
| First release should be boring | Narrow v0.1 hero path with explicit exclusions | release-agent | Public release notes must name preview/experimental surfaces and omit unsupported SOTA claims |

## Owner and freshness rule

These objections must be rechecked with the red-team report before beta, RC, or 1.0 planning. Any objection that becomes release-facing must be promoted to a blocker or warning in `reviews/red-team-report.md` with an owner, evidence path, and stage impact.
