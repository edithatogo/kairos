# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Integrity | Changing a local result byte makes its digest check fail. | The guard rejects changed benchmark bytes and rehashed parity failure. |
| False claims | Synthetic/local-placeholder data cannot satisfy a measured speedup claim. Source-only evidence cannot qualify native execution. | Both negative cases pass. Independent review checks what the retained evidence actually exercised. |
| Drift | Generated capability table must match its JSON. Dangling evidence and track IDs fail. | The table regenerates exactly and both dangling-reference cases reject. |
| Scope | The validator checks evidence structure and digest identity. | The table retains source and scope limits on historical receipts. |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.

