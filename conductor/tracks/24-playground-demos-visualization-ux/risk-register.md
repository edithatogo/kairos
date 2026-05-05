# Risk Register: Track 24 Playground, Demos & Visualization UX

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Wasm binary size balloons beyond acceptable page load budget | 3 | 4 | 12 | Add bundle-size budget to quality gates; lazy-load non-critical assets | playground-agent | Bundle exceeds budget by >10% |
| Screenshot-driven docs become stale as visualization changes | 4 | 3 | 12 | Regenerate screenshots in CI on viz crate changes; fail build if PNGs are missing | ci-agent | Screenshot regeneration fails or produces missing PNGs |
| Browser demo exposes performance worse than native, hurting adoption | 3 | 4 | 12 | Add a loading-state disclaimer; benchmark Wasm dispatch against native Rust in release notes | playground-agent | Wasm benchmark shows >3× native latency without documented reason |
| Playground claims features not yet implemented | 3 | 4 | 12 | Keep claim-versus-capability ledger; block release if demo overstates maturity | redteam-agent | Claim-capability ledger shows mismatch at release |
| Automation relies on unavailable tooling (e.g., Wasm pack, headless browser) | 3 | 3 | 9 | Use dry-run/allowed-failure lanes until toolchain versions are stable | ci-agent | Required toolchain missing on CI after 2 release cycles |
