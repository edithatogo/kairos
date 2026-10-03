# EXC-193: proposed local cache mitigation audit exception

Status: **approved by the human sole maintainer in this chat on 3 October 2026**, acting in security-owner and release-owner roles. The operational classification is approved for this record only. Activation implementation and hosted checks must pass before merge. Raw npm audit findings remain failures in the retained scanner evidence.

## Decision requested

Security owner and release owner must explicitly approve treating a version-based scanner's inability to represent a verified local source mitigation as a temporary operational tooling limitation under the Track 20 exception process. This classification extends the examples in that process; it is not assumed to be accepted. If those owners reject this classification, use the release-stage exception process, including a maintainer outside the affected track, or wait for upstream. Record actual names/handles, decision date and human approval evidence in the JSON record. An AI review is not approval.

Scope matching is conjunctive: every allowed context must belong to PR #193; alpha/beta labels never authorize another PR.

Scope: PR #193 development integration and alpha/beta package dry runs of the pinned bootstrap npm tree only. RC, 1.0, publication, website dependencies and other package trees are excluded. The exception expires at **00:00 Brisbane time, 10 October 2026**, or when the release stage advances, the dependency/patch/proof bytes change, the audit findings change, or an official replacement is adopted—whichever is first. No automatic renewal.

## Control and impact

The failing control is the moderate-threshold bootstrap npm audit in Package Dry Runs. Raw audit exits 1 for GHSA-ch52-4w7c-c8xp and 18 derived metavulnerability nodes, not 19 independent advisories. The unmodified package identity remains http-cache-semantics@4.2.0, so the advisory matches it even after source remediation. Risk acceptance would permit development integration under verified source mitigation; it does not declare the published package or the entire repository vulnerability-free.

The immutable JSON record binds the original lock, patcher, runtime consumer validator, installer tests, behavioral tests and exact raw finding graph. The raw baseline is checked in separately. Changing package identity, inventing a version, deleting findings or suppressing the scanner is excluded.

## Compensating controls

- Regenerate the exact integrity-pinned npm bundle and perform a clean ignore-scripts install.
- Apply only the reviewed upstream source delta: commit 14a8c2ad51740dc39bf3e8f1a11c845a5003f217; patched index SHA-256 fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76. Preserve BSD-2-Clause copyright/terms.
- Reject unexpected package identity, source bytes and dependency copies. Check npm and make-fetch-happen resolve expected installed paths and the exact patched source.
- Execute seven installer integrity cases and 60 cache behavior/security cases per installed copy; original released source is a demonstrated failing negative control.
- Continue registry signature verification and every other CI/security gate independently.

Hosted mitigation evidence: [run 37094455626](https://github.com/edithatogo/kairos/actions/runs/37094455626), source b68a5c780fe6dc58713fb3adaa6a85d5e6748a62, Linux / Node 22.22.2. The mitigation and both behavior suites pass; the raw audit remains a failure.

## Activation design for review

Activation is a separate reviewed change after named approvals. It must preserve the raw audit argv, stdout JSON, stderr, exit status, SHA-256s, tool/runtime version, source commit and working directory as an always-retained CI artifact. A policy classifier then reports one of `clean`, `approved_temporary_exception`, or `failed`. It never rewrites the raw exit status or reports zero vulnerabilities for an exception.

The classifier must reject a missing/pending/expired approval, wrong PR/context, publication/RC/1.0, scanner/network/authentication error, malformed JSON, unsupported audit schema, source/lock/test fingerprint drift, any additional or unpatched dependency copy, and any changed finding graph. The permitted graph must match the exact reviewed fingerprint, advisory source ID, URL, package, range and severity; each metavulnerability must trace exclusively to that advisory. Counts alone are insufficient. Any other finding at any severity must fail for review rather than being silently covered.

Positive test cases: exact approved baseline, valid clean audit, patched dependency tree and valid alpha/beta context. Negative cases: pending/expired/unnamed approval; invalid stage/PR; publication; extra advisory at every severity; mixed direct advisories; changed node, range or severity; extra/missing copy; raw scan error or malformed JSON; source, lock, proof or patch drift; behavior or signature failure. Test that failed classification exits nonzero and raw artifacts survive that failure.

The approved classification permits the bounded classifier/workflow activation change. Implementation tests and exact-head hosted checks remain required before exception-backed merge. Approval is not itself evidence that those checks pass.

## Follow-up and rollback

This decision record is the follow-up ADR for the temporary exception. Before expiry, obtain and verify the official fixed package version and advisory range; review its source/integrity, refresh the appropriate locks/preparer, run clean install, resolution, behavior and signature checks, and require the ordinary raw audit to pass. Remove the exception when that is verified. If any compensating control fails, restore strict raw audit acceptance immediately and keep PR #193 unmerged. No package publishing, release-policy relaxation or automatic upstream messaging is included.

## Preparation validation and limits

During preparation, the then-pending approval fields and all five source fingerprints, raw audit hash/finding fingerprint and exclusive advisory paths for all 19 graph nodes were independently checked against the current checkout and fresh audit. Human approval has subsequently been recorded above; implementation and hosted verification remain separate gates. The existing Track 20 trust validator exits 1 because it requires literal config:recommended in renovate.json; the unchanged repository uses github>edithatogo/renovate-config. Both files predate this proposal and have no diff. This broader validator mismatch remains a separate owner follow-up; it is not waived by EXC-193.
