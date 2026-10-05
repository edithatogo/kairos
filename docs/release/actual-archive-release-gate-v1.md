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
