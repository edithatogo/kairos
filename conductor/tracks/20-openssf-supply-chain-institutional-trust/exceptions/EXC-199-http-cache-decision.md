# EXC-199: proposed bootstrap npm audit exception for PR #199

Status: **pending human decision; not approved or active**. This record does not change the package dry-run runner, classifier, workflow, or raw audit gate. EXC-193 remains limited to PR #193; its approval is not reused here. The Track 20 security exception policy requires the security and release owners to expressly accept both the exception and its classification. A reviewer, this proposal, or a green compensating check is not that approval.

## Decision requested

Decide whether to accept a temporary operational tooling limitation for one pinned bootstrap npm dependency tree, while preserving raw audit output and its failing exit. If approved, this would apply only to PR #199 development integration plus alpha and beta package dry runs, and only after a separately reviewed fail-closed activation change and exact-head hosted verification. Until then, the current runner rejects the unapproved PR source and raw audit remains a strict failing gate.

The binding scope is conjunctive: repository `edithatogo/kairos`, pull request 199, exact head ref `codex/kairos-track48-optimistic-runtime`, and one of `development_pr_199`, `alpha_package_dry_run`, or `beta_package_dry_run` must all match. An alpha/beta context cannot authorize another PR. RC, 1.0, publication, website dependencies, other dependency trees, and every other PR are excluded. EXC-193 is untouched and remains PR-193-only.

Proposed expiry is **00:00 Brisbane time on 10 October 2026**, or earlier when the release stage advances, dependency/patch/proof bytes change, the audit graph changes, or an official fixed release is adopted. There is no automatic renewal.

## Evidence and limits

The local raw audit was run at source commit `0a3b86aa33d1f158eaca2855e0e503cdac6e08ef`; it exited 1 and emitted 19 high graph nodes (one advisory plus derived metavulnerability nodes), with empty stderr. Its raw report SHA-256 is `e5f3325920245649ca0d2af6122dc9175c2d337972620a43883be34e59a37a2c`, and its vulnerability graph SHA-256 is `0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8`. The advisory is GHSA-ch52-4w7c-c8xp for `http-cache-semantics@4.2.0`, range `<=4.2.0`. This is a fresh audit capture, not a hosted run.

A separate local compensating-control capture at source commit `e3306f4ca3e560b81725a1b07a275d98643caefe` records passing patch tests, dependency resolution, both source/installed-copy behavior checks, and npm signature verification. Those controls do not make the raw audit pass and were not run in the same invocation or source revision as the raw audit. All five source fingerprints and the patch index hash are bound in the JSON record and are identical to the prior PR-193 evidence.

The hosted PR #199 runner attempt is a third, distinct result. Its receipt reports `raw_audit_status: not_executed` and `unapproved PR source`; therefore it provides no hosted audit result or exception activation evidence. The byte-preserved captures and hashes are listed in [the evidence manifest](evidence/EXC-199/manifest.json), with readable provenance in [the evidence README](evidence/EXC-199/README.md).

Node/npm versions were measured during this packet preparation at source head `be54595e3fad310f952389520632cf9af6900e92`: Node v26.10.0 and npm CLI 12.1.0. This is preparation-time metadata, not retroactive runtime evidence for the audit or control captures. The hosted workflow's configured runtime and actual results must be established from the hosted run if a future activation is considered.

## Conditions before activation

No exception is active from this proposal. Before any implementation of exception classification, obtain explicit security-owner and release-owner approval, including acceptance of the temporary operational classification, and record the human decision in the JSON. Then separately review a fail-closed activation patch and run exact-head hosted checks. Raw audit argv, JSON, stderr, exit, source/runtime, and hashes must remain retained; the classifier may classify only the exact bound finding graph and must not rewrite the raw result or report zero vulnerabilities. Any changed graph, source/lock/proof fingerprint, context, PR, scanner result, or compensating control must fail closed. All other repository security checks remain independently required.

If either owner rejects the proposal, leave the current strict gate in force. If a future activated control fails or the scope expires, restore strict raw-audit acceptance and keep the PR out of the affected stage. No publishing, release-policy relaxation, upstream notification, or scope extension is requested.

This packet is proposed evidence only. Human approval and implementation readiness remain separate decisions.
