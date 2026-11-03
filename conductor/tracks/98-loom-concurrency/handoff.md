# Handoff

Updated: 2026-10-22. Maturity: planned; adoption decision pending.

## Summary

Separate dependency track registered. No runtime change, installation or accepted implementation phase.

## Files changed

Planning folder and coordinated registry/policy surfaces only.

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

Prospective replacement boundaries only; no runtime API changed.

## Tests added

Independent future tests specified in test-matrix.md; planning validators only this pass.

## Known risks

Value and behavioural gates remain unresolved. Preserve published contracts and explicit owner boundaries; see the dependency-specific specification.

## Follow-up issues

Record exact existing consumer, baseline and adopt/no-go decision in Phase 0. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Historical `$conductor-review` and accepted fixes are recorded in PR #353,
merge commit SHA `6920b813d23d75132fd5c28927782679101e3dd5` on main.
The historical pushed ref was `codex/track98-loom-qualification`.

The qualification packet and its review fixes were integrated in PR #353; its
bounded model receipts are recorded below. Formal phase and shared-registry
acceptance remain pending, including the explicit next-phase decision and
`pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
The merged defer evaluation does not mark production adoption complete.

## Phase 0 qualification — 1 November 2026

The PDES generation-bitset instance allocator issues IDs used to reject handles
from other instances. At source 956d8399339b8589bbb5bbdea28f4cc5539588a9,
two callers racing for the last available ID must produce one success and one
exhaustion, leaving the counter saturated. Track 78 prerequisite PR #317 merged
at ebf020706d92bff41232ebaed3b885dfacadfc6b.

The isolated test crate compiles the private production function body unchanged
against std and Loom atomic types. A deliberately broken load/store variant
issued the same ID twice. The checked CAS implementation passed seven modeled
executions with two threads, at most two preemptions and a 128-branch bound.
The native two-thread test checked the same invariant. The workspace allocator
was already correct; the negative control was injected only into the model.

The model instruments Arc, AtomicU64 load/store/CAS, spawn and join. A host
counter records iterations without affecting decisions. This model excludes
weak-CAS hardware-specific spurious failures, every other allocator state,
destruction of complete bitsets, the scheduler, Rayon, MPI, GPUs and transport
timeouts. It does not establish arbitrary system concurrency safety. The
native generation-bitset suite passed 11 tests.

**Decision proposed for integration: defer workspace adoption.** Keep the
isolated experiment reproducible. No production logic was removed; this one
13-line CAS loop needs a separate harness, source extraction, native oracle,
seeded negative control and lock (30 external locked packages). No additional
production consumer or approved CI build budget has been identified. A second
concrete synchronization contract or a production regression would justify
revisiting adoption. Phase 1 replacement tasks stay unaccepted.

Loom 0.7.2 (MIT, declared Rust 1.65) was the newest non-yanked stable release
at readback. Its crates.io checksum is
419e0dc8046cb947daa77eb95ae174acfbddb7673b4151f56d1eed8e93fbfaca.
Upstream tokio-rs/loom was unarchived; its last reported push was 20 February
2026. No optional Loom features were enabled. Actual qualification used
Rust/Cargo 1.99.1 on Darwin arm64. The isolated graph passed repository
cargo-deny advisory, ban, source and license rules, with unmatched workspace
policy entries reported as warnings. Other target runtimes remain unqualified.

From the repository root, run:
```sh
m=conductor/tracks/98-loom-concurrency/model/Cargo.toml
cargo test --locked --manifest-path "$m" -- --nocapture
cargo clippy --locked --manifest-path "$m" --all-targets -- -D warnings
cargo deny --manifest-path "$m" --config "$PWD/deny.toml" --locked check
cargo test --locked -p kairo-ecs-pdes --features time-warp --lib generation_bitset::tests
```
The initial isolated test build/run took 11.51 s and 314310656 bytes maximum
RSS. The reviewed incremental run took 1.21 s and 150552576 bytes including
Cargo/recompilation. The model test suite itself reported 0.01 s. These are
measurements of this host, not thresholds for hosted CI.

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns only this track folder in
/Users/doughnut/.codex/worktrees/track48-rollback/kairos, branch
codex/track98-loom-qualification. Native scope was local tests and read-only
registry/PR/consumer readback. The existing CareOps dependency-handoff row
for 98-loom-concurrency reports unresolved/pending. No consumer pin change is
needed for this isolated test. The integrator owns registry and phase closeout.
Next operation is protected PR integration after trusted-main coverage recovery.

Raw logs under /Volumes/PortableSSD/codex-builds/kairos-track98:

- `model.log` — 39d212220687833c5080f862e06ba2736725d52707f0eb5ddc0cdfa6333737bc
- `final.log` — b0cdd631196660916f47dbd7d40000e559d364fe8867e2e4831ecc0e5343c48a
- `native.log` — 5841a2c46167079b0365c8e89d7f3441e071f1d832c70016f8773f21a74783fb
- `clippy.log` — 9885347c0adc58fdbb06fe85dc94afd6c541067c4212ebfed9c0033e4c82f39f
- `deny.log` — 7e222cdec540776be54dcaec38b148690f2a73dee8782d223f9629882dfdb92d

Hosted review hardened the source extraction boundary: the model now uses
the following top-level test-module marker instead of searching for a closing
brace. The locked rerun again found the mutant and passed seven CAS executions.

## Current readback — 3 November 2026

Chat `01a19bed-7060-7910-af22-38cb31f51446` owns this bounded handoff readback in `/Users/doughnut/.codex/worktrees/kairos-track98-review/kairos`, branch `codex/conditional-tracks-reconcile-20261103`, based on `d7bdcd3`. Shared registry and runtime changes remain with their owners.

The earlier integration instruction is historical: [PR #353](https://github.com/edithatogo/kairos/pull/353) merged as `6920b813d23d75132fd5c28927782679101e3dd5`. From this worktree at `d7bdcd3`, Homebrew Rust/Cargo 1.99.1 on macOS arm64 ran `cargo test --locked --offline --manifest-path conductor/tracks/98-loom-concurrency/model/Cargo.toml -- --nocapture`, with `CARGO_TARGET_DIR=/Volumes/PortableSSD/codex-builds/kairos-track98-current-20261103`, `CARGO_BUILD_JOBS=2` and `CARGO_PROFILE_TEST_DEBUG=0`. Exit 0: three tests passed; the expected mutant panic was detected and the CAS model reported seven executions. The first offline attempt lacked `generator 0.8.11`; a locked online run populated the cache before the retained offline rerun. Log `/Volumes/PortableSSD/codex-builds/kairos-track98-current-20261103/model-current.log` SHA-256: `235a5c9ee784dd0b9698d655aad7af02d1683c9357a6c32ff82ee558e6e0b926`. Model lock SHA-256: `97795e3b0ee98086e297289e5bf8de31f1c2da8dfe466e5750724f8754ad337e`; `crates/kairo-ecs-pdes/src/generation_bitset.rs` SHA-256: `eb8a15153ac95666a3962e895ea6bfbbeb738191fc4b7a83c763ed00ae725a90`. This bounded model does not certify the whole engine or establish a production consumer; the prior defer decision and remaining review gates stand.
