# KairoECS Conformance Fixtures

This directory owns the shared behavioral fixtures consumed by Rust tests and later by Python, R, Julia, TypeScript, C#, and Go bindings.

The bootstrap contract is:

- Flat JSON fixtures live in `conformance/fixtures/`.
- `conformance/fixtures/manifest.json` records which fixtures are ready and who consumes them.
- `conformance/fixtures/README.md` defines the fields and assertions each fixture family must keep stable.

The longer-form directory-backed shape described in `conductor/contracts/conformance-contract.md` remains the target runner contract. These bootstrap files are the concrete inputs for the current implementation wave.

Initial fixture priorities:

1. Deterministic event ordering by `(time, priority, sequence)`.
2. Cancellation without reordering remaining events.
3. Reproducible entity-derived RNG streams.
4. Arrow event-log schema compatibility.
5. FFI lifecycle parity once Track 02 is ready.

Downstream tracks should treat these fixtures as the source of truth for core behavior. Track 01 consumes scheduler and RNG fixtures, Track 02 consumes the FFI fixture, and Tracks 06-11 consume the same manifest without redefining the semantics locally.
