# EXC-199 evidence packet provenance

This packet supports a **pending proposal only**. It does not record approval, classify a hosted run as passing, or activate an exception. Evidence captures are deliberately separated by source revision and invocation.

| Capture | Source | Result | Files |
|---|---|---|---|
| Fresh local raw audit | `0a3b86aa33d1f158eaca2855e0e503cdac6e08ef` | Exact npm audit command exited 1; 19 high graph nodes; stderr empty. Raw JSON SHA-256 `e5f3325920245649ca0d2af6122dc9175c2d337972620a43883be34e59a37a2c`; graph SHA-256 `0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8`. | `audit/pr199-0a3b86a/` |
| Local compensating controls | `e3306f4ca3e560b81725a1b07a275d98643caefe` | Five recorded checks passed: patch tests, resolution, top-level behavior, installed npm-copy behavior, and signatures. | `controls/e3306f4/` |
| Hosted PR #199 attempt | Associated PR head `0a3b86aa33d1f158eaca2855e0e503cdac6e08ef`; runner receipt does not record a source commit | `raw_audit_status` is `not_executed`; runner error is `unapproved PR source`. No hosted audit result is claimed. | `hosted-pr199-0a3b86a/` |

The first two rows are separate local captures from different source commits. They are not a unified run and the controls do not change the raw audit exit. The third row is a blocked hosted attempt, not a passing exception-backed check. `manifest.json` records each file's source path, byte count, SHA-256, and byte-identity check.

## Preparation-time runtime metadata

On 2026-10-03 at `2026-10-03T12:42:07Z`, preparation-time metadata was measured in `/Users/doughnut/Documents/careops-sim/.worktrees/kairos-implementation-programme` at source head `be54595e3fad310f952389520632cf9af6900e92`: Node v26.10.0 and npm CLI 12.1.0. The measurement file SHA-256 is `2df41a4349f4dc63e0a2bc0ac62ee896a2b3d27f3407f1442caf2f3765127e17`. It records executable paths and hashes. This measurement was made while preparing the packet and is not retroactive metadata for the audit or control runs. The hosted attempt did not reach the raw audit.

No owner approvals are recorded. The matching EXC-199 JSON intentionally remains pending, with null approval identities/timestamp/evidence and `classification_accepted: false`.
