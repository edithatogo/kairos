# Test Matrix: Track 34 PDES & Parallel Execution

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist under `conductor/tracks/34-pdes-parallel-execution/` | yes | yes | yes | yes |
| `crates/kairo-ecs-pdes/` crate compiles with `--features pdes` | yes | yes | yes | yes |
| Sequential scheduler build is unaffected when `pdes` feature is disabled | yes | yes | yes | yes |
| `LogicalProcess` trait is defined and documented in `docs/pdes/logical-process-trait.md` | yes | yes | yes | yes |
| Event exchange protocol is documented in `docs/pdes/event-exchange-protocol.md` | yes | yes | yes | yes |
| GVT algorithm is documented in `docs/pdes/gvt-algorithm.md` | yes | yes | yes | yes |
| CMB null-message protocol is implemented for deadlock avoidance | no | yes | yes | yes |
| Sequential parity test passes — PDES final state matches sequential for partitioned worlds | no | yes | yes | yes |
| Sequential parity fixture compiles under `cargo check --tests` | yes | yes | yes | yes |
| PDES benchmark suite exists in `benches/pdes/` for 4/8/16/32 LP configurations | no | yes | yes | yes |
| Speedup of 2x+ on 4 cores versus sequential baseline | no | no | yes | yes |
| Speedup of 4x+ on 8 cores versus sequential baseline | no | no | yes | yes |
| GVT progresses monotonically under representative simulation loads | no | yes | yes | yes |
| Deadlock-free stress test passes (10,000+ ticks, random events, 8 LPs, timeout) | no | no | yes | yes |
| Deadlock-stress fixture compiles under `cargo check --tests` | yes | yes | yes | yes |
| `pdes-sequential-parity` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `pdes-gvt-progression` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `pdes-deadlock-free` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| All PDES gates pass on CI with `--features pdes` | no | no | yes | yes |
| Time Warp research spike findings documented in `docs/pdes/time-warp-spike.md` | no | no | yes | yes |
| PDES does not block release when feature flag is disabled | yes | yes | yes | yes |
| Cross-platform LP communication works on Linux, macOS, and Windows (thread-based) | no | no | yes | yes |
