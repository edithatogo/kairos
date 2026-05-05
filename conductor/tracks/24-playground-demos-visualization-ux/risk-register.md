# Risk Register: Track 24 Playground, Demos & Visualization UX

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---|---|---|---:|---|
| Wasm binary size balloons beyond acceptable page load budget | Medium | High | Add bundle-size budget to quality gates; lazy-load non-critical assets | playground-agent |
| Screenshot-driven docs become stale as visualization changes | High | Medium | Regenerate screenshots in CI on viz crate changes; fail build if PNGs are missing | ci-agent |
| Browser demo exposes performance worse than native, hurting adoption | Medium | High | Add a loading-state disclaimer; benchmark Wasm dispatch against native Rust in release notes | playground-agent |
| Playground claims features not yet implemented | Medium | High | Keep claim-versus-capability ledger; block release if demo overstates maturity | redteam-agent |
| Automation relies on unavailable tooling (e.g., Wasm pack, headless browser) | Medium | Medium | Use dry-run/allowed-failure lanes until toolchain versions are stable | ci-agent |
