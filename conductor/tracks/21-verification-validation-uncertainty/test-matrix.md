# Test Matrix: Track 21 Verification, Validation & Uncertainty

## Required checks

| Check | What it proves | Alpha | Beta | 1.0 |
|---|---|---:|---:|---:|
| Docs page names the three terms | The page defines verification, validation, and uncertainty clearly. | yes | yes | yes |
| Evidence boundary is explicit | Only committed replayable artifacts count as evidence. | yes | yes | yes |
| Accepted artifacts are listed | Readers can see which artifacts support a claim. | yes | yes | yes |
| Replay/scenario fixture tie-in exists | The page links credibility claims to scenario and seed fixtures. | yes | yes | yes |
| Markdown link and lint check | The page renders and links cleanly. | yes | yes | yes |
| Artifact existence check | The referenced docs and track files exist. | yes | yes | yes |
| Red-team limit check | The page explains what the evidence does not prove. | yes | yes | yes |

## Local validation commands

```bash
test -f docs/trustworthy-simulation/verification-validation-uncertainty.md
test -f conductor/tracks/21-verification-validation-uncertainty/handoff.md
test -f conductor/tracks/21-verification-validation-uncertainty/test-matrix.md
test -f conductor/tracks/21-verification-validation-uncertainty/risk-register.md
rg -n "verification|validation|uncertainty|scenario|seed|replay|trace|evidence boundary" docs/trustworthy-simulation/verification-validation-uncertainty.md conductor/tracks/21-verification-validation-uncertainty
```
