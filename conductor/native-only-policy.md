# Rust-only execution policy

Effective: 7 November 2026. Authority: the maintainer's instruction to stop support for non-Rust cores immediately. Delivery owner: [Track 109](tracks/109-rust-only-runtime/index.md).

## Supported architecture

The authoritative simulation clock, event ordering and cancellation, engine state, resource arbitration, stochastic streams, checkpoint/rollback semantics and environment transitions execute in Rust. Rust compiled to WebAssembly is a Rust core. A Rust CPU backend is supported; a host-language CPU fallback is not.

Python, R, Julia, TypeScript/JavaScript, Go and C# may provide typed arguments, transport, array conversion, plotting and native/Wasm calls. They must not create a second scheduler, advance an independent simulation clock, substitute a host RNG for an engine stream, or compute replacement simulation outcomes. User-authored model descriptions must enter an explicit Rust-owned execution contract. External inference engines may be optional dependencies of a Rust adapter; they are not alternative simulation cores.

## Immediate compatibility break

The previous pure-language scheduler/environment demonstrations are retired. There is no supported gradual migration period, silent fallback, environment-variable escape hatch or automatic simulated-success path. Legacy entry points that cannot delegate faithfully to Rust fail with an actionable native-backend/retired-API error. Importing DTOs and reading already-produced result data remain valid.

A shared-library loader, version check, fixture-shape test or negative unavailable-backend test does not qualify a binding. Each binding needs an installed-artifact test proving actual Rust dispatch, cancellation, event counts, error propagation, seed replay and buffer/handle lifetime. Unsupported bindings may remain as explicit error surfaces or development adapters but may not be marketed or published as functioning engines.

The next package version containing this break needs a compatibility note. Existing immutable releases are historical artefacts; do not rewrite their tags or claim this policy changes already-installed copies. Main-source enforcement, hosted qualification, release publication and downstream uptake are separate events.

## Reference implementations

Independent SimPy, OpenSpiel, SciPy or other reference programs are permitted in test/oracle or benchmark tooling. They must run separately, carry explicit reference labels, and cannot be imported by production simulation entry points. Existing reference outputs remain useful historical evidence when their source, parameters and stochastic semantics are identified.

## Mandatory failure cases

Test missing native artefact, wrong architecture, unsupported ABI, missing symbols, bad handles, closed handles, invalid integer ranges and absent model support. A load failure must never run a host engine. An unsupported native model must never return a synthetic observation/reward or copied input as inference. Capability discovery must distinguish unavailable, implemented, executed, qualified and release-supported.

## Consumer boundary

Kairos is a dependency of CareOps Sim. Kairos must compile and test without CareOps code, paths, data, secrets, service credentials or a CareOps checkout. See [dependency and consumer contract](dependency-and-consumer-contracts.md). CareOps owns clinical/operational models and data interpretation. Its adapter selects an immutable Kairos version and declares required capabilities; an unsupported version fails instead of selecting a reference core.

## Evidence and ownership

Track 109 owns immediate retirement and native adapters; Track 110 owns real Rust simulation environments and scientific methods; Track 112 owns claim/evidence checks; Track 113 controls publication. Historical Track 06–11/63 completion records do not imply native-only acceptance. Only exact-source installed-consumer evidence can advance support.

