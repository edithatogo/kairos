# Risk Register: Track 27 Developer Experience & Reproducible Environments

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Dev environment not reproducible across operating systems | 4 | 4 | 16 | Nix flake or devcontainer definition with locked inputs; CI validates fresh checkout builds on all platforms | devx-agent | Fresh checkout fails `cargo build` on any supported OS |
| Container/devcontainer drift from CI environment | 3 | 4 | 12 | Use same base image in devcontainer and CI; scheduled rebuild to detect drift | ci-agent | Devcontainer and CI diverge by >1 dependency version |
| Toolchain version conflicts in dev environment | 3 | 4 | 12 | Lock all toolchains in `rust-toolchain.toml` + devShell; pin wasm-pack, wasm-bindgen, and binding toolchains | devx-agent | Toolchain version mismatch blocks developer build |
| Onboarding time >30 minutes to first `cargo build` | 4 | 3 | 12 | One-command setup script; benchmark fresh-clone-to-build time in CI | devx-agent | Fresh-clone-to-build exceeds 30min benchmark |
| Dependency cache invalidation on every checkout | 3 | 3 | 9 | Document `sccache` and `cargo-chef` usage; verify cache hits in CI | ci-agent | Cache miss rate >20% on incremental builds |
