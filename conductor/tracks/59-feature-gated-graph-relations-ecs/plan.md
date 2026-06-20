# Track 59 Plan

## Phase 0: Feature Boundary

- [x] Task 0.1: Add failing compile tests proving `graph-relations` APIs are unavailable by default. Commit after passing as `track 59 task 0.1: enforce graph feature boundary`.
  - Commit: `9c8a42c`.
  - Evidence: `cargo test -p kairo-ecs-game-theory --doc --no-default-features` passed with the `compile_fail` doctest proving `kairo_ecs_game_theory::graph_relations` is absent by default.
- [x] Task 0.2: Add the Cargo feature and module export behind `cfg(feature = "graph-relations")`. Commit as `track 59 task 0.2: add graph relations feature`.
  - Commit: `a2b4227`.
  - Evidence: `cargo check -p kairo-ecs-game-theory --no-default-features --tests` and `cargo check -p kairo-ecs-game-theory --features graph-relations --tests` passed.

Phase closeout: review, push, and GitHub Actions review.

## Phase 1: Components and Traversal

- [x] Task 1.1: Add `ChildOf` and `TransitionTo` component tests. Commit as `track 59 task 1.1: add graph edge components`.
  - Red evidence: `cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations` failed before implementation because `ChildOf` and `TransitionTo` were unresolved.
  - Green evidence: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations` passed with 2 tests.
  - Boundary evidence: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --doc --no-default-features` passed.
  - Local note: the default active MSVC toolchain can type-check the tests, but test linking is blocked because Git's `link.exe` shadows the Visual Studio linker on this machine.
- [x] Task 1.2: Add traversal over flat arrays using Entity IDs only. Commit as `track 59 task 1.2: traverse graph relations over ecs arrays`.
  - Red evidence: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations` failed before implementation because `children_of`, `transition_target`, and `depth_first_descendants` were unresolved.
  - Green evidence: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations` passed with 5 tests.
  - Boundary evidence: `rustup run stable-x86_64-pc-windows-gnu cargo check -p kairo-ecs-game-theory --no-default-features --tests` passed.
- [x] Task 1.3: Add no raw pointer, no self-reference, and no `Box` topology scan. Commit as `track 59 task 1.3: enforce no pointer graph topology`.
  - Evidence: `node scripts/validation/validate-graph-relations-no-pointer-topology.mjs --self-test` passed, including negative self-test coverage for raw pointer and `Box` patterns.
  - Regression evidence: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations` passed with 5 tests.
  - Boundary evidence: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --doc --no-default-features` passed.

Phase closeout repeats review, push, and GitHub Actions review.

## Phase closeout gate

Before any phase is accepted:

- Run `$conductor-review` for this track.
- Auto-apply accepted review fixes that are in scope for this track.
- Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Keep `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml`, and `conductor/status.md` synchronized.
- Commit and push the cleaned slice.
- Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
- Run `gh pr checks --watch` and record the GitHub Actions review result.
