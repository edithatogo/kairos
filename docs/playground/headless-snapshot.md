# Headless Snapshot Playground Slice

This playground slice is anchored to `examples/viz/headless-snapshot/`.

## Local entry points

- Browser page: `website/playground/index.html`
- Fixture: `website/playground/headless-snapshot.json`
- Source example: `examples/viz/headless-snapshot/src/main.rs`
- Smoke validator: `node website/scripts/smoke-playground.mjs`

## Claim boundary

The page renders a committed headless visualization snapshot fixture. It does not claim live Wasm execution, production charting, or a complete visualization dashboard.

## Expected source output

Run:

```powershell
cargo run --manifest-path examples/viz/headless-snapshot/Cargo.toml
```

Expected summary:

```text
frame=12 entities=2 events=0 bounds=Some(FrameBounds { min_x_milli: 0, min_y_milli: 0, max_x_milli: 1250, max_y_milli: 500 })
```

If the Windows MSVC linker is unavailable, run the non-linking gate instead:

```powershell
cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml
```
