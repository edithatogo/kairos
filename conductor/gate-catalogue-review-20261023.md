# Review: central Conductor gate catalogue

## Scope and ownership

Base: `5303641e84cdf6ac42f089a34b6e28ad68604f8a` (remote main).
Worktree: `/Volumes/PortableSSD/codex-worktrees/kairos-conductor-catalogue`.
The user authorized the catalogue repair and normal PR/checks integration.
This chat also owns the stopped toolchain qualification work. Its tracked diff,
untracked file hashes and status were retained and verified unchanged before
this repair. The expired claim was explicitly recovered with stopped-owner
evidence; no inference was made from age alone. The portfolio coordinator was
notified. The toolchain candidate remains isolated and uncommitted, including
its independent Rust-version edits near the start of quality-gates.md. Future
toolchain integration must preserve these catalogue definitions.

## Defect and correction

The registry names 245 unique required gates. Twelve existing dependency gate
definitions (93–104) used plain headings, while the unchanged validator requires
bold gate IDs. Three Track 68 registry IDs had no central definition. Bold the
existing headings without changing their acceptance text; add the three transfer
definitions from the owning specification and phase matrix. Existing Track 68
transit/parity/conservation oracles remain in the catalogue. Registry IDs,
validators, toolchains, runtime sources, implementation status and acceptance
records are unchanged. Catalogue completeness is not runtime acceptance.

## Verification

Review performed in this coordinator session using the Conductor review
procedure, not a separate-agent review. Repo-local `.venv/bin/python` actually
reports 3.15.1; PowerShell reports 7.6.8. No simulation seed applies.

- Before: `pwsh -NoProfile -File scripts/validate_track_no_skip_claims.ps1`
  exits 1 and reports the 15 missing IDs.
- After: the same command exits 0.
- Independent registry/Markdown scan: all 245 registry IDs present; each repaired
  ID defined exactly once.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: exit 0.
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`: exit 0,
  105 tracks, zero errors/warnings.
- `node scripts/validation/validate-free-compute-routes.mjs`: exit 0.
- `git diff --check`: exit 0.

No new test suite for this documentation repair: the existing validator provides
observed failing/passing evidence. Full Rust tests are not rerun locally; this
patch changes no Rust or dependency configuration. Required hosted checks must
pass at the final PR head before normal merge.

## Separate unresolved validation

`validate_conductor_setup.ps1 -SkipCargo` advances past the repaired catalogue
then exits 1 at the implementation git-closeout ledger: recorded commits are
missing locally or not contained in origin/main, including Tracks 68–70.
`validate-track13-metadata.mjs` separately rejects existing Track 52 status
`Completed`. These records and validators are unchanged and remain with their
implementation owners. No runtime acceptance, ledger waiver or all-Conductor-
checks-green claim is made by this repair.
