# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| S01–S08 | Use the named independent analytical/game/QMC/statistical oracles in scientific-completion.md, including degenerate and boundary cases. | Pending for this track |
| S09–S10 | Nonidentity outputs and real state transitions originate in Rust; scalar/batched equivalence, reset/seed/termination and lifetime tests. | Pending for this track |
| S11–S17 | Actual devices/transports/external FMUs/filesystems or explicit capability-unavailable result; no mock or fixture metadata accepted as native proof. | Pending for this track |
| Scientific claims | Every supported method has a measured residual/error criterion and held-out test; classify each unsupported method explicitly. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.

