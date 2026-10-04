# EXC-199: corrected-mitigation scope amendment proposed

Status: human source-binding amendment approved; reviewed integration and hosted acceptance remain pending. The human approved the EXC199 temporary operational classification and Track49 conditional scheduling on 4 October2026. That authority is recorded and is not being requested again. The changed mitigation fingerprints below are not covered by the old source-bound approval.

The old mitigation allows restricted responses through stale-if-error/stale-while-revalidate fallback paths, including request-mismatch revalidation. Independent expanded checks now fail129 of248 cases against its fc7b3f source. Actual EXC199 activation must remain blocked_stale_fallback_gap; do not use the old60-case pass as adequate mitigation proof.

## Concrete amendment

Bind EXC199 to corrected source2efedc5ea04c1caf26a200bd2450e9e38426e8fe and patched index5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c. The JSON records exact replacements for the patcher, npm consumer validator, installer tests and behavior tests; lockfile fingerprint is unchanged. Independent current readback passes248 behavior cases on both bootstrap copies and a separate website diagnostic, nine installer tests, consumer resolution and registry signatures. The website remains outside this exception. The fresh raw audit still exits1 with19 high nodes and the same reviewed graph0b3e5f; no findings are suppressed or reported clean.

Scope and expiry remain PR199 development integration and its alpha/beta bootstrap package dry runs only, repository edithatogo/kairos and exact branch codex/kairos-track48-optimistic-runtime, expiring00:00 Brisbane10October2026 or earlier on scope/stage/proof drift. RC,1.0, publication, website and every other PR/tree remain excluded. Approving these stronger proof bytes would amend only this source binding, not extend expiry or waive any other gate.

Reviewable details: [amendment JSON](EXC-199-mitigation-amendment.json), [raw evidence hashes](evidence/EXC-199-amendment/manifest.json), and [executed commands](evidence/EXC-199-amendment/receipt.json). Evidence captures are local and separate from hosted acceptance. New source must be integrated with the parallel cache owner's commits and the PR-bound selector under review; do not overwrite that owner's runner changes. Run exact-head hosted gates before merge. The current runner/proof authority must stay strict while this amendment is pending.

## Why a new decision is necessary

The approved EXC199 decision explicitly expires when dependency/patch/proof bytes change. These are substantive safety changes, not merely new labels for old evidence. Security-owner and release-owner human approval of the changed exact bindings is required by Track20 policy; independent review verifies the candidate but cannot grant that approval.

## Human decision — 4 October 2026

The human explicitly replied “I approve” to the source-binding amendment question in this chat. The security/release-owner decision is recorded in the JSON at 2026-10-04T10:39:27.598427+10:00. Exact corrected fingerprints, PR199-only development/alpha/beta bootstrap scope and expiry remain unchanged. This authorizes reviewed integration; it does not claim active classification, hosted success, merge or Track49 dispatch. The strict migration being prepared in a parallel chat is independent work and is not included in this approval.
