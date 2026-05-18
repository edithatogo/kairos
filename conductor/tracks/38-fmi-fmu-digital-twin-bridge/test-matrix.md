# Test Matrix: Track 38 FMI/FMU & Digital Twin Bridge

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --no-default-features` | yes | yes | yes |
| `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2` | yes | yes | yes |
| `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi3` | no | no | yes |
| `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features` | yes | yes | yes |
| `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features --tests` | yes | yes | yes |
| `cargo check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml` | yes | yes | yes |
| Dependency-free unpacked FMU layout validation reports missing platform binary directories | yes | yes | yes |
| Dependency-free unpacked FMU layout validation rejects invalid `modelDescription.xml` root/version markers | yes | yes | yes |
| Dependency-free unpacked FMU export validation reports missing package artifacts | yes | yes | yes |
| `modelDescription.xml` structural validation rejects duplicate names/value references and generated output-structure mismatches | yes | yes | yes |
| AAS structural validation rejects missing IDs and duplicate property IDs, plus duplicate submodel IDs and idShort values | yes | yes | yes |
| Digital-twin publication contract validation rejects invalid sample rates, topic prefixes, and non-finite values | yes | yes | yes |
| FMU shared library loads from `.fmu` archive | yes | yes | yes |
| Unpacked FMU layout detects `modelDescription.xml` and host binary path | yes | yes | yes |
| `fmi2Instantiate` + `fmi2SetupExperiment` + `fmi2EnterInitializationMode` success | yes | yes | yes |
| `fmi2DoStep` × 1000 without crash, memory leak, or state corruption | yes | yes | yes |
| `fmi2GetReal`/`fmi2SetReal` scalar round-trip preserves value | yes | yes | yes |
| `fmi2Terminate` + `fmi2FreeInstance` clean resource release | yes | yes | yes |
| `modelDescription.xml` generation passes FMI 2.0 XSD validation | no | yes | yes |
| FMU export: archive structure conforms to FMI 2.0 specification | no | yes | yes |
| FMU export: OpenModelica round-trip trajectory matches within 1e-6 tolerance | no | yes | yes |
| FMU import smoke test passes on Linux (x86_64) | yes | yes | yes |
| FMU import smoke test passes on macOS (arm64) | no | yes | yes |
| FMU import smoke test passes on Windows (x86_64) | no | yes | yes |
| AAS JSON descriptor validates against AASX Package Explorer schema | no | no | yes |
| Live data bridge: FMU output variable changes published to streaming topic | no | no | yes |
| Digital twin state synchronization: snapshot → diff → apply round-trip | no | no | yes |
| Digital twin stale-state detection triggers alert above threshold | no | no | yes |
| FMU subprocess sandbox: crash isolation works (FMU SIGSEGV does not kill host) | no | no | yes |
| FMI 3.0 `fmi3InstantiateCoSimulation` + `fmi3DoStep` success | no | no | yes |
| `modelDescription.xml` generation passes FMI 3.0 XSD validation | no | no | yes |
| Docs: FMI import guide exists and includes runnable example | no | yes | yes |
| Docs: FMI export guide exists and includes OpenModelica workflow | no | no | yes |
| Docs: AAS mapping reference documents component-to-submodel rules | no | no | yes |

## Current local validation

- Passing: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --no-default-features`
- Passing: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2`
- Passing: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi3`
- Passing: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`
- Passing: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features --tests`
- Passing: `cargo check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml`
- Passing: `cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`; the GNU toolchain avoids Git's `link.exe` shadowing on this host and executes the owned test suite successfully.
- Passing: `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --no-default-features`
- Passing: `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2`
- Passing: `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi3`
- Passing: `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`
- Passing: `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features --tests`
- Passing: `cargo +stable-x86_64-pc-windows-gnu check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml`
- Blocked: `cargo test --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2` remains gated until live shared-library FMU execution is available on a runner with FMI-compatible binaries.
- Passing: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/36-streaming-real-time-processing/validate-track36-40.ps1 -SkipCargoTests` covers FMI all-feature compile checks and bounded offline-claim documentation.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
