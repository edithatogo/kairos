# Actual archive release gate adapter v1

`packaging/scripts/prepare_verified_archive_release.py` is a local, fail-closed
adapter for converting retained package-archive evidence into actual archive
release subjects. It does not acquire artifacts, build packages, run scanners,
generate SBOM or provenance, publish packages, or accept a release. The release
workflow and the Track 15 validator are not wired to this adapter by this change.

## Required inputs

Every argument is required. There are no inferred paths or default trust pins.

```text
python packaging/scripts/prepare_verified_archive_release.py \
  --evidence-dir <verified-evidence-tree> \
  --archive-bundle <acquisition-root>/bundle \
  --archive-zip <acquisition-root>/<artifact-id>.zip \
  --acquisition-dir <acquisition-root> \
  --expected-inputs <independent-five-field-map.json> \
  --expected-binding <independent-acquisition-binding.json> \
  --expected-verifier-sha256 <lowercase-sha256> \
  --spdx-schema <pinned-spdx-2.3-schema.json> \
  --release-source-commit <full-lowercase-git-sha> \
  --output <new-output-directory>
```

The expected map, binding, and schema must be regular files outside the evidence
and acquisition trees. The verifier source and all five helper sources are
hashed against the independent expected map before execution. Those exact
captured bytes, plus captured copies of both expected maps and the schema, are
staged before the verifier subprocess starts; this prevents the executable,
helper, or trust inputs from changing between the pin check and verifier read.
The argv-only subprocess uses the already-pinned acquisition helper's process
group runner. It has a bounded deadline, bounded stdout, discarded stderr, and
bounded terminate/escalate/reap cleanup for the whole process group. Its result
must match the qualified archive-copy evidence profile and declared claim scope. The adapter
checks the binding, expected map, and retained archive index against the
explicit release source SHA. It requires the bundle to be the acquisition
root's `bundle/` child. Each index row must have the producer's six fields,
including ecosystem builder metadata. Those metadata values are checked against
the seven successful ecosystem records in `BUILD-RECEIPT.json` before the
manifest builder runs.

Input paths and ancestors must not be symlinks. The output must be a new
directory whose parent already exists; it may not overlap any input. The
archive-manifest builder runs with bounded time into a private staging
directory on the output filesystem. After post-build checks, the adapter
reserves the output path with exclusive directory creation, then moves the
validated children into that owned directory. A destination created by a
competitor during the run is rejected without replacing its contents. A failed
verifier or builder leaves no requested output; an existing output is never
replaced. The builder uses the captured, pinned
builder and bundle-helper bytes.

## Output checks

The adapter calls `build_archive_release_manifest.py` only after evidence
verification. It then checks the exact manifest stage, source commit and archive
index digest; every expected archive subject and its ecosystem/kind metadata;
the complete output file and directory inventory; the exact `SHA256SUMS` and
`RELEASE.txt` contents; and the size and SHA-256 of every copied archive against
both the retained bundle and manifest. Extra, missing, changed, or symlinked
output entries fail.

The output records archive subjects from actual archive bytes. The input
evidence verifier separately checks the retained ZIP, acquisition readbacks,
SBOM graph and provenance under its qualified profile. This adapter's success
does not mean that release workflow gates, hosted CI, artifact upload, release
approval, or publication have passed. Any later release workflow integration
must keep the existing dry-run/publication gates and make this adapter a
required step before evidence validation and upload.

## Integrated actual qualification

Consumer `419ac4291c763f6429a37e908d542294afc257d6` qualified the integrated
input builder and release-subject adapter against producer run
[37318162611](https://github.com/edithatogo/kairos/actions/runs/37318162611),
artifact `11349051447`, source `5bdc1b42d2e3ad4722f8317e19be92aef268f815`.
A fresh native Darwin Syft 1.54.0 installation at the reviewed installer source
passed independent receipt validation. Installer source SHA-256:
`6ac474f917222fc0ab42ad4d6c09092f4ab4a5395bbfaee526a1dbbd89b1944a`;
receipt SHA-256:
`fc8637abef8262e281219c0a24b984b22b5a244d21b2a5c9e32a7b9a02072a39`.

On Python 3.14.8, fresh independent input preparation, eight actual package
scans, full evidence verification and the actual release-subject adapter each
completed with exit zero. The verifier accepted eight archives, seven ecosystems,
nine SPDX documents and 44 evidence files. All eight prepared release archive
copies were independently reconciled byte for byte with the verified evidence.
An explicit different release source (`84523e8673b236474c2a43939fb776d669be2d19`)
was rejected with `binding_source_mismatch` and exit one before output creation.

Command argv, working directory, consumer, exit statuses, log hashes and output
readback are retained in ignored `.artifacts/archive-native-actual-qualification/`
in the qualification worktree. The prior adapter row-schema rejection remains
in `.artifacts/archive-release-gate-reviewed/`; it is not a passing result.
This local qualification does not establish a fresh hosted Linux receipt at
the updated installer source, hosted archive consumer execution, release workflow
integration, signed original-build provenance, release acceptance or publication.


### Cleanup and hosted-consumer candidate qualification

A fresh local execution at consumer
`30d3c38e9740e316c1cf6046cbce0ee8aae9d796` used the same exact retained producer
and fresh reviewed installer receipt described above. Every command treated
Python syntax warnings as errors. Input preparation, eight actual package scans,
full verification and the release adapter each exited zero; the verifier again
accepted eight archives, seven ecosystems, nine SPDX documents and 44 files.
All five helper pins were reconciled with that consumer's trusted Git blobs,
and all eight prepared archive copies matched the scanned evidence bytes.
The wrong-source case again failed with `binding_source_mismatch` and no output.

Commands and logs remain in ignored `.artifacts/archive-hosted-candidate-actual/`.
The source includes the [mainline consumer workflow](archive-main-consumer-v1.md),
whose hosted execution remains a separate gate. A subsequent integrated scanner
regression run exposed a native macOS process-group permission race in the
scanner's separate cleanup implementation; its retained failure is not converted
into a pass by this actual qualification. That repair and its focused qualification
must be completed before candidate delivery.
