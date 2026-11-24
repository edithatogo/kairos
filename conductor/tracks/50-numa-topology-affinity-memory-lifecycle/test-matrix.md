# Track 50 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Native topology and affinity | `cargo test --locked -p kairo-ecs-core --features numa --test linux_numa -- --nocapture` on Linux, plus the [installed hwloc readback](evidence/numa-and-lifetimes-20261125.json) | Node and thread-affinity evidence |
| Allocation lifetime | [Scheduler drop, C Data release and single-chunk import](evidence/numa-and-lifetimes-20261125.json) | Bounded native lifetime evidence |
| Core/state tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-core -p kairo-ecs-state --features numa` | Implementation |
| FFI layout tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-ffi --features numa` | Review |
| Miri safety | `cargo miri test -p kairo-ecs-core` | Done when available |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
