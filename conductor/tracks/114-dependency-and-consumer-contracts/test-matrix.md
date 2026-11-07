# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Dependency direction | Kairos production manifest/source cannot require CareOps; minimal consumer fixture runs without the CareOps repository. | Pending for this track |
| Consumer behaviour | Installed CareOps uses native provider capability and explicit immutable version; unsupported provider fails, no reference fallback. | Pending for this track |
| Feature closure | Minimal core excludes optional data/GPU/MPI/HTTP/binding stacks; optional profiles compile and function. | Pending for this track |
| Adoption | Old and new implementations match scientific/runtime contracts and measured value; licensing/security accepted. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.

