# Test Matrix: Track 33 WebGPU Compute for Browser

Rule: a check is marked complete only when an artifact exists and the validation command is recorded here.

| Check | Status | Artifact | Validation |
|---|---|---|---|
| Track docs exist and render cleanly | complete | `conductor/tracks/33-webgpu-compute-browser/*.md` | `rg -n "WebGPU Compute" conductor/tracks/33-webgpu-compute-browser` |
| `crates/kairo-ecs-webgpu/` crate skeleton exists with `Cargo.toml` | complete | `crates/kairo-ecs-webgpu/Cargo.toml`, `src/lib.rs` | `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --no-default-features` |
| Track 33 host-only runbook is explicit about current fallback contract | complete | `conductor/tracks/33-webgpu-compute-browser/plan.md`, `handoff.md`, `test-matrix.md` | `rg -n "Next-harvest|host-only|backend not configured|not claimed" conductor/tracks/33-webgpu-compute-browser` |
| Native CI reports WebGPU unavailable safely | complete | `src/adapter.rs` | `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --no-default-features` |
| Buffer bridge validates WebGPU alignment constraints | complete | `src/bridge.rs` | `pwsh -NoProfile -File conductor/tracks/33-webgpu-compute-browser/validate-track33.ps1 -RunRuntimeTests` passes via `stable-x86_64-pc-windows-gnu`; this is host crate proof, not browser device proof |
| Dispatch orchestration scaffold exists with 256-thread workgroup math | complete | `src/dispatch.rs` | `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --no-default-features` |
| Fallback/parity metadata reports CPU reference contract without browser GPU | complete | `src/capability.rs`, `tests/parity_webgpu.rs` | `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --features webgpu --tests` |
| Browser WebGPU dispatch reports explicit not-configured error | complete | `src/dispatch.rs`, `tests/parity_webgpu.rs` | `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --features webgpu --tests` |
| Demo README/JS contract is explicit about CPU fallback and no performance claims | complete | `website/webgpu-demo/README.md`, `website/webgpu-demo/src/main.js`, `website/webgpu-demo/index.html` | `npm test --prefix website/webgpu-demo` |
| WebGPU WGSL ABM shader scaffold exists | complete | `src/shaders/abm_webgpu.wgsl` | `npm run validate:wgsl --prefix website/webgpu-demo` |
| CPU vs WebGPU parity harness exists | complete for host fallback | `tests/parity_webgpu.rs`, CPU fallback contract | `pwsh -NoProfile -File conductor/tracks/33-webgpu-compute-browser/validate-track33.ps1 -RunRuntimeTests` passes via `stable-x86_64-pc-windows-gnu`; browser WebGPU device parity remains blocked below |
| Demo page exists with canvas, agent count, backend toggle, metrics, and backend-not-configured label | complete | `website/webgpu-demo/index.html`, `styles.css`, `src/main.js` | `npm test --prefix website/webgpu-demo` |
| Demo has static smoke test | complete | `website/webgpu-demo/scripts/smoke.mjs` | `npm test --prefix website/webgpu-demo` |
| Browser-GPU-free WGSL subset validator exists | complete | `website/webgpu-demo/scripts/validate-wgsl-subset.mjs`, `docs/gpu-compute/webgpu-wgsl-subset.md` | `npm run validate:wgsl --prefix website/webgpu-demo` |
| Track 33 offline validator checks crate, demo, WGSL, and no-performance-claim boundaries | complete | `conductor/tracks/33-webgpu-compute-browser/validate-track33.ps1` | `pwsh -NoProfile -File conductor/tracks/33-webgpu-compute-browser/validate-track33.ps1` |
| Demo README exists | complete | `website/webgpu-demo/README.md` | `rg -n "npm test" website/webgpu-demo/README.md` |
| WebGPU WGSL subset doc exists | complete | `docs/gpu-compute/webgpu-wgsl-subset.md` | `rg -n "64-bit atomics|workgroups" docs/gpu-compute/webgpu-wgsl-subset.md` |
| WebGPU vs native comparison doc exists | complete | `docs/gpu-compute/webgpu-comparison.md` | `rg -n "Native GPU|Browser WebGPU" docs/gpu-compute/webgpu-comparison.md` |
| `wasm-pack build --target web` succeeds | blocked | no Track 09 Wasm binding package yet | blocked until wasm-bindgen/web-sys dependencies can be introduced and fetched |
| Browser WebGPU device initialization | blocked | no browser binding dependency yet | blocked until wasm-bindgen/web-sys dependency wiring and headless Chrome WebGPU validation are available |
| 100K-agent frame-rate claim | blocked | no hardware/browser run yet | blocked until real WebGPU backend and reference GPU/browser results are available |
| Cross-browser smoke test | blocked | no Playwright/Puppeteer setup in owned scope yet | blocked until browser test harness and installed target browsers are available |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
