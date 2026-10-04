# Track49 source-integrity false positive — 4 October 2026

PR208 Secret Scan at head72850e96d787c20c7031768ff1b8fc725b0804c2 failed on one generic-api-key finding in historical preparation commitffb788b9107ae2c9a6b0bdc2d767faebdde1ac80, packet line15. Hosted scanner8.24.3 run37199827298 remains the original failed evidence.

The value is SHA256 `f8c4e76b0c3c78b660ecefa814a627c7938d7562c21a70d4802ebdf043131456` of the tracked native `owned-handler-retirement-api.md`, immutable source blob `2f6ed3bb06811bb2a1d8f51e4b298a51dd816e64`. Independent security role `track48_external_interface_prepare` and coordinator each recomputed those exact Git bytes. It is public source-integrity metadata, not an authentication credential.

The qualified security role approved this one exact historical fingerprint under a separate exclusive coordinator claim. This records the narrow Track20/security handoff; it grants no broader policy/release exception. `.gitleaksignore` contains only this fingerprint and explanatory comments. Default rules, full historical scan, workflow permissions and exit codes remain unchanged. There is no regex/path-wide/rule-wide suppression, no packet mutation or public-history rewrite; future commits/findings do not match this fingerprint. See [pinned scanner documentation](https://github.com/gitleaks/gitleaks/blob/v8.24.3/README.md#gitleaksignore).

## Executed local checks

Working directory `/Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004`, source head72850e96d787c20c7031768ff1b8fc725b0804c2, installed Gitleaks8.30.1 (different from hosted8.24.3). Full command prefix `gitleaks git --redact --no-banner --no-color --log-opts='--no-merges --first-parent ffb788b9107ae2c9a6b0bdc2d767faebdde1ac80^..72850e96d787c20c7031768ff1b8fc725b0804c2' --report-format=json`:

- Before adding fingerprint, `--report-path=artifacts/track49-secret-scan/before.json`: exit1, exactly the hosted fingerprint.
- After adding fingerprint, `--report-path=artifacts/track49-secret-scan/after.json`: exit0, no findings; same two historical commits and73007 scanned bytes.
- `gitleaks dir artifacts/track49-secret-scan/negative-control --gitleaks-ignore-path=.gitleaksignore --redact --no-banner --no-color --report-format=json --report-path=artifacts/track49-secret-scan/negative.json`: exit1, one unrelated deliberately synthetic noncredential generic-api-key fixture. The narrow fingerprint does not suppress that control.

Hosted pinned8.24.3 scan at the new delivery head remains pending at this record's writing; local8.30.1 results are not relabeled as hosted evidence. Full contract/dependency/storage/runtime holds remain unchanged; source/pin/global status and parallel queue/calibration claims are untouched.

Local report hashes:
- `artifacts/track49-secret-scan/before.json` SHA256 `45f2f6d3d95ccac60f2c1327857ad6b3e8d064fa513a23d1cf952da1e3fc7af6`
- `artifacts/track49-secret-scan/after.json` SHA256 `37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570`
- `artifacts/track49-secret-scan/negative.json` SHA256 `54e3f9be7e45294cf2dd3f1b9aca89fa3c3e53a477761d0edc9c8f5aba5f66ce`
