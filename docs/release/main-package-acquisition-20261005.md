# Main package acquisition readback — 2026-10-05

## Result and scope

The default-main package dry run completed successfully. A separate local
acquisition and independent byte readback verified eight package archives across
seven ecosystems. This qualifies retained archive acquisition and consistency;
it does not establish scanner coverage, complete SBOM/provenance evidence,
registry publication, or release acceptance.

## Exact producer identity

- Repository: `edithatogo/kairos`.
- Source and workflow head: `5bdc1b42d2e3ad4722f8317e19be92aef268f815`.
- Source tree: `4613cada2aa7e66aedfd500ee4147cba541864a6`.
- Workflow: `.github/workflows/package-dry-run.yml`.
- Event and branch: `workflow_dispatch`, `main`.
- [Producer run 37318162611](https://github.com/edithatogo/kairos/actions/runs/37318162611): completed success; all eight jobs succeeded.
- Retained artifact ID: `11349051447`.
- Acquisition ZIP: 57,381 bytes; SHA-256
  `56c309e9264496c0a7ad6c8cfaa01ba301496a8f96d4ee3fa664bd52930fb8fe`.
- Archive index SHA-256:
  `bb16da02fe2697f9dfb06ded87eaadf4e8c44cb53563044d6830c0fbd1283ab3`.

Native GitHub commit readback returned one parent, source tree matching the
above, and signature verification `verified: true`, `reason: valid`. Main was
identical to the producer source at acquisition. Main identity does not require
or imply a self-parent; the source's actual parent is
`650afd15ec7f4aa8fac8d54f66cf4b16af12f377`.

## Executed checks

Working directory:
`/Volumes/PortableSSD/codex-worktrees/kairos-archive-main-candidate-20261005`,
clean source at the exact producer commit.

```sh
python3 packaging/scripts/acquire_package_archive_bundle.py \
  --run-id 37318162611 --require-main-dispatch \
  --output .artifacts/archive-main-candidate/bundle
```

Exit status: `0`, `verified exact-run package acquisition`.

Independent standard-library readback reconciled the acquisition receipt,
archive index and build receipt source identities; the exact run, artifact and
ZIP digest; index digest; all eight archive sizes and SHA-256 hashes; seven
ecosystem build receipts and successful builder exit statuses. Result: pass.

The merged archive verifier CLI was then executed with Python 3.14.8:

```sh
python packaging/scripts/build_package_archive_bundle.py \
  --verify-existing --output .artifacts/archive-main-candidate/bundle \
  --source-commit 5bdc1b42d2e3ad4722f8317e19be92aef268f815
```

Exit status: `0`, `verified 8 package archives`. The extracted archives,
compact acquisition receipt and independent readback record remain under
`.artifacts/archive-main-candidate/` in the acquisition worktree. The original
ZIP remains retained by the identified GitHub artifact; this legacy local
acquisition mode does not retain its downloaded ZIP or complete native API
readbacks. The separate retained acquisition mode addresses that limitation.

## Remaining evidence gates

The fresh-main archive consumer still requires retained raw acquisition records,
qualified main-dispatch lineage validation, independently constructed expectation
maps, native Linux scanner qualification and the complete archive-derived
SBOM/provenance verifier. Package build receipts describe their own scope: Go and
Julia are source archives, and this R dry run used `--no-tests` with one optional
package availability NOTE. This record does not turn those checks into broader
runtime or clinical validation.
