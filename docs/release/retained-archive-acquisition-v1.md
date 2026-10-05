# Retained archive acquisition v1

`packaging/scripts/acquire_package_archive_bundle.py` has two exclusive output modes. The legacy `--output DIR` mode preserves its existing extracted-bundle behavior and writes the compact sibling `DIR.acquisition.json` receipt. The new `--acquisition-output DIR` mode creates one fresh directory containing the original downloaded artifact ZIP and the records required to independently reconstruct the admission decision.

## Exact-run use

For a successful same-repository manual package run on `main`, pass only its exact run ID:

```sh
python3 packaging/scripts/acquire_package_archive_bundle.py \
  --run-id "$PRODUCER_RUN_ID" \
  --require-main-dispatch \
  --acquisition-output "$RUNNER_TEMP/archive-acquisition"
```

The destination must not exist. The strict mode derives the source commit from the selected workflow run; `--source-commit` and `--head-commit` are forbidden in this mode. The run must be complete and successful, belong to `edithatogo/kairos`, use `.github/workflows/package-dry-run.yml`, be a `workflow_dispatch` on `main`, and have no pull-request identity. Artifact selection uses that run's exact source-SHA name and validates the artifact ID, origin, repository IDs, digest, expiry state, and retained archive index. It never queries for or substitutes the latest run. The source must equal current main or be its exact compare merge base with `status=ahead`, positive `ahead_by`, and zero `behind_by`.

The historical/PR mode remains available through `--source-commit` and optional `--head-commit`. It preserves the existing run/merge/head identity checks. For a PR build, the independently read source commit must have the supplied PR head among its actual GitHub parent records. For main dispatch, the source and workflow head are the same commit; parent records are validated as real commit parents, without fabricating a self-parent.

## Retained directory

The new fresh output directory contains:

- `bundle/`: extracted and verified seven-ecosystem archive bundle.
- `{artifact_id}.zip`: byte-preserved original Actions artifact ZIP, downloaded once and checked against the API digest.
- `acquisition.json`: the compact original acquisition receipt, including the exact source SHA, run and artifact IDs, ZIP digest, archive-index digest, selection policy, and typed main-ancestry result where applicable.
- `receipt.json`: the outer acquisition record used by the archive-evidence profile, including producer source/head/tree, run and artifact IDs, ZIP digest, archive count, ecosystem set, exit status, and bounded claim scope.
- `artifact-metadata.json`: the raw selected artifact API object.
- `run-metadata.json`: the raw selected workflow-run API object.
- `branch-main-readback.json`: the raw `main` branch API response in strict mode.
- `compare-main-readback.json`: the raw compare API response when the source is behind current main; it is omitted when the source equals current main, and `main_ancestry.compare_url` is then null.
- `source-commit-readback.json`: a normalized source-commit record for the independent verifier, keyed by the exact source SHA.
- `source-commit-api-readback.json`: the raw GitHub Git Data API response from `/repos/{owner}/{repo}/git/commits/{sha}`, with top-level tree, parent and signature fields.

The source commit readback is acquired from the Git Data API endpoint `/repos/{owner}/{repo}/git/commits/{sha}`. It must bind the exact SHA and tree SHA, retain the native parent objects and `/git/commits/{parent}` URLs, and report a valid verified signature. The normalized verifier record is derived from this raw response without changing parent URLs. In main mode the source SHA equals the workflow head; in PR mode the distinct selected head must be an actual source parent. A missing, malformed, mismatched, unsigned, or invalid readback fails before success.

## Failure and security behavior

The destination is reserved exclusively before acquisition begins. On any later failure, the command removes only that newly created output directory; it never replaces or cleans a pre-existing destination. The retained ZIP is hashed while downloaded, matched to the artifact API digest, and used as the input to verified extraction. Extraction and bundle verification must both pass before the command emits success. The artifact ZIP is never downloaded a second time.

The helper does not serialize environment variables, access tokens, or command logs into the acquisition directory. A hosted consumer should expose its read-only `GH_TOKEN` only to this exact acquisition step. Downstream scanner/build jobs should receive staged files, not that token.

## Qualification boundary

This receipt proves exact-run selection, retained artifact identity, commit readback, and local bundle verification. It does not establish original compilation provenance, a signed producer attestation, SLSA level, registry readiness, or release acceptance. The consumer must independently validate the complete SBOM/provenance evidence tree before uploading a final evidence artifact.

The `receipt.json`/`source-commit-readback.json` contract is coordinated with the independent evidence verifier. For main dispatch, verifier lineage rules must treat `producer_pr_head == source_commit` as the accepted main identity and must not require a commit to be its own parent. PR lineage continues to require the distinct head among the source commit's parents.
