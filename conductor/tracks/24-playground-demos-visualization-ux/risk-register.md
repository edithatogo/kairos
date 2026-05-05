# Risk Register: Track 24 Playground, Demos & Visualization UX

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Wasm binary size balloons beyond acceptable page load budget | 3 | 4 | 12 | Add bundle-size budget to quality gates; lazy-load non-critical assets | playground-agent | Bundle exceeds budget by >10% |
| Screenshot-driven docs become stale as visualization changes | 4 | 3 | 12 | Regenerate screenshots in CI on viz crate changes; fail build if PNGs are missing | ci-agent | Screenshot regeneration fails or produces missing PNGs |
| Browser demo exposes performance worse than native, hurting adoption | 3 | 4 | 12 | Add a loading-state disclaimer; benchmark Wasm dispatch against native Rust in release notes | playground-agent | Wasm benchmark shows >3× native latency without documented reason |
| Playground claims features not yet implemented | 3 | 4 | 12 | Keep claim-versus-capability ledger; block release if demo overstates maturity | redteam-agent | Claim-capability ledger shows mismatch at release |
| Automation relies on unavailable tooling (e.g., Wasm pack, headless browser) | 3 | 3 | 9 | Use dry-run/allowed-failure lanes until toolchain versions are stable | ci-agent | Required toolchain missing on CI after 2 release cycles |
| Static playground fixture drifts from the Rust visualization example | 3 | 3 | 9 | Run `node website/scripts/smoke-playground.mjs` and `cargo run --manifest-path examples/viz/headless-snapshot/Cargo.toml` before claiming the slice is current | playground-agent | Smoke output or Rust summary no longer matches the committed fixture |
| Local Windows linker setup blocks executable demo verification | 2 | 3 | 6 | Use `cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml` until MSVC `link.exe` is installed or available on PATH | playground-agent | `cargo run` remains required for release evidence but cannot link locally |

## Current mitigation notes

- The first implemented playground slice is static and fixture-backed, so it avoids Wasm runtime claims until Track 09 provides a browser execution contract.
- `website/playground/headless-snapshot.json` includes `claimBoundary` text and points to `examples/viz/headless-snapshot/src/main.rs`.
- `website/scripts/smoke-playground.mjs` fails if the page assets are missing, the fixture schema changes unexpectedly, source labels drift, or summary counts/bounds are inconsistent.
- On 2026-05-06, `cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml` passed, but `cargo run --manifest-path examples/viz/headless-snapshot/Cargo.toml` was blocked by the local Windows linker configuration.
