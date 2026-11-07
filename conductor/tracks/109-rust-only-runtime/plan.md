# Track 109 implementation plan

Retire every executable non-Rust simulation core immediately and qualify supported native/Wasm adapters without silent fallbacks.

## Phase 0: Immediate enforcement

- [ ] 109.0.1: Inventory all production schedulers, RNGs, fake environment transitions and backend selectors; separate DTO/visualization/reference tooling.
- [ ] 109.0.2: Remove executable host cores and make missing/unsupported native operations explicit typed errors; no opt-in fallback escape hatch.
- [ ] 109.0.3: Add real Python C ABI operations where the current native contract supports them; keep richer unsupported APIs explicit.
- [ ] 109.0.4: Replace synthetic success tests with negative retirement tests plus distinct actual-native execution tests.
- [ ] 109.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Native product surface

- [ ] 109.1.1: Freeze native handle/event/RNG/error/schema contracts with ABI and lifetime tests; implement missing Rust exports before advertising host APIs.
- [ ] 109.1.2: Generate or consolidate adapters using C ABI/PyO3/wasm-bindgen; qualify Python, TypeScript/Wasm, C#, R, Julia and Go separately.
- [ ] 109.1.3: Package clean artefacts for each advertised platform and run installed-artifact dispatch/cancellation/replay/ownership tests.
- [ ] 109.1.4: Ensure examples and package metadata name supported APIs and fail when native artefacts or model capabilities are absent.
- [ ] 109.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Acceptance

- [ ] 109.2.1: Prove no retired production core is reachable using entry-point inventories and adversarial load failures.
- [ ] 109.2.2: Run hosted binding/platform suites, standalone downstream fixture and independent interface review.
- [ ] 109.2.3: Publish accurate support matrix and breaking-change guidance; preserve immutable historical releases.
- [ ] 109.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.

