# Track48 future-PR security gate analysis

**Read-only snapshot:** 3 October 2026. Local source checkout `7a432ab46c3bfa5f3917f1cce0f4545fac39f649`; live GitHub `main` read through API at `8ed3c9acef60b107da243ae640a4d62f2075b0c5` (commit timestamp 2026-10-03T10:32:33Z). No source, lockfile, alert, exception, or website changes were made.

## Finding

A cache-only, PR-specific audit exception does not clear the live Scorecard finding and is not a CodeQL exception. The current open [Code Scanning alert #482](https://github.com/edithatogo/kairos/security/code-scanning/482) is Scorecard `VulnerabilitiesID`, **High / error**, on `refs/heads/main` at `8ed3c9a`. Its message names both `GHSA-ch52-4w7c-c8xp` and `GHSA-vfj7-8cjw-p6xm`.

The pinned `code-scanning-gate` action, called at the end of both `.github/workflows/scorecard.yml` and `.github/workflows/codeql.yml`, fetches open code-scanning alerts and fails on high/critical alerts whose `most_recent_instance.commit_sha` equals the workflow's `GITHUB_SHA`. Thus a Scorecard run for current `main` is blocked by #482. The CodeQL workflow also invokes that gate, but the existing alert is attached to the current main SHA, while a future PR run has its own analyzed SHA; the action's exact-SHA filter means I cannot infer from #482 alone that a future PR's CodeQL job will fail. Scorecard itself is configured for main pushes, schedule, and `branch_protection_rule`, not `pull_request`. Actual future PR and post-merge hosted results decide those checks. This distinction avoids claiming a branch-wide failure from a main-only alert.

The npm exception runner is narrower still: `run_npm_audit_gate.py` applies only `EXC-193-http-cache.json`, runs npm audit with `--prefix scripts/bootstrap-node-tools`, verifies bootstrap lock/patch/test fingerprints, and classifies the preserved raw audit result. The policy hard-codes PR 193 and the `development_pr_193`, alpha, and beta contexts; excludes `website_dependency_tree`, other package trees, RC/1.0, and publication. It cannot change Scorecard SARIF or satisfy the separate code-scanning gate. EXC-193 remains approved only for PR #193 and the pinned bootstrap tree; its expiry is 00:00 Brisbane, 10 October 2026 or earlier on stated trigger. The record says the raw audit remains failing and is not a vulnerability-free claim.

## Locked vulnerable package paths at the local source snapshot

- `scripts/bootstrap-node-tools/package-lock.json`: `npm@12.1.0 → make-fetch-happen@16.0.1 → http-cache-semantics@4.2.0`.
- `website/package-lock.json`: direct `astro@7.3.5 → http-cache-semantics@4.2.0`; and `starlight-llms-txt@0.12.0 → micromatch@4.0.8 → braces@3.0.3`.
- `bindings/typescript/package-lock.json` and `tools/docs-quality-node-tools/package-lock.json`: neither package appears in the checked lock snapshots.

The official GitHub advisories reviewed today say no fixed package version is listed for either advisory: http-cache-semantics versions `<=4.2.0` are affected by cross-user cached-response disclosure ([GHSA-ch52-4w7c-c8xp](https://github.com/advisories/GHSA-ch52-4w7c-c8xp)); braces `<=3.0.3` is affected by nested-pattern stack exhaustion ([GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm)). Alert #482's current message ties both findings to main, but its summary does not give package paths; the lockfile ancestry above is local source evidence, not proof of the exact package graph fetched by Scorecard at that remote SHA.

## Governed disposition

1. Prefer source remediation. For cache, adopt an official fixed release when available, refresh each relevant lock, then prove install/integrity/resolution, relevant behavior, and ordinary audit clean. The local EXC-193 source patch is evidence only for its explicitly fingerprinted bootstrap copies; it does not mutate website lock state or dismiss Scorecard's advisory finding.
2. For braces, update to an official fix or remove the dependency path and regenerate/verify the website lock. A separate active website-continuation packet reportedly proposes removing `braces`, `micromatch`, and related lock nodes; treat that only as a candidate. It is not merged-source proof and does not clear #482 until a fresh default-branch scan confirms the resulting tree.
3. Do not dismiss the alert or add an OSV ignore as a shortcut. Track20 labels exceptions release decisions, not missing evidence. Its temporary operational exception requires Security owner + Release owner and may unblock alpha/beta only when recorded as allowed-failure. A release-stage exception additionally needs one maintainer outside the affected track and blocks RC/1.0 until approval; a permanent waiver requires maintainer decision + ADR and blocks beta+ until accepted. Each record must name the control, stage, impact/reason, compensating control, approvers, expiry, and follow-up.

If fresh hosted Track48 results show only the bootstrap cache advisory still fails, the smallest possible approval after those exact results is a new, Track48-PR-specific temporary operational exception with Security and Release owner approval, exact tree/advisory fingerprints, evidence, compensating controls, and expiry; EXC-193 cannot be reused because it is conjunctively bound to PR #193. If braces or another high/critical finding remains, a cache-only exception is insufficient. Whether a separate exception could satisfy the code-scanning gate is unproven: the gate currently has no exception input and fails on matching open high/critical alerts, so preserve the blocker pending source remediation or a separately governed control change plus explicit approval. Do not request that broader approval before the actual hosted findings are known.

## Evidence boundaries and provenance

- `gh api repos/edithatogo/kairos/commits/main`: SHA and timestamp above.
- `gh api repos/edithatogo/kairos/code-scanning/alerts/482`: open Scorecard alert, ref/SHA, rule severity, and both advisory URLs. The Dependabot-alert endpoint returned 404; #482 is a code-scanning alert, not evidence of a Dependabot alert.
- Inspected local `.github/workflows/package-dry-run.yml`, `.github/workflows/scorecard.yml`, `.github/workflows/codeql.yml`, `run_npm_audit_gate.py`, `npm_audit_policy.py`, EXC-193 JSON/decision and lockfiles at local HEAD above. The referenced gate action source is pinned at `edithatogo/.github@c3e51f894a500198e67c864a1f0c460ba72e12cd`; its implementation filters by exact current commit SHA and high/critical severity.
- This is a read-only static/API assessment. It is not a new Track48 hosted run, a PR result, or evidence that either lockfile still matches remote `main` after the local snapshot.
