# Track107 validation evidence

## Mainline candidate

- Worktree: `/Users/doughnut/.codex/worktrees/kairos-track107-mainline`
- Branch: `codex/track107-careops-staged-flow-20261027`
- Base commit: `797e969b4a3cd3605a617a32d0134d0a093aa06a` (`origin/main` at integration start)
- Rust toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`
- Python validator runtime: repo-local `.venv/bin/python`, Python 3.15.1; verified with `scripts/python_runtime.py`.
- Cargo.lock is unchanged from the mainline base.
- `cargo fmt --all -- --check`: passed on the mainline candidate; `fmt-mainline.log`.
- `cargo test --locked -p kairo-ecs-des --test careops_staged_allocation_compatibility`: passed, 4 tests; `staged-allocation-mainline.log`.
- `cargo test --workspace --locked`: passed on the mainline candidate; `workspace-test-mainline.log`.
- `just ci` with Rust 1.99.1/LLVM 23.1.1: formatting and clippy passed; 899 coverage-instrumented tests passed; core line coverage was 93.39% against the 90% requirement; workspace rustdoc and `cargo deny check advisories sources` passed. The final `cargo audit` step fails on three zstd advisories; full output is in `just-ci-mainline.log`.
- `Cargo.lock` SHA256: `db047fde8908ffd2995e66db50aec2d4c5f58f07704b1cb29cb94ea69d5c1ca5`; it is unchanged from base `797e969b4a3cd3605a617a32d0134d0a093aa06a`. The findings come through Polars 0.45.1 → `polars-arrow` → zstd 0.13.3 and zstd-safe 7.3.0; see `zstd-dependency-path.log`. No dependency or lock changes are in Track107 scope. A separate reviewed dependency task is required to remediate them; no advisory is allowlisted here.
- Mainline fixture, compatibility inventory, Conductor DAG, phase-gate, artifact, and track-document validators passed; see the `*-mainline.log` files.
- CareOps parent pin and lockfile have not been changed. R10.3 consumer migration and seeded replay qualification remain separate acceptance gates.
- The released-lease fix passed the focused 4-test target and formatting check at source commit `5897ebf95d5993d069d85a1fd70a1a62123c0873`; exact command output is in `review-fix-validation.log`. Two independent reviewer agents assessed that exact source commit; their scoped receipts are in `reviewer-agent-review.md`.

## Prototype receipts

The following logs were captured in the earlier prototype checkout. They are useful development receipts, not proof of mainline validation:

- Prototype worktree: `/Users/doughnut/.codex/worktrees/kairos-careops-flow-compat-5692`
- Prototype branch/base: `codex/careops-r10-3-flow-compat-20261027` at `d3a6d831ddbe5886078541468e3fbf08b6f8c4c9`.
- Prototype full workspace test: `workspace-test-final.log`.
- Prototype workspace check: `workspace-check-final.log`.
- Prototype focused tests: `staged-allocation-test.log`.
- Prototype format, rustdoc, fixture, compatibility, Conductor DAG, phase-gate, artifact and diff checks: respective unqualified log names in this directory.

The parent CareOps checkout and pin were not changed by the upstream candidate. Local checks do not qualify a parent pin change or complete CareOps R10.3. Hosted PR checks and merge evidence are also pending.
