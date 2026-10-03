# Track 47 Test Matrix

Commands below use the installed native macOS Rust 1.98.1 toolchain. They replace
historical Windows GNU commands in the earlier scaffold evidence; a native pass
is not recorded as a Windows pass. Hosted Actions remain a separate gate.

| Gate | Command | Required for |
|---|---|---|
| Production lookahead / queues / typed errors | `rustup run 1.98.1 cargo test -p kairo-ecs-pdes --features pdes --test production_protocol` | Implementation |
| Real sequential DES / ABM / mixed parity | `rustup run 1.98.1 cargo test -p kairo-ecs-pdes --features pdes --test production_parity` | Implementation |
| Legacy compatibility and full PDES crate | `rustup run 1.98.1 cargo test -p kairo-ecs-pdes --all-features` | Review |
| Feature disabled | `rustup run 1.98.1 cargo test -p kairo-ecs-pdes --no-default-features` | Review |
| GVT progression / 8-LP 10,000-tick adversarial deadlock | production protocol tests above | Review |
| Benchmark target | `rustup run 1.98.1 cargo check --benches -p kairo-ecs-pdes --features pdes` | Review |
| 4/8/16/32-LP strong / weak raw hardware profiles | `python3 benches/pdes/collect_evidence.py --help` describes collection command; execute collector against pushed source and immutable output path | Evidence |
| Shared conformance inventory | `pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1` | Review |
| HPC manifest and claim boundary | `node scripts/validation/validate-hpc-parity-evidence.mjs` plus collector manifest/checksum validation | Review |
| One-command Rust lane | `just ci` with Rust 1.98.1 on PATH | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| DAG | `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |
| Hosted checks | GitHub Actions on the exact final PR head | Merge |

Record command, working directory, source commit, toolchain, fixture/input hash,
exit status and artifact path in the handoff. Do not mark a planned gate passed.
Strict Git closeout requires a clean committed/pushed tree. Threaded host
throughput and oversubscription measurements do not certify Track 55 HPC parity.
