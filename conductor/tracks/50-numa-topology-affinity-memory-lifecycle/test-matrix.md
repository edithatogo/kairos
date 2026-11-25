# Track 50 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Native topology and affinity | `cargo test --locked -p kairo-ecs-core --features numa --test linux_numa -- --nocapture` and [installed hwloc against sysfs](evidence/native-qualification/README.md#topology-and-affinity) | Two Linux tests and 29 matched topology records |
| Lifecycle allocation and contention | [Matched schedule, cancel and dispatch capture](evidence/native-qualification/README.md#scheduler-allocation) | Zero timed allocator calls and zero requested bytes after drop for 320 reserved schedulers |
| Core tests | `cargo test --locked --offline -p kairo-ecs-core --all-features` with Rust 1.99.1 | 74 cases passed on macOS |
| Shared conformance | `node tests/conformance/runner-self-test.mjs` and the same Rust core suite | [ADR 0028](../../../docs/adr/0028-opt-in-scheduler-reservation-and-hwloc.md#conformance) binds the scheduler and hwloc fixtures |
| C Data ownership | `cargo test --release --locked -p kairo-ecs-ffi --features native-data --test track70_phase2_ffi_polyglot --test track70_domain_c_data_consumers` and [compiled C and installed PyArrow consumers](evidence/native-qualification/README.md#native-c-data-lifetime) | 13 Rust cases and two native consumers passed |
| Miri safety | `cargo miri test -p kairo-ecs-core` when a configured Miri toolchain is available | Local nightly entry lacks its toolchain manifest |
| Multi-node locality and performance | Actual two-node page-placement and local/remote timings | Open. [Runtime inventory](evidence/native-qualification/README.md#runtime-routes-for-the-remaining-placement-check) |
| Full workspace | `just ci` and required exact-source GitHub Actions workflows | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
