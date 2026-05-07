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
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, cleaned commit/push, and blocker recording.