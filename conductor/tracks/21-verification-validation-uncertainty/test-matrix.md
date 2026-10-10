# Test Matrix: Track 21 Verification, Validation & Uncertainty

## Required checks

| Check | What it proves | Alpha | Beta | 1.0 |
|---|---|---:|---:|---:|
| Docs page names the three terms | The page defines verification, validation, and uncertainty clearly. | yes | yes | yes |
| Evidence boundary is explicit | Only committed replayable artifacts count as evidence. | yes | yes | yes |
| Accepted artifacts are listed | Readers can see which artifacts support a claim. | yes | yes | yes |
| Replay/scenario fixture tie-in exists | The page links credibility claims to scenario and seed fixtures. | yes | yes | yes |
| VVUQ note is surfaced in public docs | The site navigation and scenario evidence page link the artifact-backed VVUQ note. | yes | yes | yes |
| Markdown link and lint check | The page renders and links cleanly. | yes | yes | yes |
| Artifact existence check | The referenced docs and track files exist. | yes | yes | yes |
| Red-team limit check | The page explains what the evidence does not prove. | yes | yes | yes |
| VVUQ scenario fixture check | The conformance runner validates the scenario/seed replay evidence fixture. | yes | yes | yes |
| VVUQ note fixture check | The validation note names the committed scenario, seed, replay fixture, comparison basis, required outputs, and uncertainty limits. | yes | yes | yes |
| Cross-track evidence-boundary guard | The aggregate Track 21-27 validator rejects missing artifacts, broadened claims, or unsynchronised docs. | yes | yes | yes |

## Local validation commands

```bash
test -f docs/trustworthy-simulation/verification-validation-uncertainty.md
test -f conductor/tracks/21-verification-validation-uncertainty/handoff.md
test -f conductor/tracks/21-verification-validation-uncertainty/test-matrix.md
test -f conductor/tracks/21-verification-validation-uncertainty/risk-register.md
rg -n "verification|validation|uncertainty|scenario|seed|replay|trace|evidence boundary" docs/trustworthy-simulation/verification-validation-uncertainty.md conductor/tracks/21-verification-validation-uncertainty
node scripts/validation/validate-vvuq-note.mjs
node scripts/validation/validate-track21-27-evidence-boundaries.mjs
node tests/conformance/conformance-check.mjs
node scripts/validation/validate-tracks21-27.mjs
```

## Current evidence - 2026-05-06

| Command | Result | Evidence |
|---|---|---|
| `node scripts/validation/validate-vvuq-note.mjs` | pass | Cross-checked `docs/validation/factory-bottleneck-v1-vvuq-note.md` against `conformance/fixtures/vvuq_scenario_replay.json`, the scenario manifest, the seed manifest, and `expected_kind_order`. |
| `node tests/conformance/conformance-check.mjs` | pass | Revalidated the ready conformance fixture set, including `vvuq_scenario_replay_v1`. |
| `node scripts/validation/validate-vvuq-note.mjs` | pass | 2026-05-11 rerun confirmed the VVUQ note still names `factory_bottleneck_v1`, `scheduler_ordering_v1`, `expected_kind_order`, and the required outputs. |
| `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` | pass | 2026-05-11 rerun confirmed the cross-track boundary still rejects broadened claims. |
| `node tests/conformance/conformance-check.mjs` | pass | 2026-05-11 rerun validated the ready fixture set, including `vvuq_scenario_replay_v1`. |
| `node scripts/validation/validate-tracks21-27.mjs` | pass | 2026-05-11 rerun passed all Track 21-27 focused checks. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.


## C4.1 bounded fixture preparation

- `cargo +1.99.0 test --locked -p kairo-ecs-calibration --test metrics_c41`:
  eight comparator/fixture controls pass; one named future report test ignored.
- Repeat on Rust 1.88 floor; explicit actual-output opt-in requires
  `C41_CANDIDATE_REPORT` and rejects comparator mocks.
- Pinned optional reference environment runs
  `python -m unittest discover -s conformance/c41 -p 'test_*.py' -v` and
  `python conformance/c41/generate_reference.py --check`.
- [Actual commands/versions/hashes and red controls](../../evidence/c4.1-preparation-20261005/README.md)
  qualify test preparation only. Production metric/sidecar/C-04 gates remain open.

## C4.2 actual runtime qualification

- `rustup run 1.99.0 cargo test --locked -p kairo-ecs-calibration`:119pass/4ignored; repeat on1.88floor passes.
- `rustup run 1.99.0 cargo clippy --locked -p kairo-ecs-calibration --all-targets -- -D warnings` and workspace Rustfmt pass.
- Actual `metrics_c42` report is source/commit/toolchain-bound; explicitly execute existing ignored `metrics_c41::c42_candidate_report` on both toolchains and independent exact Python comparator:42cases pass.
- False commit/toolchain and mutated metric controls fail as required; explicit100000points/side debug timing passes. [Exact receipts](../../evidence/c4.2-runtime-20261005/README.md) preserve source and output hashes. Public API, Arrow sidecars and C-04 remain open.

## C4.3 actual Arrow sidecar qualification

- Real private runtime residual/metric IPC file/stream and Parquet outputs reconcile through pinned independent PyArrow, complete joins/raw hashes and15 manifest negative controls.
- Rust1.99 both-feature calibration140pass/4namedignored; Rust1.88 C4.3 targets21pass; feature-minimal/default compatibility checked. Strict Clippy/Rustfmt pass.
- [Retained exact source-bound evidence](../../evidence/c4.3-arrow-20261005/README.md); hosted successor and parent pin recorded separately. C4.4/C-04, public API, release and clinical acceptance remain open.

## C3 experimental private shadow runner — local review 2026-10-10

Local runtime source `6f2c4e305e8b885b67f534035e5a72ac5673a094` and Rust 1.99-only
CI gate `4e7d6075861d42a8f480ca0feb04414fa29d9bd1` qualify the bounded synthetic
shadow runner locally. [Evidence](../../evidence/c3-local-qualification-20261010/README.md)
records 351 full calibration passes, 26 release shadow passes, native fresh-process
recovery, actual IPC/Parquet roundtrip and independent manual readback. Hosted C3
qualification, C3.4/parent acceptance and parent pin remain pending. Historical
owner-track Done scopes are unchanged; no C5, public API, clinical or release gate closes.
